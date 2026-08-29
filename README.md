# 🫧 Frutiger Rust

> A lightweight, native, GPU-accelerated monolithic Wayland Desktop Environment written in Rust with **Frutiger Aero** / **Liquid Glass** aesthetics.

![Rust](https://img.shields.io/badge/Language-Rust%202021-orange.svg)
![Wayland](https://img.shields.io/badge/Compositor-Smithay%200.7-blue.svg)
![Graphics](https://img.shields.io/badge/Graphics-wgpu%20%2F%20GLES-green.svg)
![UI](https://img.shields.io/badge/UI-Slint%201.17-purple.svg)
![Config](https://img.shields.io/badge/Config-Lua%205.4-yellow.svg)
![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-lightgrey.svg)

---

## ✨ Features

- 💎 **Frutiger Aero & Liquid Glass Aesthetics**: Multi-pass shaders with Dual-Kawase blur, chromatic aberration (RGB split), normal map refractions, and glossy specular borders.
- ⚡ **Ultra-Lightweight & Native**: Monolithic architecture without WebViews, Electron, or heavy runtimes (~9 MB release binary).
- 🦀 **Pure Rust Core**: Built on [Smithay](https://github.com/Smithay/smithay) for robust Wayland protocol handling and DRM/KMS backend.
- 🎨 **Slint Shell UI**: Declarative, native UI components for the glass taskbar, workspace switcher, and application launcher.
- 📜 **Lua Scripting Engine**: Highly customizable configuration and keybindings via `config.lua` powered by `mlua`.

---

## 🏗️ Architecture

```
frutiger-rust/
├── crates/
│   ├── frutiger-render/      # wgpu pipeline, WGSL Liquid Glass & Blur shaders
│   ├── frutiger-shell/       # Slint UI definitions (Bar, Launcher, Dock)
│   ├── frutiger-config/      # Lua configuration engine (mlua)
│   ├── frutiger-compositor/  # Smithay Wayland compositor (DRM/KMS & Nested Winit)
│   └── frutiger-de/          # Main application entry point
├── config.lua.example        # Example Lua configuration
└── deploy.sh                 # Fast build & deployment script
```

---

## 🚀 Getting Started

### Prerequisites

Make sure you have Rust and standard Wayland build dependencies installed:

```bash
# Arch Linux
sudo pacman -S base-devel libxkbcommon mesa libseat wayland-protocols
```

### Build & Run Locally (Nested Mode)

```bash
cargo run --bin frutiger-de -- --backend winit
```

### Run Native DRM/KMS on Real Hardware / TTY

```bash
cargo run --release --bin frutiger-de -- --backend drm
```

---

## 📝 Configuration (`~/.config/frutiger/config.lua`)

```lua
-- Liquid Glass Shader Effects
de.glass = {
    blur_radius = 14.0,
    refraction_strength = 0.07,
    chromatic_aberration = 0.02,
    corner_radius = 16.0,
    specular_strength = 0.7,
}

-- Keybindings
de.keys.bind("Super+Return", "spawn:foot")
de.keys.bind("Super+Space", "toggle_launcher")
de.keys.bind("Super+q", "close_window")
de.keys.bind("Super+Shift+e", "quit")
```

---

## 📜 License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
