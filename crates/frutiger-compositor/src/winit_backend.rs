use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker,
            element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        winit::{self, WinitEvent},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::EventLoop,
        wayland_server::Display,
    },
    utils::{Rectangle, Transform},
    wayland::socket::ListeningSocketSource,
};
use std::sync::Arc;
use tracing::info;

use crate::state::{ClientState, FrutigerState};
use frutiger_config::FrutigerConfig;
use frutiger_shell::FrutigerShell;

pub fn run_winit(config: FrutigerConfig) -> anyhow::Result<()> {
    info!("Starting Frutiger Rust in Nested (Winit) mode with Frutiger Aero Shell...");

    let mut event_loop: EventLoop<FrutigerState> = EventLoop::try_new()?;
    let display: Display<FrutigerState> = Display::new()?;

    // Initialize Slint Shell
    let shell = FrutigerShell::new()?;
    info!("Frutiger Aero Shell initialized (Panel + Launcher)");

    let (mut backend, mut winit_event_loop) = winit::init::<GlesRenderer>()
        .map_err(|e| anyhow::anyhow!("Failed to init winit backend: {:?}", e))?;

    let mode = Mode {
        size: backend.window_size(),
        refresh: 60_000,
    };

    let output = Output::new(
        "nested-1".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Frutiger".into(),
            model: "Nested Window".into(),
        },
    );

    let _global = output.create_global::<FrutigerState>(&display.handle());
    output.change_current_state(Some(mode), Some(Transform::Flipped180), None, Some((0, 0).into()));
    output.set_preferred(mode);

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    let mut state = FrutigerState::new(&display, event_loop.get_signal(), config);
    state.space.map_output(&output, (0, 0));

    // Listening socket source for Wayland clients
    let socket = ListeningSocketSource::new_auto()?;
    let socket_name = socket.socket_name().to_string_lossy().into_owned();
    let mut display_handle = display.handle();

    event_loop.handle().insert_source(socket, move |client_stream, _, _state| {
        let client_state = Arc::new(ClientState {
            compositor_state: Default::default(),
        });
        if let Err(err) = display_handle.insert_client(client_stream, client_state) {
            tracing::warn!("Error adding wayland client: {:?}", err);
        }
    })?;

    info!("Wayland socket available: {:?}", socket_name);
    std::env::set_var("WAYLAND_DISPLAY", &socket_name);

    info!("=== Frutiger Rust Compositor Ready ===");
    info!("Open a Wayland app in another terminal: WAYLAND_DISPLAY={} foot", socket_name);

    while state.is_running {
        let _status = winit_event_loop.dispatch_new_events(|event| match event {
            WinitEvent::Resized { size, .. } => {
                let mode = Mode {
                    size,
                    refresh: 60_000,
                };
                output.change_current_state(Some(mode), None, None, None);
            }
            WinitEvent::Input(_event) => {
                // Input dispatch
            }
            WinitEvent::CloseRequested => {
                state.is_running = false;
            }
            WinitEvent::Redraw => {
                let size = backend.window_size();
                let damage = Rectangle::new((0, 0).into(), size);

                // Update clock
                let now = std::time::SystemTime::now();
                if let Ok(duration) = now.duration_since(std::time::UNIX_EPOCH) {
                    let secs = duration.as_secs();
                    let hours = (secs / 3600 + 3) % 24; // MSK UTC+3
                    let mins = (secs % 3600) / 60;
                    shell.update_time(&format!("{:02}:{:02}", hours, mins));
                }

                let render_result = if let Ok((renderer, mut target)) = backend.bind() {
                    let elements: Vec<WaylandSurfaceRenderElement<GlesRenderer>> = Vec::new();
                    let clear_color = [0.10f32, 0.52, 0.82, 1.0]; // Vibrant Frutiger Aqua Glass Sky
                    damage_tracker.render_output(
                        renderer,
                        &mut target,
                        0,
                        &elements,
                        clear_color,
                    ).is_ok()
                } else {
                    false
                };

                if render_result {
                    let _ = backend.submit(Some(&[damage]));
                }
            }
            _ => {}
        });

        event_loop.dispatch(Some(std::time::Duration::from_millis(16)), &mut state)?;
    }

    Ok(())
}
