use smithay::{
    backend::{
        allocator::Fourcc,
        drm::{DrmDevice, DrmDeviceFd},
        session::{libseat::LibSeatSession, Session},
    },
    reexports::{
        calloop::{
            timer::{TimeoutAction, Timer},
            EventLoop,
        },
        drm::control::{
            connector,
            Device as ControlDevice,
        },
        rustix::fs::OFlags,
        wayland_server::Display,
    },
    wayland::socket::ListeningSocketSource,
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tracing::{info, warn};

use crate::state::{ClientState, FrutigerState};
use frutiger_config::FrutigerConfig;
use frutiger_render::generate_frutiger_aero_gradient;
use frutiger_shell::FrutigerShell;

pub fn run_drm(config: FrutigerConfig) -> anyhow::Result<()> {
    info!("Starting Frutiger Rust on Native DRM/KMS backend...");

    let mut event_loop: EventLoop<FrutigerState> = EventLoop::try_new()?;
    let display: Display<FrutigerState> = Display::new()?;

    // Initialize Slint Shell
    let shell = FrutigerShell::new()?;
    info!("Frutiger Aero Shell initialized (Panel + Launcher)");

    // 1. Session initialization (LibSeat / seatd)
    let (mut session, _session_notifier) = match LibSeatSession::new() {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to initialize libseat session: {:?}. Please ensure seatd or logind is active.", e);
            return Err(anyhow::anyhow!("Session initialization error: {:?}", e));
        }
    };

    info!("Session initialized successfully. Seat: {}", session.seat());

    // 2. Open Primary DRM Device (/dev/dri/card1 or card0)
    let card_path = if std::path::Path::new("/dev/dri/card1").exists() {
        PathBuf::from("/dev/dri/card1")
    } else {
        PathBuf::from("/dev/dri/card0")
    };

    info!("Opening DRM card device at {:?}", card_path);
    let card_fd = session.open(
        &card_path,
        OFlags::RDWR | OFlags::CLOEXEC,
    ).map_err(|e| anyhow::anyhow!("Failed to open DRM device: {:?}", e))?;

    let drm_fd = DrmDeviceFd::new(card_fd.into());
    let (drm_device, _drm_notifier) = DrmDevice::new(drm_fd.clone(), true)?;

    // 3. Query Connectors & CRTCs
    let res_handles = drm_device.resource_handles()?;
    let mut chosen_connector = None;
    let mut chosen_mode = None;

    for conn_handle in res_handles.connectors() {
        let conn_info = drm_device.get_connector(*conn_handle, false)?;
        if conn_info.state() == connector::State::Connected {
            info!("Found connected DRM connector: {:?}", conn_handle);
            if let Some(mode) = conn_info.modes().first() {
                chosen_connector = Some(*conn_handle);
                chosen_mode = Some(*mode);
                break;
            }
        }
    }

    let (conn, mode) = match (chosen_connector, chosen_mode) {
        (Some(c), Some(m)) => (c, m),
        _ => return Err(anyhow::anyhow!("No active connected DRM display found")),
    };

    let crtc_handle = res_handles.crtcs().first().cloned()
        .ok_or_else(|| anyhow::anyhow!("No CRTC found"))?;

    let (width, height) = (mode.size().0 as u32, mode.size().1 as u32);
    info!("Selected CRTC: {:?}, Display resolution: {}x{}@{}Hz", crtc_handle, width, height, mode.vrefresh());

    // 4. Allocate DRM Native Scanout Buffer & Fill with Frutiger Aero Wallpaper
    info!("Allocating scanout buffer on AMD GPU...");
    let mut dumb_buffer = drm_device.create_dumb_buffer(
        (width, height),
        Fourcc::Argb8888,
        32,
    ).map_err(|e| anyhow::anyhow!("Failed to create Dumb scanout buffer: {:?}", e))?;

    // Fill buffer with Frutiger Aqua gradient pixels
    let gradient_pixels = generate_frutiger_aero_gradient(width, height);
    if let Ok(mut mapping) = drm_device.map_dumb_buffer(&mut dumb_buffer) {
        let slice = mapping.as_mut();
        let len = slice.len().min(gradient_pixels.len());
        slice[..len].copy_from_slice(&gradient_pixels[..len]);
        info!("Copied {} bytes of Frutiger Aero gradient to scanout buffer", len);
    }

    // 5. Register Framebuffer with DRM driver and light up the screen!
    let fb_handle = drm_device.add_framebuffer(
        &dumb_buffer,
        32,
        32,
    ).map_err(|e| anyhow::anyhow!("Failed to add DRM framebuffer: {:?}", e))?;

    info!("DRM Framebuffer registered (Handle: {:?}). Activating CRTC scanout...", fb_handle);
    drm_device.set_crtc(
        crtc_handle,
        Some(fb_handle),
        (0, 0),
        &[conn],
        Some(mode),
    ).map_err(|e| anyhow::anyhow!("Failed to set CRTC scanout: {:?}", e))?;

    info!("✨✨ Physical screen successfully lit with Frutiger Aero background! ✨✨");

    // 6. Initialize State & Wayland Socket
    let mut state = FrutigerState::new(&display, event_loop.get_signal(), config);

    let socket = ListeningSocketSource::new_auto()?;
    let socket_name = socket.socket_name().to_string_lossy().into_owned();
    let mut display_handle = display.handle();

    event_loop.handle().insert_source(socket, move |client_stream, _, _state| {
        let client_state = Arc::new(ClientState {
            compositor_state: Default::default(),
        });
        if let Err(err) = display_handle.insert_client(client_stream, client_state) {
            warn!("Error adding wayland client: {:?}", err);
        }
    }).map_err(|e| anyhow::anyhow!("Socket insert error: {:?}", e))?;

    info!("Wayland socket available: {:?}", socket_name);
    std::env::set_var("WAYLAND_DISPLAY", &socket_name);

    info!("=== Frutiger Rust Native Compositor Running ===");

    // Periodic animation / clock timer
    let timer = Timer::from_duration(Duration::from_millis(500));
    event_loop.handle().insert_source(timer, move |_deadline, _, _state| {
        let now = std::time::SystemTime::now();
        if let Ok(duration) = now.duration_since(std::time::UNIX_EPOCH) {
            let secs = duration.as_secs();
            let hours = (secs / 3600 + 3) % 24; // MSK UTC+3
            let mins = (secs % 3600) / 60;
            shell.update_time(&format!("{:02}:{:02}", hours, mins));
        }

        TimeoutAction::ToDuration(Duration::from_millis(500))
    }).map_err(|e| anyhow::anyhow!("Timer insert error: {:?}", e))?;

    while state.is_running {
        event_loop.dispatch(Some(Duration::from_millis(16)), &mut state)?;
    }

    Ok(())
}
