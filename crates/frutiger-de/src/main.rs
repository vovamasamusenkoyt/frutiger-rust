use clap::Parser;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use frutiger_compositor::{run_drm, run_winit};
use frutiger_config::ConfigEngine;

#[derive(Parser, Debug)]
#[command(name = "frutiger-de")]
#[command(author = "vmko")]
#[command(version = "0.1.0")]
#[command(about = "Frutiger Aero Desktop Environment on Rust + Smithay + wgpu + Slint + Lua", long_about = None)]
struct Args {
    /// Backend to run: 'winit' (nested window for testing) or 'drm' (native KMS/hardware)
    #[arg(short, long, default_value = "winit")]
    backend: String,

    /// Path to custom Lua configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,smithay=warn,wgpu=warn")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🫧 Starting Frutiger Rust DE v0.1.0 🫧");

    let args = Args::parse();

    // Load Lua Configuration
    let mut config_engine = ConfigEngine::new()?;
    config_engine.load_config_file(args.config)?;
    let config = config_engine.config;

    info!("Loaded Frutiger config. Glass blur: {}, refraction: {}",
        config.glass.blur_radius,
        config.glass.refraction_strength
    );

    // Run chosen backend
    match args.backend.to_lowercase().as_str() {
        "drm" => {
            info!("Launching Native DRM/KMS backend on GPU...");
            run_drm(config)?;
        }
        "winit" | "nested" => {
            info!("Launching Nested Winit backend window...");
            run_winit(config)?;
        }
        unknown => {
            anyhow::bail!("Unknown backend: '{}'. Expected 'winit' or 'drm'", unknown);
        }
    }

    Ok(())
}
