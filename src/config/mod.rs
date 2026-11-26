//! The configuration of a run.
//!
//! The types live here, the shipped defaults in defaults.rs, reading
//! the file in load.rs and combining it with the command line in
//! merge.rs.

use std::collections::BTreeMap;

use serde::Deserialize;
use toml::Value;

use crate::logo::LogoPreset;
use crate::model::{ColorMode, ModuleId, PaletteName, RenderStyle};

pub mod defaults;
pub mod load;
pub mod merge;
pub mod presets;

pub use defaults::default_config_toml;
pub use load::{LoadedConfig, init_config, load, load_theme};
pub use merge::merge;
pub use presets::Preset;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub version: u32,
    pub appearance: AppearanceConfig,
    pub logo: LogoConfig,
    /// How the output is animated: "split", "lines", "wave" or "static".
    pub animation: Option<crate::animation::RevealStyle>,
    /// Replace identifying values in the output (for public screenshots).
    pub redact: bool,
    /// Skip expensive probes for a near-instant render.
    pub fast: bool,
    /// Render with retro arcade styling.
    pub retro: bool,
    /// Replace [LABEL] tags with Nerd Font icons.
    pub icons: bool,
    /// Artwork loaded from --logo-file (not serialized to config files).
    #[serde(skip)]
    pub custom_logo: Option<String>,
    /// A PNG logo drawn with the kitty graphics protocol.
    pub logo_image: Option<String>,
    /// How many cells wide it is drawn.
    pub logo_image_cols: u32,
    pub modules: Vec<ModuleConfig>,
    pub plugins: Vec<PluginConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            appearance: AppearanceConfig::default(),
            logo: LogoConfig::default(),
            animation: None,
            fast: false,
            redact: false,
            retro: true,
            icons: true,
            custom_logo: None,
            logo_image: None,
            logo_image_cols: 0,
            // The default is the curated screenshot set; every other module is
            // reachable through --preset all or the config file.
            modules: crate::config::Preset::Rice
                .modules()
                .into_iter()
                .map(ModuleConfig::from)
                .collect(),
            plugins: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppearanceConfig {
    pub style: RenderStyle,
    pub palette: PaletteName,
    pub color: ColorMode,
    pub plain: bool,
    pub color_bars: bool,
    pub padding: usize,
    pub separator: String,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            style: RenderStyle::Split,
            palette: PaletteName::Arcade,
            color: ColorMode::Auto,
            plain: false,
            color_bars: true,
            padding: 1,
            separator: "  ".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LogoConfig {
    pub preset: LogoPreset,
}

impl Default for LogoConfig {
    fn default() -> Self {
        Self {
            // Distribution detection: the logo always matches the running
            // distro. `hypr-pixel` stays selectable explicitly.
            preset: LogoPreset::Auto,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModuleConfig {
    pub name: ModuleId,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(flatten)]
    pub options: BTreeMap<String, Value>,
}

fn default_enabled() -> bool {
    true
}

impl From<ModuleId> for ModuleConfig {
    fn from(name: ModuleId) -> Self {
        Self {
            name,
            enabled: true,
            options: BTreeMap::new(),
        }
    }
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self::from(ModuleId::Os)
    }
}

impl ModuleConfig {
    pub fn option_bool(&self, key: &str, default: bool) -> bool {
        self.options
            .get(key)
            .and_then(Value::as_bool)
            .unwrap_or(default)
    }

    pub fn option_string(&self, key: &str) -> Option<String> {
        self.options
            .get(key)
            .and_then(Value::as_str)
            .map(ToString::to_string)
    }

    pub fn option_string_map(&self, key: &str) -> BTreeMap<String, String> {
        self.options
            .get(key)
            .and_then(Value::as_table)
            .map(|table| {
                table
                    .iter()
                    .filter_map(|(key, value)| {
                        value
                            .as_str()
                            .map(|value| (key.to_string(), value.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A list of strings, used for options such as the mount points the disk
    /// module should report.
    pub fn option_string_list(&self, key: &str) -> Vec<String> {
        self.options
            .get(key)
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str().map(ToString::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PluginConfig {
    pub name: String,
    pub enabled: bool,
    pub command: String,
    pub args: Vec<String>,
    pub label: Option<String>,
    pub timeout_ms: u64,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            enabled: true,
            command: String::new(),
            args: Vec::new(),
            label: None,
            timeout_ms: 1_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub appearance: AppearanceConfig,
    pub logo: LogoPreset,
    pub resolved_logo: LogoPreset,
    /// Artwork loaded from --logo-file, rendered instead of a built-in preset.
    pub custom_logo: Option<String>,
    /// A PNG drawn with the kitty graphics protocol instead of any text logo.
    pub logo_image: Option<String>,
    /// How many cells wide that image should be.
    pub logo_image_cols: u32,
    pub modules: Vec<ModuleConfig>,
    pub plugins: Vec<PluginConfig>,
    pub json: bool,
    pub diagnostics: bool,
    pub doctor: bool,
    pub retro: bool,
    pub icons: bool,
    pub animation: crate::animation::RevealStyle,
    pub fast: bool,
    /// Scrub identifying values before the output is printed.
    pub redact: bool,
    pub watch: bool,
    pub watch_interval: u64,
}

impl RuntimeConfig {
    pub fn is_plain(&self) -> bool {
        self.appearance.plain
    }
}
