use smithay::{
    delegate_compositor, delegate_output, delegate_seat, delegate_shm, delegate_xdg_shell,
    desktop::{Space, Window},
    input::{
        keyboard::{Keysym, ModifiersState},
        Seat, SeatHandler, SeatState,
    },
    output::Output,
    reexports::{
        calloop::LoopSignal,
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::{wl_buffer::WlBuffer, wl_surface::WlSurface},
            Display, DisplayHandle,
        },
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{CompositorClientState, CompositorHandler, CompositorState},
        output::{OutputHandler, OutputManagerState},
        shm::{ShmHandler, ShmState},
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
        },
    },
};
use tracing::{info, warn};

use frutiger_config::FrutigerConfig;

pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

pub struct FrutigerState {
    pub display_handle: DisplayHandle,
    pub loop_signal: LoopSignal,
    pub space: Space<Window>,
    pub seat: Seat<Self>,

    // Smithay States
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Self>,

    // Config & Shell State
    pub config: FrutigerConfig,
    pub active_workspace: usize,
    pub is_running: bool,
}

impl FrutigerState {
    pub fn new(
        display: &Display<Self>,
        loop_signal: LoopSignal,
        config: FrutigerConfig,
    ) -> Self {
        let display_handle = display.handle();

        let compositor_state = CompositorState::new::<Self>(&display_handle);
        let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);
        let shm_state = ShmState::new::<Self>(&display_handle, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&display_handle);
        let mut seat_state = SeatState::new();

        let mut seat = seat_state.new_wl_seat(&display_handle, "seat0");
        seat.add_keyboard(Default::default(), 200, 25).expect("Failed to initialize keyboard");
        seat.add_pointer();

        let space = Space::default();

        Self {
            display_handle,
            loop_signal,
            space,
            seat,
            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager_state,
            seat_state,
            config,
            active_workspace: 1,
            is_running: true,
        }
    }

    pub fn handle_key_action(&mut self, keysym: Keysym, modifiers: &ModifiersState) {
        let is_super = modifiers.logo;
        let is_shift = modifiers.shift;
        let is_ctrl = modifiers.ctrl;

        // Construct key string matching config format
        let key_name = match keysym {
            Keysym::Return => "Return",
            Keysym::q | Keysym::Q => "q",
            Keysym::d | Keysym::D => "d",
            Keysym::t | Keysym::T => "t",
            Keysym::e | Keysym::E => "e",
            Keysym::space => "Space",
            _ => return,
        };

        let mut combo = String::new();
        if is_super {
            combo.push_str("Super+");
        }
        if is_ctrl {
            combo.push_str("Ctrl+");
        }
        if is_shift {
            combo.push_str("Shift+");
        }
        combo.push_str(key_name);

        if let Some(action) = self.config.keybindings.get(&combo).cloned() {
            info!("Triggered key action: {} -> {}", combo, action);
            self.execute_action(&action);
        }
    }

    pub fn execute_action(&mut self, action: &str) {
        if let Some(cmd) = action.strip_prefix("spawn:") {
            info!("Spawning command: {}", cmd);
            let _ = std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .spawn();
        } else {
            match action {
                "quit" => {
                    info!("Quitting Frutiger Rust DE...");
                    self.is_running = false;
                    self.loop_signal.stop();
                }
                "close_window" => {
                    if let Some(window) = self.space.elements().last().cloned() {
                        if let Some(toplevel) = window.toplevel() {
                            toplevel.send_close();
                        }
                    }
                }
                _ => {
                    warn!("Unknown action: {}", action);
                }
            }
        }
    }
}

// Delegate macros for Smithay protocols
delegate_compositor!(FrutigerState);
delegate_output!(FrutigerState);
delegate_shm!(FrutigerState);
delegate_seat!(FrutigerState);
delegate_xdg_shell!(FrutigerState);

impl BufferHandler for FrutigerState {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl CompositorHandler for FrutigerState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a smithay::reexports::wayland_server::Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        smithay::backend::renderer::utils::on_commit_buffer_handler::<Self>(surface);
    }
}

impl OutputHandler for FrutigerState {}

impl ShmHandler for FrutigerState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl SeatHandler for FrutigerState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {}
    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: smithay::input::pointer::CursorImageStatus) {}
}

impl XdgShellHandler for FrutigerState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new_wayland_window(surface.clone());
        self.space.map_element(window, (100, 100), true);
        surface.send_configure();
        info!("New XDG toplevel window mapped!");
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {}
    fn grab(&mut self, _surface: PopupSurface, _seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat, _serial: smithay::utils::Serial) {}
    fn reposition_request(&mut self, _surface: PopupSurface, _positioner: PositionerState, _token: u32) {}
}
