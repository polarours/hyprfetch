use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::cli::CliArgs;

use super::AppConfig;
use super::defaults::default_config_toml;

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

pub(super) fn validate(config: &AppConfig) -> Result<()> {
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
    use super::*;

    use crate::logo::LogoPreset;
    use crate::model::PaletteName;
    use std::fs;
    use tempfile::tempdir;

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
}
