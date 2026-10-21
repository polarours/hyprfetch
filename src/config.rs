use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use toml::Value;

use crate::cli::CliArgs;
use crate::logo::LogoPreset;
use crate::model::{ColorMode, ModuleId, PaletteName, RenderStyle};

const DEFAULT_CONFIG_TOML: &str = r#"version = 1

[appearance]
style = "split"
palette = "hypr"
color = "auto"
plain = false
color_bars = true
padding = 1
separator = "  "

[logo]
preset = "auto"

[[modules]]
name = "os"

[[modules]]
name = "host"

[[modules]]
name = "kernel"

[[modules]]
name = "uptime"

[[modules]]
name = "packages"

[[modules]]
name = "shell"

[[modules]]
name = "terminal"

[[modules]]
name = "cpu"

[[modules]]
name = "gpu"

[[modules]]
name = "memory"

[[modules]]
name = "swap"

[[modules]]
name = "disk"

[[modules]]
name = "battery"

[[modules]]
name = "locale"

[[modules]]
name = "network"

[[modules]]
name = "temperature"

[[modules]]
name = "audio"

[[modules]]
name = "datetime"

[[modules]]
name = "hyprland"
# Set true to always show HYPRLAND_INSTANCE_SIGNATURE/socket runtime details.
show_runtime = false

[[modules]]
name = "hypr-health"

[[modules]]
name = "hypr-instances"
compact = true

[[modules]]
name = "hypr-splash"

[[modules]]
name = "hypr-active-window"
compact = true

[[modules]]
name = "hypr-cursor"

[[modules]]
name = "hypr-workspaces"
compact = true

[[modules]]
name = "hypr-monitors"
compact = true

[[modules]]
name = "hypr-config"
compact = true

[[modules]]
name = "hypr-rules"
compact = true

[[modules]]
name = "hypr-devices"
compact = true

[[modules]]
name = "hypr-binds"
compact = true

[[modules]]
name = "hypr-shortcuts"
compact = true

[[modules]]
name = "hypr-layouts"
compact = true

[[modules]]
name = "hypr-layers"
compact = true

[[modules]]
name = "hypr-rice"
compact = true

[[modules]]
name = "theme"
compact = false
show_font = true

[[modules]]
name = "wallpaper"

[[modules]]
name = "hypr-ecosystem"
compact = true

# Command plugins are optional. Uncomment and customize when needed.
# [[plugins]]
# name = "music"
# command = "printf"
# args = ["Track=Lo-fi radio\n"]
# timeout_ms = 1000
"#;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub version: u32,
    pub appearance: AppearanceConfig,
    pub logo: LogoConfig,
    pub modules: Vec<ModuleConfig>,
    pub plugins: Vec<PluginConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            appearance: AppearanceConfig::default(),
            logo: LogoConfig::default(),
            modules: ModuleId::default_order()
                .iter()
                .copied()
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
            palette: PaletteName::Hypr,
            color: ColorMode::Auto,
            plain: false,
            color_bars: true,
            padding: 1,
            separator: "  ".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LogoConfig {
    pub preset: LogoPreset,
}

impl Default for LogoConfig {
    fn default() -> Self {
        Self {
            preset: LogoPreset::Auto,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ModuleConfig {
    pub name: ModuleId,
    pub enabled: bool,
    #[serde(flatten)]
    pub options: BTreeMap<String, Value>,
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
    pub modules: Vec<ModuleConfig>,
    pub plugins: Vec<PluginConfig>,
    pub json: bool,
    pub diagnostics: bool,
    pub animated: bool,
    pub watch: bool,
    pub watch_interval: u64,
}

impl RuntimeConfig {
    pub fn is_plain(&self) -> bool {
        self.appearance.plain
    }
}

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub path: Option<PathBuf>,
    pub config: AppConfig,
}

pub fn load(cli: &CliArgs) -> Result<LoadedConfig> {
    if cli.no_config {
        return Ok(LoadedConfig {
            path: None,
            config: AppConfig::default(),
        });
    }

    if let Some(path) = cli.config.as_deref() {
        let config = load_from_path(path)?;
        return Ok(LoadedConfig {
            path: Some(path.to_path_buf()),
            config,
        });
    }

    if let Some(path) = discover_default_path()
        && path.exists()
    {
        let config = load_from_path(&path)?;
        return Ok(LoadedConfig {
            path: Some(path),
            config,
        });
    }

    Ok(LoadedConfig {
        path: None,
        config: AppConfig::default(),
    })
}

pub fn default_config_toml() -> &'static str {
    DEFAULT_CONFIG_TOML
}

pub fn init_config(cli: &CliArgs) -> Result<PathBuf> {
    let path =
        cli.config.clone().or_else(discover_default_path).context(
            "could not determine config path; set HOME, XDG_CONFIG_HOME, or use --config",
        )?;

    if path.exists() {
        bail!("config file already exists at {}", path.display());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }

    fs::write(&path, default_config_toml())
        .with_context(|| format!("failed to write config file at {}", path.display()))?;
    Ok(path)
}

fn load_from_path(path: &Path) -> Result<AppConfig> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file at {}", path.display()))?;
    let config = toml::from_str::<AppConfig>(&raw)
        .with_context(|| format!("failed to parse config file at {}", path.display()))?;
    validate(&config)?;
    Ok(config)
}

fn validate(config: &AppConfig) -> Result<()> {
    if config.version != 1 {
        bail!("unsupported config version {}; expected 1", config.version);
    }
    if config.modules.is_empty() && config.plugins.is_empty() {
        bail!("config must include at least one module or plugin");
    }
    for plugin in &config.plugins {
        if !plugin.enabled {
            continue;
        }
        if plugin.name.trim().is_empty() {
            bail!("plugin name cannot be empty");
        }
        if plugin.command.trim().is_empty() {
            bail!("plugin '{}' command cannot be empty", plugin.name);
        }
    }
    Ok(())
}

pub fn merge(cli: &CliArgs, config: AppConfig) -> RuntimeConfig {
    let mut appearance = config.appearance.clone();
    if cli.share {
        appearance.style = RenderStyle::Split;
        appearance.color_bars = true;
        appearance.plain = false;
        if cli.theme.is_none() {
            appearance.palette = PaletteName::Hypr;
        }
    }
    if cli.plain {
        appearance.plain = true;
    }
    if cli.color != ColorMode::Auto {
        appearance.color = cli.color;
    }
    if let Some(theme) = cli.theme {
        appearance.palette = theme;
    }
    if let Some(style) = cli.style {
        appearance.style = style;
    }
    if appearance.plain {
        appearance.color = ColorMode::Never;
    }

    let mut modules = if cli.share && cli.modules.is_none() {
        SHARE_PRESET_MODULES
            .iter()
            .copied()
            .map(ModuleConfig::from)
            .collect()
    } else if let Some(overrides) = &cli.modules {
        overrides.iter().copied().map(ModuleConfig::from).collect()
    } else {
        config.modules
    };
    modules.retain(|module| module.enabled);
    let mut plugins = config.plugins;
    plugins.retain(|plugin| plugin.enabled);

    if modules.is_empty() && plugins.is_empty() {
        modules = ModuleId::default_order()
            .iter()
            .copied()
            .map(ModuleConfig::from)
            .collect();
    }

    let logo = if appearance.plain {
        LogoPreset::None
    } else {
        cli.logo.unwrap_or(config.logo.preset)
    };

    RuntimeConfig {
        appearance,
        logo,
        resolved_logo: logo,
        modules,
        plugins,
        json: cli.json,
        diagnostics: cli.diagnostics,
        animated: cli.animated,
        watch: cli.watch,
        watch_interval: cli.watch_interval,
    }
}

const SHARE_PRESET_MODULES: &[ModuleId] = &[
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
    ModuleId::Disk,
    ModuleId::Network,
    ModuleId::Audio,
    ModuleId::Theme,
];

fn discover_default_path() -> Option<PathBuf> {
    if let Some(xdg_config_home) = env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg_config_home).join("hyprfetch/config.toml"));
    }

    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|path| path.join(".config/hyprfetch/config.toml"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{
        AppConfig, AppearanceConfig, ModuleConfig, default_config_toml, load_from_path, merge,
        validate,
    };
    use crate::cli::CliArgs;
    use crate::logo::LogoPreset;
    use crate::model::{ColorMode, ModuleId, PaletteName};

    #[test]
    fn loads_valid_toml_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
version = 1

[appearance]
style = "split"
palette = "mono"
color = "auto"
plain = false
padding = 2
separator = " | "

[logo]
preset = "minimal"

[[modules]]
name = "os"

[[modules]]
name = "hyprland"
"#,
        )
        .unwrap();

        let config = load_from_path(&path).unwrap();
        assert_eq!(config.version, 1);
        assert_eq!(config.appearance.palette, PaletteName::Mono);
        assert_eq!(config.logo.preset, LogoPreset::Minimal);
        assert_eq!(config.modules.len(), 2);
    }

    #[test]
    fn default_config_toml_is_valid() {
        let config = toml::from_str::<AppConfig>(default_config_toml()).unwrap();
        validate(&config).unwrap();
        assert_eq!(config.version, 1);
        assert!(
            config
                .modules
                .iter()
                .any(|module| module.name == ModuleId::Hyprland)
        );
    }

    #[test]
    fn cli_plain_mode_overrides_config() {
        let cli = CliArgs {
            config: None,
            init_config: false,
            print_config: false,
            no_config: false,
            plain: true,
            share: false,
            animated: false,
            watch: false,
            watch_interval: 1,
            color: ColorMode::Always,
            modules: Some(vec![ModuleId::Os]),
            theme: None,
            style: None,
            logo: Some(LogoPreset::Hypr),
            json: false,
            diagnostics: false,
            list_logos: false,
            list_modules: false,
            list_themes: false,
        };
        let runtime = merge(
            &cli,
            AppConfig {
                version: 1,
                appearance: AppearanceConfig::default(),
                logo: super::LogoConfig::default(),
                modules: vec![ModuleConfig::from(ModuleId::Kernel)],
                plugins: Vec::new(),
            },
        );
        assert!(runtime.is_plain());
        assert_eq!(runtime.appearance.color, ColorMode::Never);
        assert_eq!(runtime.logo, LogoPreset::None);
        assert_eq!(runtime.modules.len(), 1);
        assert_eq!(runtime.modules[0].name, ModuleId::Os);
    }
}
