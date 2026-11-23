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
            Preset::Rice => vec![
                ModuleId::Os,
                ModuleId::Host,
                ModuleId::Kernel,
                ModuleId::Uptime,
                ModuleId::Cpu,
                ModuleId::Gpu,
                ModuleId::Memory,
                ModuleId::Disk,
                ModuleId::Theme,
                ModuleId::Wallpaper,
                ModuleId::Hyprland,
                ModuleId::HyprEcosystem,
                ModuleId::HyprWorkspaces,
                ModuleId::HyprActiveWindow,
                ModuleId::HyprMonitors,
                ModuleId::HyprDevices,
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

    #[test]
    fn rice_and_minimal_are_distinct_subsets() {
        let rice = Preset::Rice.modules();
        let minimal = Preset::Minimal.modules();
        assert!(rice.len() > minimal.len());
        assert!(rice.contains(&ModuleId::Wallpaper));
        assert!(rice.contains(&ModuleId::HyprEcosystem));
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
