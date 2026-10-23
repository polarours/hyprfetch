//! Hyprland session modules.
//!
//! Every submodule owns one domain of the compositor state: session
//! overview, workspaces, monitors, rice settings, input devices, keybinds,
//! layer-shell surfaces, config health, workspace rules, global shortcuts,
//! layouts and the desktop ecosystem.

mod binds;
mod config;
mod ecosystem;
mod input;
mod layers;
mod layouts;
mod monitors;
mod rice;
mod rules;
mod session;
mod shortcuts;
mod window;
mod workspaces;

pub use binds::HyprBindsModule;
pub use config::HyprConfigModule;
pub use ecosystem::HyprEcosystemModule;
pub use input::HyprDevicesModule;
pub use layers::HyprLayersModule;
pub use layouts::HyprLayoutsModule;
pub use monitors::HyprMonitorsModule;
pub use rice::HyprRiceModule;
pub use rules::HyprRulesModule;
pub use session::{HyprHealthModule, HyprInstancesModule, HyprSplashModule, HyprlandModule};
pub use shortcuts::HyprShortcutsModule;
pub use window::{HyprActiveWindowModule, HyprCursorModule};
pub use workspaces::{HyprWorkspaceHistoryModule, HyprWorkspacesModule};

fn push_failure(failures: &mut Vec<String>, error: impl ToString) {
    let message = error.to_string();
    if !message.is_empty() && !failures.iter().any(|existing| existing == &message) {
        failures.push(message);
    }
}

fn failure_summary(failures: &[String]) -> String {
    failures.join("; ")
}

fn count_label(count: usize, singular: &str, plural: &str) -> String {
    let label = if count == 1 { singular } else { plural };
    format!("{count} {label}")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use crate::config::{AppearanceConfig, ModuleConfig, RuntimeConfig};
    use crate::model::{ColorMode, ModuleId, ModuleStatus, PaletteName, RenderStyle};
    use crate::modules::{ModuleContext, build_module};
    use crate::providers::Providers;
    use crate::providers::hyprland::MockRunner;

    /// The modules that read from the compositor.
    const HYPRLAND_MODULES: &[ModuleId] = &[
        ModuleId::Hyprland,
        ModuleId::HyprHealth,
        ModuleId::HyprInstances,
        ModuleId::HyprSplash,
        ModuleId::HyprWorkspaces,
        ModuleId::HyprMonitors,
        ModuleId::HyprActiveWindow,
        ModuleId::HyprCursor,
        ModuleId::HyprWorkspaceHistory,
        ModuleId::HyprRice,
        ModuleId::HyprDevices,
        ModuleId::HyprBinds,
        ModuleId::HyprLayers,
        ModuleId::HyprConfig,
        ModuleId::HyprRules,
        ModuleId::HyprShortcuts,
        ModuleId::HyprLayouts,
        ModuleId::HyprEcosystem,
    ];

    fn runtime() -> RuntimeConfig {
        RuntimeConfig {
            appearance: AppearanceConfig {
                style: RenderStyle::Split,
                palette: PaletteName::Hypr,
                color: ColorMode::Never,
                plain: false,
                color_bars: false,
                padding: 1,
                separator: "  ".to_string(),
            },
            logo: crate::logo::LogoPreset::None,
            resolved_logo: crate::logo::LogoPreset::None,
            modules: Vec::new(),
            plugins: Vec::new(),
            json: false,
            diagnostics: false,
            animated: false,
            watch: false,
            watch_interval: 1,
        }
    }

    /// Every module has to stay usable when hyprctl is not there.
    #[test]
    fn every_module_survives_a_missing_hyprctl() {
        let runtime = runtime();
        let providers = Providers::with_hypr_runner(Arc::new(MockRunner {
            payloads: HashMap::new(),
        }));
        let context = ModuleContext {
            runtime: &runtime,
            providers: &providers,
        };

        for id in HYPRLAND_MODULES {
            let module = build_module(*id);
            let output = module.run(&context, &ModuleConfig::from(*id));

            match output.status {
                ModuleStatus::Unavailable => {
                    let message = output.message.unwrap_or_default();
                    assert!(
                        !message.is_empty(),
                        "{id:?} is unavailable without a reason"
                    );
                }
                ModuleStatus::Ok | ModuleStatus::Partial => {
                    assert!(
                        !output.rows.is_empty(),
                        "{id:?} reported {output:?} without rows"
                    );
                }
            }
        }
    }
}
