use smithay::{
    backend::{
        allocator::{
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
            Fourcc,
        },
        drm::{DrmDevice, DrmDeviceFd},
        egl::{EGLContext, EGLDisplay},
        renderer::{
            damage::OutputDamageTracker,
            element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        session::{libseat::LibSeatSession, Session},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{
            timer::{TimeoutAction, Timer},
            EventLoop,
        },
        drm::control::{connector, Device as ControlDevice},
        rustix::fs::OFlags,
        wayland_server::Display,
    },
    utils::Transform,
    wayland::socket::ListeningSocketSource,
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tracing::{info, warn};

use crate::state::{ClientState, FrutigerState};
use frutiger_config::FrutigerConfig;
use frutiger_shell::FrutigerShell;

pub fn run_drm(config: FrutigerConfig) -> anyhow::Result<()> {
    info!("Starting Frutiger Rust on Native DRM/KMS backend with Frutiger Aero Shell...");

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
    let (mut drm_device, _drm_notifier) = DrmDevice::new(drm_fd.clone(), true)?;

    let gbm_device = GbmDevice::new(drm_fd.clone())?;
    let _gbm_allocator = GbmAllocator::new(gbm_device.clone(), GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT);
    let egl_display = unsafe { EGLDisplay::new(gbm_device.clone())? };
    let egl_context = EGLContext::new(&egl_display)?;
    let mut _renderer = unsafe { GlesRenderer::new(egl_context)? };

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

    let (width, height) = (mode.size().0 as i32, mode.size().1 as i32);
    info!("Selected CRTC: {:?}, Mode: {}x{}@{}Hz", crtc_handle, width, height, mode.vrefresh());

    let _drm_surface = drm_device.create_surface(crtc_handle, mode, &[conn])?;

    // 4. Output & Damage Tracking
    let output = Output::new(
        "eDP-1".to_string(),
        PhysicalProperties {
            size: (width, height).into(),
            subpixel: Subpixel::Unknown,
            make: "Frutiger".into(),
            model: "Laptop Display".into(),
        },
    );

    let output_mode = Mode {
        size: (width, height).into(),
        refresh: (mode.vrefresh() * 1000) as i32,
    };
    output.change_current_state(Some(output_mode), Some(Transform::Normal), None, Some((0, 0).into()));
    output.set_preferred(output_mode);

    let mut _damage_tracker = OutputDamageTracker::from_output(&output);

    // 5. Initialize State & Wayland Socket
    let mut state = FrutigerState::new(&display, event_loop.get_signal(), config);
    state.space.map_output(&output, (0, 0));

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

    // 6. Frame Render Loop
    let timer = Timer::from_duration(Duration::from_millis(16));
    event_loop.handle().insert_source(timer, move |_deadline, _, _state| {
        // Update live clock
        let now = std::time::SystemTime::now();
        if let Ok(duration) = now.duration_since(std::time::UNIX_EPOCH) {
            let secs = duration.as_secs();
            let hours = (secs / 3600 + 3) % 24; // MSK UTC+3
            let mins = (secs % 3600) / 60;
            shell.update_time(&format!("{:02}:{:02}", hours, mins));
        }

        TimeoutAction::ToDuration(Duration::from_millis(16))
    }).map_err(|e| anyhow::anyhow!("Timer insert error: {:?}", e))?;

    while state.is_running {
        event_loop.dispatch(Some(Duration::from_millis(16)), &mut state)?;
    }

    Ok(())
}
