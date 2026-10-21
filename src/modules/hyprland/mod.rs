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
