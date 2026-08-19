use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Color output mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

impl Display for ColorMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Never => "never",
        };
        f.write_str(value)
    }
}

impl FromStr for ColorMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            _ => Err(format!("invalid color mode: {value}")),
        }
    }
}

/// Render style for output formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RenderStyle {
    #[default]
    Split,
    Compact,
    Minimal,
}

impl Display for RenderStyle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Split => "split",
            Self::Compact => "compact",
            Self::Minimal => "minimal",
        };
        f.write_str(value)
    }
}

impl FromStr for RenderStyle {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "split" => Ok(Self::Split),
            "compact" => Ok(Self::Compact),
            "minimal" => Ok(Self::Minimal),
            _ => Err(format!("invalid style: {value}")),
        }
    }
}

/// Color palette for styled output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PaletteName {
    #[default]
    Hypr,
    Mono,
    Catppuccin,
    Dracula,
}

impl PaletteName {
    pub const fn all() -> &'static [Self] {
        &[Self::Hypr, Self::Mono, Self::Catppuccin, Self::Dracula]
    }
}

impl Display for PaletteName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Hypr => "hypr",
            Self::Mono => "mono",
            Self::Catppuccin => "catppuccin",
            Self::Dracula => "dracula",
        };
        f.write_str(value)
    }
}

impl FromStr for PaletteName {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "hypr" => Ok(Self::Hypr),
            "mono" => Ok(Self::Mono),
            "catppuccin" => Ok(Self::Catppuccin),
            "dracula" => Ok(Self::Dracula),
            _ => Err(format!("invalid theme: {value}")),
        }
    }
}

/// Logo preset for distro-specific ASCII art.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum LogoPreset {
    #[default]
    Auto,
    None,
    Linux,
    Windows,
    Macos,
    Hypr,
    Arch,
    Garuda,
    Nobara,
    Artix,
    Asahi,
    Cachyos,
    Endeavouros,
    Fedora,
    Ubuntu,
    Debian,
    Nixos,
    Opensuse,
    Void,
    Alpine,
    Pop,
    Gentoo,
    Manjaro,
    Minimal,
}

impl LogoPreset {
    pub const fn all() -> &'static [Self] {
        &[
            Self::Auto,
            Self::Hypr,
            Self::Linux,
            Self::Windows,
            Self::Macos,
            Self::Arch,
            Self::Garuda,
            Self::Nobara,
            Self::Artix,
            Self::Asahi,
            Self::Cachyos,
            Self::Endeavouros,
            Self::Fedora,
            Self::Ubuntu,
            Self::Debian,
            Self::Nixos,
            Self::Opensuse,
            Self::Void,
            Self::Alpine,
            Self::Pop,
            Self::Gentoo,
            Self::Manjaro,
            Self::Minimal,
            Self::None,
        ]
    }
}

impl Display for LogoPreset {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Auto => "auto",
            Self::None => "none",
            Self::Linux => "linux",
            Self::Windows => "windows",
            Self::Macos => "macos",
            Self::Hypr => "hypr",
            Self::Arch => "arch",
            Self::Garuda => "garuda",
            Self::Nobara => "nobara",
            Self::Artix => "artix",
            Self::Asahi => "asahi",
            Self::Cachyos => "cachyos",
            Self::Endeavouros => "endeavouros",
            Self::Fedora => "fedora",
            Self::Ubuntu => "ubuntu",
            Self::Debian => "debian",
            Self::Nixos => "nixos",
            Self::Opensuse => "opensuse",
            Self::Void => "void",
            Self::Alpine => "alpine",
            Self::Pop => "pop",
            Self::Gentoo => "gentoo",
            Self::Manjaro => "manjaro",
            Self::Minimal => "minimal",
        };
        f.write_str(value)
    }
}

impl FromStr for LogoPreset {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let normalized = value
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>();

        match normalized.as_str() {
            "auto" => Ok(Self::Auto),
            "none" => Ok(Self::None),
            "linux" => Ok(Self::Linux),
            "windows" | "windowsnt" | "win32" | "win64" => Ok(Self::Windows),
            "macos" | "macosx" | "osx" | "mac" | "darwin" => Ok(Self::Macos),
            "hypr" => Ok(Self::Hypr),
            "arch" | "archlinux" => Ok(Self::Arch),
            "garuda" => Ok(Self::Garuda),
            "nobara" => Ok(Self::Nobara),
            "artix" => Ok(Self::Artix),
            "asahi" => Ok(Self::Asahi),
            "cachyos" => Ok(Self::Cachyos),
            "endeavouros" | "endeavour" | "eos" => Ok(Self::Endeavouros),
            "fedora" => Ok(Self::Fedora),
            "ubuntu" => Ok(Self::Ubuntu),
            "debian" => Ok(Self::Debian),
            "nixos" => Ok(Self::Nixos),
            "opensuse" | "suse" => Ok(Self::Opensuse),
            "void" => Ok(Self::Void),
            "alpine" => Ok(Self::Alpine),
            "pop" | "popos" => Ok(Self::Pop),
            "gentoo" => Ok(Self::Gentoo),
            "manjaro" => Ok(Self::Manjaro),
            "minimal" => Ok(Self::Minimal),
            _ => Err(format!("invalid logo preset: {value}")),
        }
    }
}

/// Module identifier for system and Hyprland information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModuleId {
    Os,
    Host,
    Kernel,
    Uptime,
    Packages,
    Shell,
    Terminal,
    Cpu,
    Gpu,
    Memory,
    Swap,
    Disk,
    Battery,
    Locale,
    Network,
    Temperature,
    Audio,
    Datetime,
    Colorscheme,
    Hyprland,
    HyprHealth,
    HyprInstances,
    HyprSplash,
    HyprWorkspaces,
    HyprMonitors,
    HyprActiveWindow,
    HyprCursor,
    HyprWorkspaceHistory,
    HyprRice,
    HyprDevices,
    HyprBinds,
    HyprLayers,
    HyprConfig,
    HyprRules,
    HyprShortcuts,
    HyprLayouts,
    HyprEcosystem,
    Theme,
    Wallpaper,
}

impl ModuleId {
    pub const fn all() -> &'static [Self] {
        &[
            Self::Os,
            Self::Host,
            Self::Kernel,
            Self::Uptime,
            Self::Packages,
            Self::Shell,
            Self::Terminal,
            Self::Cpu,
            Self::Gpu,
            Self::Memory,
            Self::Swap,
            Self::Disk,
            Self::Battery,
            Self::Locale,
            Self::Network,
            Self::Temperature,
            Self::Audio,
            Self::Datetime,
            Self::Colorscheme,
            Self::Hyprland,
            Self::HyprHealth,
            Self::HyprInstances,
            Self::HyprSplash,
            Self::HyprWorkspaces,
            Self::HyprMonitors,
            Self::HyprActiveWindow,
            Self::HyprCursor,
            Self::HyprWorkspaceHistory,
            Self::HyprRice,
            Self::HyprDevices,
            Self::HyprBinds,
            Self::HyprLayers,
            Self::HyprConfig,
            Self::HyprRules,
            Self::HyprShortcuts,
            Self::HyprLayouts,
            Self::HyprEcosystem,
            Self::Theme,
            Self::Wallpaper,
        ]
    }

    pub const fn default_order() -> &'static [Self] {
        &[
            Self::Os,
            Self::Host,
            Self::Kernel,
            Self::Uptime,
            Self::Packages,
            Self::Shell,
            Self::Terminal,
            Self::Cpu,
            Self::Gpu,
            Self::Memory,
            Self::Swap,
            Self::Disk,
            Self::Battery,
            Self::Locale,
            Self::Network,
            Self::Temperature,
            Self::Audio,
            Self::Datetime,
            Self::Colorscheme,
            Self::Hyprland,
            Self::HyprHealth,
            Self::HyprInstances,
            Self::HyprSplash,
            Self::HyprActiveWindow,
            Self::HyprCursor,
            Self::HyprWorkspaces,
            Self::HyprMonitors,
            Self::HyprConfig,
            Self::HyprRules,
            Self::HyprDevices,
            Self::HyprBinds,
            Self::HyprShortcuts,
            Self::HyprLayouts,
            Self::HyprLayers,
            Self::Theme,
            Self::Wallpaper,
            Self::HyprEcosystem,
        ]
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Os => "OS",
            Self::Host => "Host",
            Self::Kernel => "Kernel",
            Self::Uptime => "Uptime",
            Self::Packages => "Packages",
            Self::Shell => "Shell",
            Self::Terminal => "Terminal",
            Self::Cpu => "CPU",
            Self::Gpu => "GPU",
            Self::Memory => "Memory",
            Self::Swap => "Swap",
            Self::Disk => "Disk",
            Self::Battery => "Battery",
            Self::Locale => "Locale",
            Self::Network => "Network",
            Self::Temperature => "Temp",
            Self::Audio => "Audio",
            Self::Datetime => "DateTime",
            Self::Colorscheme => "Colors",
            Self::Hyprland => "Hyprland",
            Self::HyprHealth => "Health",
            Self::HyprInstances => "Instances",
            Self::HyprSplash => "Splash",
            Self::HyprWorkspaces => "Workspaces",
            Self::HyprMonitors => "Monitors",
            Self::HyprActiveWindow => "Window",
            Self::HyprCursor => "Cursor",
            Self::HyprWorkspaceHistory => "Hist",
            Self::HyprRice => "Hypr Rice",
            Self::HyprDevices => "Input",
            Self::HyprBinds => "Binds",
            Self::HyprLayers => "Layers",
            Self::HyprConfig => "Hypr Config",
            Self::HyprRules => "Hypr Rules",
            Self::HyprShortcuts => "Shortcuts",
            Self::HyprLayouts => "Layouts",
            Self::HyprEcosystem => "Ecosystem",
            Self::Theme => "Theme",
            Self::Wallpaper => "Wallpaper",
        }
    }
}

impl Display for ModuleId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Os => "os",
            Self::Host => "host",
            Self::Kernel => "kernel",
            Self::Uptime => "uptime",
            Self::Packages => "packages",
            Self::Shell => "shell",
            Self::Terminal => "terminal",
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
            Self::Memory => "memory",
            Self::Swap => "swap",
            Self::Disk => "disk",
            Self::Battery => "battery",
            Self::Locale => "locale",
            Self::Network => "network",
            Self::Temperature => "temperature",
            Self::Audio => "audio",
            Self::Datetime => "datetime",
            Self::Colorscheme => "colorscheme",
            Self::Hyprland => "hyprland",
            Self::HyprHealth => "hypr-health",
            Self::HyprInstances => "hypr-instances",
            Self::HyprSplash => "hypr-splash",
            Self::HyprWorkspaces => "hypr-workspaces",
            Self::HyprMonitors => "hypr-monitors",
            Self::HyprActiveWindow => "hypr-active-window",
            Self::HyprCursor => "hypr-cursor",
            Self::HyprWorkspaceHistory => "hypr-workspace-history",
            Self::HyprRice => "hypr-rice",
            Self::HyprDevices => "hypr-devices",
            Self::HyprBinds => "hypr-binds",
            Self::HyprLayers => "hypr-layers",
            Self::HyprConfig => "hypr-config",
            Self::HyprRules => "hypr-rules",
            Self::HyprShortcuts => "hypr-shortcuts",
            Self::HyprLayouts => "hypr-layouts",
            Self::HyprEcosystem => "hypr-ecosystem",
            Self::Theme => "theme",
            Self::Wallpaper => "wallpaper",
        };
        f.write_str(value)
    }
}

impl FromStr for ModuleId {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "os" => Ok(Self::Os),
            "host" => Ok(Self::Host),
            "kernel" => Ok(Self::Kernel),
            "uptime" => Ok(Self::Uptime),
            "packages" => Ok(Self::Packages),
            "shell" => Ok(Self::Shell),
            "terminal" => Ok(Self::Terminal),
            "cpu" => Ok(Self::Cpu),
            "gpu" => Ok(Self::Gpu),
            "memory" => Ok(Self::Memory),
            "swap" => Ok(Self::Swap),
            "disk" => Ok(Self::Disk),
            "battery" => Ok(Self::Battery),
            "locale" => Ok(Self::Locale),
            "network" => Ok(Self::Network),
            "temperature" => Ok(Self::Temperature),
            "audio" => Ok(Self::Audio),
            "datetime" => Ok(Self::Datetime),
            "colorscheme" => Ok(Self::Colorscheme),
            "hyprland" => Ok(Self::Hyprland),
            "hypr-health" => Ok(Self::HyprHealth),
            "hypr-instances" => Ok(Self::HyprInstances),
            "hypr-splash" => Ok(Self::HyprSplash),
            "hypr-workspaces" => Ok(Self::HyprWorkspaces),
            "hypr-monitors" => Ok(Self::HyprMonitors),
            "hypr-active-window" => Ok(Self::HyprActiveWindow),
            "hypr-cursor" => Ok(Self::HyprCursor),
            "hypr-workspace-history" => Ok(Self::HyprWorkspaceHistory),
            "hypr-rice" => Ok(Self::HyprRice),
            "hypr-devices" => Ok(Self::HyprDevices),
            "hypr-binds" => Ok(Self::HyprBinds),
            "hypr-layers" => Ok(Self::HyprLayers),
            "hypr-config" => Ok(Self::HyprConfig),
            "hypr-rules" => Ok(Self::HyprRules),
            "hypr-shortcuts" => Ok(Self::HyprShortcuts),
            "hypr-layouts" => Ok(Self::HyprLayouts),
            "hypr-ecosystem" => Ok(Self::HyprEcosystem),
            "theme" => Ok(Self::Theme),
            "wallpaper" => Ok(Self::Wallpaper),
            _ => Err(format!("invalid module id: {value}")),
        }
    }
}

/// Status of a module's output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModuleStatus {
    Ok,
    Partial,
    Unavailable,
}

/// A key-value row in module output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputRow {
    pub key: String,
    pub value: String,
}

impl OutputRow {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

/// Output from a module execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleOutput {
    pub id: String,
    pub status: ModuleStatus,
    pub rows: Vec<OutputRow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub data: Value,
}

impl ModuleOutput {
    pub fn ok(id: ModuleId, rows: Vec<OutputRow>, data: Value) -> Self {
        Self::ok_custom(id.to_string(), rows, data)
    }

    pub fn ok_custom(id: impl Into<String>, rows: Vec<OutputRow>, data: Value) -> Self {
        Self {
            id: id.into(),
            status: ModuleStatus::Ok,
            rows,
            message: None,
            data,
        }
    }

    pub fn partial(
        id: ModuleId,
        rows: Vec<OutputRow>,
        data: Value,
        message: impl Into<String>,
    ) -> Self {
        Self::partial_custom(id.to_string(), rows, data, message)
    }

    pub fn partial_custom(
        id: impl Into<String>,
        rows: Vec<OutputRow>,
        data: Value,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            status: ModuleStatus::Partial,
            rows,
            message: Some(message.into()),
            data,
        }
    }

    pub fn unavailable(id: ModuleId, message: impl Into<String>) -> Self {
        Self::unavailable_custom(id.to_string(), message)
    }

    pub fn unavailable_custom(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: ModuleStatus::Unavailable,
            rows: Vec::new(),
            message: Some(message.into()),
            data: Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ModuleId;

    #[test]
    fn default_order_groups_system_session_and_rice_modules() {
        assert_eq!(
            ModuleId::default_order(),
            &[
                ModuleId::Os,
                ModuleId::Host,
                ModuleId::Kernel,
                ModuleId::Uptime,
                ModuleId::Packages,
                ModuleId::Shell,
                ModuleId::Terminal,
                ModuleId::Cpu,
                ModuleId::Gpu,
                ModuleId::Memory,
                ModuleId::Swap,
                ModuleId::Disk,
                ModuleId::Battery,
                ModuleId::Locale,
                ModuleId::Network,
                ModuleId::Temperature,
                ModuleId::Audio,
                ModuleId::Datetime,
                ModuleId::Colorscheme,
                ModuleId::Hyprland,
                ModuleId::HyprHealth,
                ModuleId::HyprInstances,
                ModuleId::HyprSplash,
                ModuleId::HyprActiveWindow,
                ModuleId::HyprCursor,
                ModuleId::HyprWorkspaceHistory,
                ModuleId::HyprWorkspaces,
                ModuleId::HyprMonitors,
                ModuleId::HyprConfig,
                ModuleId::HyprRules,
                ModuleId::HyprDevices,
                ModuleId::HyprBinds,
                ModuleId::HyprShortcuts,
                ModuleId::HyprLayouts,
                ModuleId::HyprLayers,
                ModuleId::Theme,
                ModuleId::Wallpaper,
                ModuleId::HyprEcosystem,
            ],
        );
    }
}
