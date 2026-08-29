use std::f64::consts::PI;
use cairo::{Context, Format, ImageSurface, LinearGradient};
use chrono::Local;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
        },
        WaylandSurface,
    },
    shm::{
        slot::SlotPool,
        Shm, ShmHandler,
    },
};
use tracing::info;
use wayland_client::{
    globals::{registry_queue_init, GlobalList},
    protocol::{wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, QueueHandle,
};

const PANEL_HEIGHT: u32 = 36;

struct FrutigerPanelApp {
    registry_state: RegistryState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm: Shm,
    layer_shell: LayerShell,
    seat_state: SeatState,
    slot_pool: Option<SlotPool>,

    layer_surface: Option<LayerSurface>,
    width: u32,
    height: u32,
    active_workspace: u32,
    _pointer_pos: (f64, f64),
    is_hovering_orb: bool,
    hovered_ws: Option<u32>,
}

impl FrutigerPanelApp {
    fn new(globals: &GlobalList, qh: &QueueHandle<Self>) -> Self {
        let registry_state = RegistryState::new(globals);
        let output_state = OutputState::new(globals, qh);
        let compositor_state =
            CompositorState::bind(globals, qh).expect("wl_compositor is required");
        let shm = Shm::bind(globals, qh).expect("wl_shm is required");
        let layer_shell =
            LayerShell::bind(globals, qh).expect("zwlr_layer_shell_v1 is required");
        let seat_state = SeatState::new(globals, qh);

        Self {
            registry_state,
            output_state,
            compositor_state,
            shm,
            layer_shell,
            seat_state,
            slot_pool: None,
            layer_surface: None,
            width: 1920,
            height: PANEL_HEIGHT,
            active_workspace: 1,
            _pointer_pos: (0.0, 0.0),
            is_hovering_orb: false,
            hovered_ws: None,
        }
    }

    fn init_surface(&mut self, qh: &QueueHandle<Self>) {
        let surface = self.compositor_state.create_surface(qh);
        let layer_surface = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Top,
            Some("frutiger-panel"),
            None,
        );

        layer_surface.set_anchor(Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
        layer_surface.set_size(0, PANEL_HEIGHT);
        layer_surface.set_exclusive_zone(PANEL_HEIGHT as i32);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer_surface.commit();

        self.layer_surface = Some(layer_surface);
        self.slot_pool = Some(
            SlotPool::new((self.width * PANEL_HEIGHT * 4) as usize, &self.shm)
                .expect("Failed to create SHM SlotPool"),
        );
    }

    fn draw(&mut self, _qh: &QueueHandle<Self>) {
        let width = self.width as i32;
        let height = self.height as i32;
        if width <= 0 || height <= 0 {
            return;
        }

        let stride = width * 4;
        let pool = match self.slot_pool.as_mut() {
            Some(p) => p,
            None => return,
        };

        let (buffer, canvas_slice) = pool
            .create_buffer(
                width,
                height,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("Failed to create buffer");

        // Render with Cairo
        {
            let surface = unsafe {
                ImageSurface::create_for_data_unsafe(
                    canvas_slice.as_mut_ptr(),
                    Format::ARgb32,
                    width,
                    height,
                    stride,
                )
                .expect("Failed to create Cairo ImageSurface")
            };

            let cr = Context::new(&surface).expect("Failed to create Cairo context");

            // 1. Clear background
            cr.set_operator(cairo::Operator::Source);
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
            cr.paint().unwrap();
            cr.set_operator(cairo::Operator::Over);

            // 2. Liquid Glass Background Gradient
            let bg_grad = LinearGradient::new(0.0, 0.0, 0.0, height as f64);
            bg_grad.add_color_stop_rgba(0.0, 0.06, 0.18, 0.30, 0.88);
            bg_grad.add_color_stop_rgba(0.5, 0.03, 0.10, 0.20, 0.92);
            bg_grad.add_color_stop_rgba(1.0, 0.01, 0.06, 0.12, 0.96);
            cr.set_source(&bg_grad).unwrap();
            cr.paint().unwrap();

            // 3. White Specular Top Glint Line (Glossy reflection)
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.75);
            cr.set_line_width(1.0);
            cr.move_to(0.0, 0.5);
            cr.line_to(width as f64, 0.5);
            cr.stroke().unwrap();

            // Bottom subtle border line
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.4);
            cr.move_to(0.0, height as f64 - 0.5);
            cr.line_to(width as f64, height as f64 - 0.5);
            cr.stroke().unwrap();

            // 4. Left: Frutiger Aqua Orb Button
            let orb_x = 10.0;
            let orb_y = 4.0;
            let orb_w = 105.0;
            let orb_h = 28.0;
            let orb_r = 14.0;

            draw_rounded_rect(&cr, orb_x, orb_y, orb_w, orb_h, orb_r);
            let orb_grad = LinearGradient::new(orb_x, orb_y, orb_x, orb_y + orb_h);
            if self.is_hovering_orb {
                orb_grad.add_color_stop_rgba(0.0, 0.35, 0.85, 1.0, 1.0);
                orb_grad.add_color_stop_rgba(0.45, 0.0, 0.65, 0.95, 1.0);
                orb_grad.add_color_stop_rgba(1.0, 0.0, 0.42, 0.78, 1.0);
            } else {
                orb_grad.add_color_stop_rgba(0.0, 0.25, 0.76, 0.98, 0.95);
                orb_grad.add_color_stop_rgba(0.45, 0.0, 0.55, 0.88, 0.95);
                orb_grad.add_color_stop_rgba(1.0, 0.0, 0.35, 0.70, 0.95);
            }
            cr.set_source(&orb_grad).unwrap();
            cr.fill_preserve().unwrap();

            // Orb Border & Gloss Highlight
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.6);
            cr.set_line_width(1.0);
            cr.stroke().unwrap();

            // Top Sheen Half-Pill
            draw_rounded_rect(&cr, orb_x + 2.0, orb_y + 1.0, orb_w - 4.0, orb_h * 0.46, orb_r * 0.8);
            let sheen_grad = LinearGradient::new(orb_x, orb_y, orb_x, orb_y + orb_h * 0.46);
            sheen_grad.add_color_stop_rgba(0.0, 1.0, 1.0, 1.0, 0.70);
            sheen_grad.add_color_stop_rgba(1.0, 1.0, 1.0, 1.0, 0.05);
            cr.set_source(&sheen_grad).unwrap();
            cr.fill().unwrap();

            // Orb Text: "🫧 Frutiger"
            let pango_ctx = pangocairo::functions::create_context(&cr);
            let layout = pango::Layout::new(&pango_ctx);
            let mut font_desc = pango::FontDescription::from_string("Cantarell, Inter, Sans-Serif Bold 11");
            layout.set_font_description(Some(&font_desc));
            layout.set_text("🫧 Frutiger");

            cr.move_to(orb_x + 12.0, orb_y + 4.5);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.98);
            pangocairo::functions::show_layout(&cr, &layout);

            // 5. Workspaces (1..4)
            for ws in 1..=4 {
                let ws_x = orb_x + orb_w + 14.0 + (ws as f64 - 1.0) * 34.0;
                let ws_y = 5.0;
                let ws_w = 28.0;
                let ws_h = 26.0;
                let ws_r = 13.0;

                draw_rounded_rect(&cr, ws_x, ws_y, ws_w, ws_h, ws_r);

                if ws == self.active_workspace {
                    let ws_grad = LinearGradient::new(ws_x, ws_y, ws_x, ws_y + ws_h);
                    ws_grad.add_color_stop_rgba(0.0, 0.25, 0.85, 1.0, 0.95);
                    ws_grad.add_color_stop_rgba(1.0, 0.0, 0.50, 0.85, 0.95);
                    cr.set_source(&ws_grad).unwrap();
                    cr.fill_preserve().unwrap();
                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.7);
                    cr.stroke().unwrap();
                } else if self.hovered_ws == Some(ws) {
                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.22);
                    cr.fill_preserve().unwrap();
                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.4);
                    cr.stroke().unwrap();
                } else {
                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.10);
                    cr.fill_preserve().unwrap();
                    cr.set_source_rgba(1.0, 1.0, 1.0, 0.2);
                    cr.stroke().unwrap();
                }

                layout.set_text(&ws.to_string());
                cr.move_to(ws_x + 9.5, ws_y + 4.0);
                cr.set_source_rgba(1.0, 1.0, 1.0, if ws == self.active_workspace { 1.0 } else { 0.85 });
                pangocairo::functions::show_layout(&cr, &layout);
            }

            // 6. Center: Live Date & Clock
            let now = Local::now();
            let time_str = now.format("%a, %d %b   %H:%M").to_string();

            let center_w = 200.0;
            let center_x = (width as f64 - center_w) / 2.0;
            let center_y = 4.0;
            let center_h = 28.0;

            draw_rounded_rect(&cr, center_x, center_y, center_w, center_h, 14.0);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.08);
            cr.fill_preserve().unwrap();
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.18);
            cr.stroke().unwrap();

            font_desc.set_weight(pango::Weight::Medium);
            layout.set_font_description(Some(&font_desc));
            layout.set_text(&time_str);

            let (tw, _th) = layout.pixel_size();
            cr.move_to(center_x + (center_w - tw as f64) / 2.0, center_y + 5.0);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.95);
            pangocairo::functions::show_layout(&cr, &layout);

            // 7. Right: System Status Glass Pills
            let right_w = 175.0;
            let right_x = width as f64 - right_w - 10.0;
            let right_y = 4.0;
            let right_h = 28.0;

            draw_rounded_rect(&cr, right_x, right_y, right_w, right_h, 14.0);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.08);
            cr.fill_preserve().unwrap();
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.18);
            cr.stroke().unwrap();

            layout.set_text("⚡ 85%   🔊 70%   ⏻");
            cr.move_to(right_x + 14.0, right_y + 5.0);
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.95);
            pangocairo::functions::show_layout(&cr, &layout);
        }

        // Commit frame to Wayland surface
        if let Some(layer_surface) = &self.layer_surface {
            buffer
                .attach_to(layer_surface.wl_surface())
                .expect("Failed to attach buffer");
            layer_surface.wl_surface().damage_buffer(0, 0, width, height);
            layer_surface.commit();
        }
    }

    fn on_click(&mut self, x: f64, _y: f64) {
        // Frutiger Orb Clicked
        if x >= 10.0 && x <= 115.0 {
            info!("🫧 Frutiger Orb clicked! Spawning application launcher or overview...");
            let _ = std::process::Command::new("fuzzel").spawn();
        }

        // Workspaces 1..4
        for ws in 1..=4 {
            let ws_x = 10.0 + 105.0 + 14.0 + (ws as f64 - 1.0) * 34.0;
            if x >= ws_x && x <= ws_x + 28.0 {
                info!("Switched to workspace {}", ws);
                self.active_workspace = ws;
                let _ = std::process::Command::new("frutiger")
                    .args(["msg", "action", "focus-workspace", &ws.to_string()])
                    .spawn();
            }
        }

        // Right Power Button Clicked
        if x >= (self.width as f64 - 45.0) {
            info!("Power menu clicked!");
            let _ = std::process::Command::new("frutiger")
                .args(["msg", "action", "quit"])
                .spawn();
        }
    }
}

fn draw_rounded_rect(cr: &Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cr.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
    cr.close_path();
}

impl CompositorHandler for FrutigerPanelApp {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wayland_client::protocol::wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw(qh);
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for FrutigerPanelApp {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for FrutigerPanelApp {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        info!("Layer surface closed");
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: smithay_client_toolkit::shell::wlr_layer::LayerSurfaceConfigure,
        _serial: u32,
    ) {
        if configure.new_size.0 > 0 {
            self.width = configure.new_size.0;
        }
        if configure.new_size.1 > 0 {
            self.height = configure.new_size.1;
        }

        self.slot_pool = Some(
            SlotPool::new((self.width * self.height * 4) as usize, &self.shm)
                .expect("Failed to resize SlotPool"),
        );

        self.draw(qh);
    }
}

impl SeatHandler for FrutigerPanelApp {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            let _ = self.seat_state.get_pointer(qh, &seat);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}
}

impl PointerHandler for FrutigerPanelApp {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            match event.kind {
                PointerEventKind::Motion { .. } => {
                    self._pointer_pos = event.position;
                    let was_hover = self.is_hovering_orb;
                    self.is_hovering_orb = event.position.0 >= 10.0 && event.position.0 <= 115.0;

                    let mut hovered_ws = None;
                    for ws in 1..=4 {
                        let ws_x = 10.0 + 105.0 + 14.0 + (ws as f64 - 1.0) * 34.0;
                        if event.position.0 >= ws_x && event.position.0 <= ws_x + 28.0 {
                            hovered_ws = Some(ws);
                            break;
                        }
                    }

                    if was_hover != self.is_hovering_orb || self.hovered_ws != hovered_ws {
                        self.hovered_ws = hovered_ws;
                        self.draw(qh);
                    }
                }
                PointerEventKind::Press { button, .. } => {
                    if button == 0x110 {
                        // Left click
                        self.on_click(event.position.0, event.position.1);
                        self.draw(qh);
                    }
                }
                _ => {}
            }
        }
    }
}

impl ShmHandler for FrutigerPanelApp {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_compositor!(FrutigerPanelApp);
delegate_output!(FrutigerPanelApp);
delegate_shm!(FrutigerPanelApp);
delegate_layer!(FrutigerPanelApp);
delegate_seat!(FrutigerPanelApp);
delegate_pointer!(FrutigerPanelApp);
delegate_registry!(FrutigerPanelApp);

impl ProvidesRegistryState for FrutigerPanelApp {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("🫧 Starting Frutiger Liquid Glass Top Panel (Layer-Shell) 🫧");

    let conn = Connection::connect_to_env()
        .map_err(|e| anyhow::anyhow!("Failed to connect to Wayland display: {:?}", e))?;

    let (globals, mut event_queue) = registry_queue_init::<FrutigerPanelApp>(&conn)?;
    let qh = event_queue.handle();

    let mut app = FrutigerPanelApp::new(&globals, &qh);
    app.init_surface(&qh);

    let _ = event_queue.roundtrip(&mut app);

    let mut last_draw = std::time::Instant::now();
    loop {
        event_queue.blocking_dispatch(&mut app)?;

        if last_draw.elapsed() >= std::time::Duration::from_secs(1) {
            app.draw(&qh);
            last_draw = std::time::Instant::now();
        }
    }
}
