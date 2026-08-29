slint::include_modules!();

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use slint::ComponentHandle;

pub struct FrutigerShell {
    pub window: ShellWindow,
    pub is_launcher_open: Arc<AtomicBool>,
}

impl FrutigerShell {
    pub fn new() -> anyhow::Result<Self> {
        let window = ShellWindow::new()
            .map_err(|e| anyhow::anyhow!("Failed to create ShellWindow: {:?}", e))?;

        let is_launcher_open = Arc::new(AtomicBool::new(false));
        let open_clone = is_launcher_open.clone();

        // Setup callbacks
        window.on_toggle_launcher(move || {
            let current = open_clone.load(Ordering::SeqCst);
            open_clone.store(!current, Ordering::SeqCst);
            tracing::info!("Launcher toggled. Open: {}", !current);
        });

        window.on_workspace_selected(|ws| {
            tracing::info!("Switched to workspace: {}", ws);
        });

        window.on_launch_app(|app| {
            let app_str = app.to_string();
            tracing::info!("Launching application from shell: {}", app_str);
            if let Some(cmd) = app_str.strip_prefix("spawn:") {
                let _ = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(cmd)
                    .spawn();
            } else {
                let _ = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&app_str)
                    .spawn();
            }
        });

        Ok(Self {
            window,
            is_launcher_open,
        })
    }

    pub fn update_time(&self, time_str: &str) {
        self.window.set_time_text(time_str.into());
    }

    pub fn update_window_title(&self, title: &str) {
        self.window.set_current_window_title(title.into());
    }

    pub fn set_active_workspace(&self, ws: i32) {
        self.window.set_active_workspace(ws);
    }
}
