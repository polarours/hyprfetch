//! Built-in presets: ready-made module sets and looks, selectable with
//! `--preset <name>`. A preset is applied to the config before CLI flags are
//! merged, so explicit flags (`--theme`, `--modules`, …) still win.

use clap::ValueEnum;

use crate::config::AppConfig;
use crate::model::ModuleId;

/// Built-in presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Preset {
    /// Every module the binary can report, in the default order.
    All,
    /// Rice-screenshot focused: identity, hardware, theme and live session.
    Rice,
    /// A short, dense core: identity, CPU, memory, compositor and theme.
    Minimal,
}

impl Preset {
    /// The ordered module set this preset selects.
    pub fn modules(self) -> Vec<ModuleId> {
        match self {
            Preset::All => ModuleId::default_order().to_vec(),
            // Kept in step with the shipped default config: a screenshot reads
            // in one glance, and the diagnostics are one --preset all away.
            Preset::Rice => vec![
                ModuleId::Os,
                ModuleId::Host,
                ModuleId::Bios,
                ModuleId::Board,
                ModuleId::Chassis,
                ModuleId::Kernel,
                ModuleId::InitSystem,
                ModuleId::Uptime,
                ModuleId::Packages,
                ModuleId::Shell,
                ModuleId::Terminal,
                ModuleId::Cpu,
                ModuleId::Gpu,
                ModuleId::Memory,
                ModuleId::Disk,
                ModuleId::Battery,
                ModuleId::PowerAdapter,
                ModuleId::Network,
                ModuleId::Datetime,
                ModuleId::Hyprland,
                ModuleId::HyprWorkspaces,
                ModuleId::Theme,
                ModuleId::Wallpaper,
            ],
            Preset::Minimal => vec![
                ModuleId::Os,
                ModuleId::Kernel,
                ModuleId::Uptime,
                ModuleId::Cpu,
                ModuleId::Memory,
                ModuleId::Hyprland,
                ModuleId::Theme,
            ],
        }
    }

    /// Apply this preset to a config: replace the module set, keep the user's
    /// per-module options where present, and use the preset's recommended look.
    pub fn apply(self, config: &mut AppConfig) {
        let existing = std::mem::take(&mut config.modules);
        config.modules = self
            .modules()
            .into_iter()
            .map(|id| {
                existing
                    .iter()
                    .find(|m| m.name == id)
                    .cloned()
                    .unwrap_or_else(|| id.into())
            })
            .collect();

        match self {
            Preset::Rice => {
                config.appearance.style = crate::model::RenderStyle::Split;
                config.appearance.color_bars = true;
            }
            Preset::Minimal => {
                config.appearance.style = crate::model::RenderStyle::Minimal;
            }
            Preset::All => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_preset_uses_the_full_default_order() {
        assert_eq!(Preset::All.modules(), ModuleId::default_order());
    }

    /// The shipped default and the Rice preset must not drift apart.
    #[test]
    fn the_default_config_is_the_rice_preset() {
        let config = AppConfig::default();
        let configured: Vec<ModuleId> = config.modules.iter().map(|m| m.name).collect();
        assert_eq!(configured, Preset::Rice.modules());
    }

    #[test]
    fn rice_stays_short_enough_for_one_screen() {
        assert!(
            Preset::Rice.modules().len() <= 24,
            "the screenshot preset must stay glanceable"
        );
    }

    #[test]
    fn rice_and_minimal_are_distinct_subsets() {
        let rice = Preset::Rice.modules();
        let minimal = Preset::Minimal.modules();
        assert!(rice.len() > minimal.len());
        assert!(rice.contains(&ModuleId::Wallpaper));
        assert!(rice.contains(&ModuleId::HyprWorkspaces));
        // The diagnostics belong to the full preset, not the screenshot one.
        assert!(!rice.contains(&ModuleId::HyprRules));
        assert!(!rice.contains(&ModuleId::HyprBinds));
        assert!(!minimal.contains(&ModuleId::Wallpaper));
        assert!(minimal.contains(&ModuleId::Cpu));
    }

    #[test]
    fn applying_a_preset_keeps_configured_module_options() {
        let mut config = AppConfig::default();
        // give OS a custom label
        if let Some(os) = config.modules.iter_mut().find(|m| m.name == ModuleId::Os) {
            os.options.insert(
                "label".to_string(),
                toml::Value::String("Distro".to_string()),
            );
        }
        Preset::Rice.apply(&mut config);
        let os = config
            .modules
            .iter()
            .find(|m| m.name == ModuleId::Os)
            .unwrap();
        assert_eq!(os.option_string("label").as_deref(), Some("Distro"));
        assert!(config.modules.iter().any(|m| m.name == ModuleId::Wallpaper));
    }
}
