//! # hyprfetch
//!
//! A Hyprland-native fetch utility for system, session, and rice state.
//!
//! ## Features
//!
//! - System information modules (OS, CPU, GPU, memory, etc.)
//! - Hyprland session modules (version, workspaces, monitors, etc.)
//! - Rice/theme detection (GTK theme, wallpaper backend)
//! - Multiple render styles (split, compact, minimal)
//! - Color palettes (hypr, mono, catppuccin, dracula)
//! - Share preset for rice screenshots
//! - Command plugins for extensibility
//!
//! ## Example
//!
//! ```bash
//! # Basic usage
//! hyprfetch
//!
//! # Plain mode
//! hyprfetch --plain
//!
//! # Share preset
//! hyprfetch --share
//!
//! # Specific modules
//! hyprfetch --modules os,kernel,memory
//!
//! # JSON output
//! hyprfetch --json
//! ```

pub mod animation;
pub mod app;
pub mod cli;
pub mod config;
pub mod logo;
pub mod model;
pub mod modules;
pub mod plugins;
pub mod providers;
pub mod render;
pub mod util;
