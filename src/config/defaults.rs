pub(super) const DEFAULT_CONFIG_TOML: &str = r#"version = 1

# How the output is animated: "split" (the logo skeleton grows first, then
# the session rows light up), "lines" (one row after another), "wave" (the
# logo lights up column by column) or "static" for no animation.
animation = "split"

# Arcade styling is on by default; disable with --no-retro.
retro = true

# Nerd Font glyphs in front of every label. On by default; turn it off with
# icons = false or --no-icons if the terminal font lacks the private-use glyphs.
icons = true

# Draw a PNG as the logo and place the rows beside it. Needs the kitty graphics
# protocol (kitty, ghostty, WezTerm); elsewhere the text logo is used. Setting
# this turns the arcade view off, because a picture and a HUD cannot share the
# same space.
# logo_image = "~/pictures/logo.png"
# Maximum width in cells; 0 derives it from the number of rows.
# logo_image_cols = 0

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
name = "bios"

[[modules]]
name = "board"

[[modules]]
name = "kernel"

[[modules]]
name = "init-system"

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
name = "disk"

[[modules]]
name = "battery"

[[modules]]
name = "network"

[[modules]]
name = "datetime"

[[modules]]
name = "hyprland"

[[modules]]
name = "hypr-workspaces"

[[modules]]
name = "theme"

[[modules]]
name = "wallpaper"

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
        // A user who runs --init-config must get the same output as a user who
        // runs with no config at all, so the shipped TOML and the built-in
        // default have to agree — and every shipped module has to exist in the
        // full order, or the file would name something the binary cannot build.
        use crate::config::Preset;
        use crate::model::ModuleId;

        let config = toml::from_str::<AppConfig>(default_config_toml()).unwrap();
        let shipped: Vec<ModuleId> = config.modules.iter().map(|module| module.name).collect();
        assert_eq!(
            shipped,
            Preset::Rice.modules(),
            "shipped config and built-in defaults disagree on modules"
        );

        let full: std::collections::BTreeSet<ModuleId> =
            ModuleId::default_order().iter().copied().collect();
        for id in &shipped {
            assert!(full.contains(id), "shipped module {id} is unknown");
        }
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
