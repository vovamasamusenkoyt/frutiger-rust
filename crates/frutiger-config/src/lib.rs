use mlua::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct GlassConfig {
    pub blur_radius: f32,
    pub refraction_strength: f32,
    pub chromatic_aberration: f32,
    pub corner_radius: f32,
    pub specular_strength: f32,
    pub frost_noise: f32,
    pub tint_color: [f32; 4],
    pub border_color: [f32; 4],
}

impl Default for GlassConfig {
    fn default() -> Self {
        Self {
            blur_radius: 12.0,
            refraction_strength: 0.06,
            chromatic_aberration: 0.02,
            corner_radius: 16.0,
            specular_strength: 0.65,
            frost_noise: 0.025,
            tint_color: [0.12, 0.45, 0.78, 0.3],
            border_color: [1.0, 1.0, 1.0, 0.45],
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrutigerConfig {
    pub wallpaper_path: Option<String>,
    pub glass: GlassConfig,
    pub keybindings: HashMap<String, String>,
    pub autostart: Vec<String>,
    pub panel_position: String, // "top" or "bottom"
    pub panel_height: u32,
}

impl Default for FrutigerConfig {
    fn default() -> Self {
        let mut keybindings = HashMap::new();
        keybindings.insert("Super+Return".to_string(), "spawn:foot".to_string());
        keybindings.insert("Super+d".to_string(), "toggle_launcher".to_string());
        keybindings.insert("Super+Space".to_string(), "toggle_launcher".to_string());
        keybindings.insert("Super+q".to_string(), "close_window".to_string());
        keybindings.insert("Super+Shift+e".to_string(), "quit".to_string());

        Self {
            wallpaper_path: None,
            glass: GlassConfig::default(),
            keybindings,
            autostart: vec![],
            panel_position: "top".to_string(),
            panel_height: 38,
        }
    }
}

pub struct ConfigEngine {
    lua: Lua,
    pub config: FrutigerConfig,
}

impl ConfigEngine {
    pub fn new() -> anyhow::Result<Self> {
        let lua = Lua::new();
        let config = FrutigerConfig::default();
        let engine = Self { lua, config };
        engine.setup_lua_environment().map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(engine)
    }

    fn setup_lua_environment(&self) -> LuaResult<()> {
        let globals = self.lua.globals();

        // Create 'de' table
        let de = self.lua.create_table()?;

        // Version info
        de.set("version", "0.1.0")?;
        de.set("name", "Frutiger Rust")?;

        globals.set("de", de)?;
        Ok(())
    }

    pub fn load_config_file(&mut self, path: Option<PathBuf>) -> anyhow::Result<()> {
        let config_path = path.unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config/frutiger/config.lua")
        });

        if config_path.exists() {
            info!("Loading configuration from {:?}", config_path);
            let content = std::fs::read_to_string(&config_path)?;
            self.eval_lua_script(&content)?;
        } else {
            info!("Config file not found at {:?}, using default Frutiger Aero configuration.", config_path);
            let default_script = include_str!("../../../config.lua.example");
            self.eval_lua_script(default_script)?;
        }

        Ok(())
    }

    pub fn eval_lua_script(&mut self, script: &str) -> anyhow::Result<()> {
        self.eval_lua_inner(script).map_err(|e| anyhow::anyhow!("{e}"))
    }

    fn eval_lua_inner(&mut self, script: &str) -> LuaResult<()> {
        let globals = self.lua.globals();

        // Set up temporary bindings capture
        let keybinds_table = self.lua.create_table()?;
        let keys_ns = self.lua.create_table()?;
        let k_tbl = keybinds_table.clone();
        keys_ns.set(
            "bind",
            self.lua.create_function(move |_, (key, action): (String, String)| {
                k_tbl.set(key, action)?;
                Ok(())
            })?,
        )?;

        let de: LuaTable = globals.get("de")?;
        de.set("keys", keys_ns)?;

        // Execute user script
        if let Err(err) = self.lua.load(script).exec() {
            warn!("Error executing Lua config: {}", err);
        }

        // Parse keybindings
        for pair in keybinds_table.pairs::<String, String>() {
            if let Ok((k, v)) = pair {
                self.config.keybindings.insert(k, v);
            }
        }

        // Read wallpaper if set
        if let Ok(wp) = de.get::<Option<String>>("wallpaper") {
            if let Some(wp) = wp {
                self.config.wallpaper_path = Some(wp);
            }
        }

        // Read glass effects table
        if let Ok(glass_tbl) = de.get::<LuaTable>("glass") {
            if let Ok(blur) = glass_tbl.get::<f32>("blur_radius") {
                self.config.glass.blur_radius = blur;
            }
            if let Ok(refract) = glass_tbl.get::<f32>("refraction_strength") {
                self.config.glass.refraction_strength = refract;
            }
            if let Ok(aberration) = glass_tbl.get::<f32>("chromatic_aberration") {
                self.config.glass.chromatic_aberration = aberration;
            }
            if let Ok(radius) = glass_tbl.get::<f32>("corner_radius") {
                self.config.glass.corner_radius = radius;
            }
            if let Ok(specular) = glass_tbl.get::<f32>("specular_strength") {
                self.config.glass.specular_strength = specular;
            }
        }

        Ok(())
    }
}
