use crate::cli::CliArgs;
use crate::logo::LogoPreset;
use crate::model::{ColorMode, ModuleId, PaletteName, RenderStyle};

use super::{AppConfig, ModuleConfig, RuntimeConfig};

/// The configuration of a module, keeping the options the user configured
/// for it. Selecting a module narrows the output, it does not reset the
/// configuration.
fn configured_module(configured: &[ModuleConfig], id: ModuleId) -> ModuleConfig {
    configured
        .iter()
        .find(|module| module.name == id)
        .cloned()
        .map(|mut module| {
            module.enabled = true;
            module
        })
        .unwrap_or_else(|| ModuleConfig::from(id))
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
            .map(|id| configured_module(&config.modules, id))
            .collect()
    } else if let Some(overrides) = &cli.modules {
        overrides
            .iter()
            .copied()
            .map(|id| configured_module(&config.modules, id))
            .collect()
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

/// The modules the share preset shows.
///
/// A rice screenshot is about the desktop, so the preset leads with the
/// theme and the ecosystem and closes with the live session state, not with
/// the machine alone.
const SHARE_PRESET_MODULES: &[ModuleId] = &[
    // the machine
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
    // the rice
    ModuleId::Theme,
    ModuleId::Wallpaper,
    ModuleId::HyprEcosystem,
    // the session
    ModuleId::Hyprland,
    ModuleId::HyprActiveWindow,
    ModuleId::HyprWorkspaces,
    ModuleId::HyprMonitors,
    ModuleId::HyprDevices,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::CliArgs;
    use crate::logo::LogoPreset;
    use crate::model::{ColorMode, ModuleId};

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
            super::super::AppConfig {
                version: 1,
                appearance: super::super::AppearanceConfig::default(),
                logo: super::super::LogoConfig::default(),
                modules: vec![super::super::ModuleConfig::from(ModuleId::Kernel)],
                plugins: Vec::new(),
            },
        );
        assert!(runtime.is_plain());
        assert_eq!(runtime.appearance.color, ColorMode::Never);
        assert_eq!(runtime.logo, LogoPreset::None);
        assert_eq!(runtime.modules.len(), 1);
        assert_eq!(runtime.modules[0].name, ModuleId::Os);
    }

    #[test]
    fn cli_selection_keeps_the_configured_options() {
        let mut disk = ModuleConfig::from(ModuleId::Disk);
        disk.options.insert(
            "mounts".to_string(),
            toml::Value::Array(vec![
                toml::Value::String("/".to_string()),
                toml::Value::String("/home".to_string()),
            ]),
        );

        let mut cli = CliArgs {
            config: None,
            init_config: false,
            print_config: false,
            no_config: false,
            plain: false,
            share: false,
            animated: false,
            watch: false,
            watch_interval: 1,
            color: ColorMode::Auto,
            modules: Some(vec![ModuleId::Disk]),
            theme: None,
            style: None,
            logo: None,
            json: false,
            diagnostics: false,
            list_logos: false,
            list_modules: false,
            list_themes: false,
        };
        cli.modules = Some(vec![ModuleId::Disk]);

        let runtime = merge(
            &cli,
            super::super::AppConfig {
                version: 1,
                appearance: super::super::AppearanceConfig::default(),
                logo: super::super::LogoConfig::default(),
                modules: vec![disk],
                plugins: Vec::new(),
            },
        );

        assert_eq!(runtime.modules.len(), 1);
        assert_eq!(runtime.modules[0].name, ModuleId::Disk);
        assert_eq!(
            runtime.modules[0].option_string_list("mounts"),
            vec!["/".to_string(), "/home".to_string()]
        );
    }

    #[test]
    fn a_disabled_module_can_still_be_selected_on_the_command_line() {
        let mut disk = ModuleConfig::from(ModuleId::Disk);
        disk.enabled = false;

        let cli = CliArgs {
            config: None,
            init_config: false,
            print_config: false,
            no_config: false,
            plain: false,
            share: false,
            animated: false,
            watch: false,
            watch_interval: 1,
            color: ColorMode::Auto,
            modules: Some(vec![ModuleId::Disk]),
            theme: None,
            style: None,
            logo: None,
            json: false,
            diagnostics: false,
            list_logos: false,
            list_modules: false,
            list_themes: false,
        };

        let runtime = merge(
            &cli,
            super::super::AppConfig {
                version: 1,
                appearance: super::super::AppearanceConfig::default(),
                logo: super::super::LogoConfig::default(),
                modules: vec![disk],
                plugins: Vec::new(),
            },
        );

        assert_eq!(runtime.modules.len(), 1);
        assert!(runtime.modules[0].enabled);
    }

    #[test]
    fn the_share_preset_keeps_the_configured_options() {
        let mut disk = ModuleConfig::from(ModuleId::Disk);
        disk.options.insert(
            "mounts".to_string(),
            toml::Value::Array(vec![toml::Value::String("/home".to_string())]),
        );

        let cli = CliArgs {
            config: None,
            init_config: false,
            print_config: false,
            no_config: false,
            plain: false,
            share: true,
            animated: false,
            watch: false,
            watch_interval: 1,
            color: ColorMode::Auto,
            modules: None,
            theme: None,
            style: None,
            logo: None,
            json: false,
            diagnostics: false,
            list_logos: false,
            list_modules: false,
            list_themes: false,
        };

        let runtime = merge(
            &cli,
            super::super::AppConfig {
                version: 1,
                appearance: super::super::AppearanceConfig::default(),
                logo: super::super::LogoConfig::default(),
                modules: vec![disk],
                plugins: Vec::new(),
            },
        );

        let disk = runtime
            .modules
            .iter()
            .find(|module| module.name == ModuleId::Disk)
            .expect("the share preset includes the disk module");
        assert_eq!(disk.option_string_list("mounts"), vec!["/home".to_string()]);
    }
}
