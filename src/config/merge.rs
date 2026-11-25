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
    let had_configured_modules = !modules.is_empty();
    let had_configured_plugins = !config.plugins.is_empty();
    modules.retain(|module| module.enabled);
    let mut plugins = config.plugins;
    plugins.retain(|plugin| plugin.enabled);

    // Only fall back to the defaults when nothing was configured at all. If
    // the user listed modules and disabled every one of them, that is an
    // explicit choice and must not be silently overridden.
    if !had_configured_modules && !had_configured_plugins {
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
        custom_logo: config.custom_logo,
        modules,
        plugins,
        json: cli.json,
        diagnostics: cli.diagnostics,
        doctor: cli.doctor,
        retro: (cli.retro || config.retro) && !cli.no_retro,
        icons: cli.icons || config.icons,
        // The command line wins over the config file; the default tells the
        // two-phase story. The renderer gates the rest off where animation
        // cannot run.
        animation: cli.animation.or(config.animation).unwrap_or_default(),
        fast: cli.fast || config.fast,
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
    fn an_explicitly_empty_module_set_stays_empty() {
        // When the user disables every module and has no plugins, that is an
        // explicit choice. The old fallback re-enabled the whole default set
        // and silently ignored all the enabled=false entries.
        let cli = CliArgs {
            config: None,
            preset: None,
            theme_file: None,
            logo_file: None,
            doctor: false,
            retro: false,
            no_retro: false,
            icons: false,
            init_config: false,
            print_config: false,
            no_config: false,
            plain: false,
            fast: false,
            animation: None,
            share: false,
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
        let mut os = ModuleConfig::from(ModuleId::Os);
        os.enabled = false;

        let runtime = merge(
            &cli,
            AppConfig {
                version: 1,
                appearance: super::super::AppearanceConfig::default(),
                logo: super::super::LogoConfig::default(),
                animation: None,
                fast: false,
                retro: false,
                icons: false,

                custom_logo: None,
                modules: vec![os],
                plugins: Vec::new(),
            },
        );

        assert!(
            runtime.modules.is_empty(),
            "explicitly disabled modules must stay disabled"
        );
    }

    #[test]
    fn fast_mode_is_enabled_by_cli_or_config() {
        let mut cli = CliArgs {
            config: None,
            preset: None,
            theme_file: None,
            logo_file: None,
            doctor: false,
            retro: false,
            no_retro: false,
            icons: false,
            init_config: false,
            print_config: false,
            no_config: false,
            plain: false,
            fast: false,
            animation: None,
            share: false,
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
        let config = AppConfig::default();

        assert!(
            !merge(&cli, config.clone()).fast,
            "default must be normal mode"
        );
        cli.fast = true;
        assert!(
            merge(&cli, config.clone()).fast,
            "cli --fast must enable it"
        );
        cli.fast = false;
        let mut config_fast = config;
        config_fast.fast = true;
        assert!(
            merge(&cli, config_fast).fast,
            "config fast = true must enable it"
        );
    }

    #[test]
    fn cli_plain_mode_overrides_config() {
        let cli = CliArgs {
            config: None,
            preset: None,
            theme_file: None,
            logo_file: None,
            doctor: false,
            retro: false,
            no_retro: false,
            icons: false,
            init_config: false,
            print_config: false,
            no_config: false,
            plain: true,
            fast: false,
            animation: None,
            share: false,
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
                animation: None,
                fast: false,
                retro: false,
                icons: false,

                custom_logo: None,
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
            preset: None,
            theme_file: None,
            logo_file: None,
            doctor: false,
            retro: false,
            no_retro: false,
            icons: false,
            init_config: false,
            print_config: false,
            no_config: false,
            plain: false,
            fast: false,
            animation: None,
            share: false,
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
                animation: None,
                fast: false,
                retro: false,
                icons: false,

                custom_logo: None,
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
            preset: None,
            theme_file: None,
            logo_file: None,
            doctor: false,
            retro: false,
            no_retro: false,
            icons: false,
            init_config: false,
            print_config: false,
            no_config: false,
            plain: false,
            fast: false,
            animation: None,
            share: false,
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
                animation: None,
                fast: false,
                retro: false,
                icons: false,

                custom_logo: None,
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
            preset: None,
            theme_file: None,
            logo_file: None,
            doctor: false,
            retro: false,
            no_retro: false,
            icons: false,
            init_config: false,
            print_config: false,
            no_config: false,
            plain: false,
            fast: false,
            animation: None,
            share: true,
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
                animation: None,
                fast: false,
                retro: false,
                icons: false,

                custom_logo: None,
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

    #[test]
    fn the_share_preset_shows_the_desktop_not_only_the_machine() {
        let cli = CliArgs {
            config: None,
            preset: None,
            theme_file: None,
            logo_file: None,
            doctor: false,
            retro: false,
            no_retro: false,
            icons: false,
            init_config: false,
            print_config: false,
            no_config: false,
            plain: false,
            fast: false,
            animation: None,
            share: true,
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
            AppConfig {
                version: 1,
                appearance: super::super::AppearanceConfig::default(),
                logo: super::super::LogoConfig::default(),
                animation: None,
                fast: false,
                retro: false,
                icons: false,

                custom_logo: None,
                modules: Vec::new(),
                plugins: Vec::new(),
            },
        );

        let names: Vec<ModuleId> = runtime.modules.iter().map(|module| module.name).collect();
        for expected in [
            ModuleId::Theme,
            ModuleId::Wallpaper,
            ModuleId::HyprEcosystem,
            ModuleId::HyprActiveWindow,
            ModuleId::HyprWorkspaces,
            ModuleId::HyprMonitors,
        ] {
            assert!(
                names.contains(&expected),
                "the share preset is missing {expected:?}"
            );
        }
    }
}
