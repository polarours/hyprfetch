pub(super) const DEFAULT_CONFIG_TOML: &str = r#"version = 1

# How the output is animated: "split" (the logo skeleton grows first, then
# the session rows light up), "lines" (one row after another), "wave" (the
# logo lights up column by column) or "static" for no animation.
animation = "split"

# Arcade styling is on by default; disable with --no-retro.
retro = true

# Replace the [LABEL] tags with Nerd Font icons (requires a Nerd Font).
icons = false

[appearance]
style = "split"
palette = "arcade"
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
name = "media"

[[modules]]
name = "datetime"

[[modules]]
name = "colorscheme"

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
name = "hypr-workspace-history"
compact = true

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

pub fn default_config_toml() -> &'static str {
    DEFAULT_CONFIG_TOML
}

#[cfg(test)]
mod tests {
    use super::super::AppConfig;
    use super::super::load::validate;
    use super::*;

    use crate::model::ModuleId;

    #[test]
    fn default_config_matches_the_builtin_module_order() {
        // A user who runs --init-config should get the same output as a user
        // who runs with no config at all. The shipped TOML and the built-in
        // default_order must therefore list the same modules.
        use crate::model::ModuleId;

        let config = toml::from_str::<AppConfig>(default_config_toml()).unwrap();
        let shipped: std::collections::BTreeSet<ModuleId> =
            config.modules.iter().map(|module| module.name).collect();
        let builtin: std::collections::BTreeSet<ModuleId> =
            ModuleId::default_order().iter().copied().collect();
        assert_eq!(
            shipped, builtin,
            "shipped config and built-in defaults disagree on modules"
        );
    }

    #[test]
    fn shipped_toml_logo_matches_the_code_default() {
        // --init-config must produce the same look as running with no config:
        // the shipped logo preset has to track LogoConfig::default().
        let config = toml::from_str::<AppConfig>(default_config_toml()).unwrap();
        assert_eq!(
            config.logo,
            crate::config::LogoConfig::default(),
            "shipped [logo] preset disagrees with LogoConfig::default()"
        );
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
}
