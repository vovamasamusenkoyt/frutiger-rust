slint::include_modules!();

pub struct ShellState {
    pub active_workspace: i32,
    pub launcher_open: bool,
    pub current_title: String,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            active_workspace: 1,
            launcher_open: false,
            current_title: "Frutiger Aero Desktop".to_string(),
        }
    }
}
