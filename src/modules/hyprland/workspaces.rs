use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::providers::hyprland::HyprWorkspace;

use super::{failure_summary, push_failure};

pub struct HyprWorkspacesModule;
pub struct HyprWorkspaceHistoryModule;
impl Module for HyprWorkspacesModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprWorkspaces
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        let active_workspace = ctx.providers.hyprland.active_workspace();
        let workspaces = ctx.providers.hyprland.workspaces();

        match (active_workspace, workspaces) {
            (Ok(active_workspace), Ok(mut workspaces)) => {
                if workspaces.is_empty() {
                    return ModuleOutput::unavailable(self.id(), "hyprctl reported no workspaces");
                }

                workspaces.sort_by_key(workspace_sort_key);
                let compact = config.option_bool("compact", true);
                if compact {
                    return ModuleOutput::ok(
                        self.id(),
                        vec![OutputRow::new(
                            "Workspaces",
                            summarize_workspaces(&workspaces, active_workspace.id),
                        )],
                        json!({ "active_workspace": active_workspace, "workspaces": workspaces }),
                    );
                }

                let mut rows = vec![OutputRow::new("Workspaces", workspaces.len().to_string())];
                for workspace in &workspaces {
                    rows.push(OutputRow::new(
                        workspace_label(workspace, active_workspace.id),
                        workspace_details(workspace),
                    ));
                }

                ModuleOutput::ok(
                    self.id(),
                    rows,
                    json!({ "active_workspace": active_workspace, "workspaces": workspaces }),
                )
            }
            (Err(error), Ok(_)) | (Ok(_), Err(error)) => {
                ModuleOutput::unavailable(self.id(), error.to_string())
            }
            (Err(left), Err(right)) => {
                let mut failures = Vec::new();
                push_failure(&mut failures, left);
                push_failure(&mut failures, right);
                ModuleOutput::unavailable(self.id(), failure_summary(&failures))
            }
        }
    }
}

impl Module for HyprWorkspaceHistoryModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprWorkspaceHistory
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match (
            ctx.providers.hyprland.active_workspace(),
            ctx.providers.hyprland.workspace_history(),
        ) {
            (Ok(active), Ok(history)) => {
                let row = format!(
                    "{history} ← {active}",
                    history = history,
                    active = active.display_name()
                );
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Hist", row)],
                    json!({ "history": history, "active": active }),
                )
            }
            (Err(_), Err(e)) => ModuleOutput::unavailable(self.id(), e.to_string()),
            (Err(e), _) => ModuleOutput::unavailable(self.id(), e.to_string()),
            (_, Err(e)) => ModuleOutput::unavailable(self.id(), e.to_string()),
        }
    }
}

pub fn summarize_workspaces(workspaces: &[HyprWorkspace], active_id: Option<i32>) -> String {
    workspaces
        .iter()
        .map(|workspace| workspace_compact_token(workspace, active_id))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn workspace_label(workspace: &HyprWorkspace, active_id: Option<i32>) -> String {
    let base = workspace_name(workspace);
    if workspace.id == active_id {
        format!("*{base}")
    } else {
        base
    }
}

pub fn workspace_details(workspace: &HyprWorkspace) -> String {
    let mut parts = Vec::new();

    if let Some(monitor) = workspace
        .monitor
        .as_deref()
        .filter(|monitor| !monitor.is_empty())
    {
        parts.push(monitor.to_string());
    }

    match workspace.windows {
        Some(0) => parts.push("empty".to_string()),
        Some(windows) => {
            let label = if windows == 1 { "window" } else { "windows" };
            parts.push(format!("{windows} {label}"));
        }
        None => {}
    }

    if workspace_is_special(workspace) {
        parts.push("special".to_string());
    }

    if workspace.has_fullscreen.unwrap_or(false) {
        parts.push("fullscreen".to_string());
    }

    if parts.is_empty() {
        return "active".to_string();
    }

    parts.join(", ")
}

pub fn workspace_sort_key(workspace: &HyprWorkspace) -> (i32, String) {
    let order = match workspace.id {
        Some(id) if id >= 0 => id,
        _ => i32::MAX,
    };
    (order, workspace.display_name())
}

pub fn workspace_compact_token(workspace: &HyprWorkspace, active_id: Option<i32>) -> String {
    let mut token = workspace_label(workspace, active_id);

    if workspace.has_fullscreen.unwrap_or(false) {
        token.push('!');
    }
    if let Some(windows) = workspace.windows {
        token.push(':');
        token.push_str(&windows.to_string());
    }

    token
}

pub(super) fn workspace_name(workspace: &HyprWorkspace) -> String {
    let display = workspace.display_name();
    if workspace_is_special(workspace) {
        let name = display.strip_prefix("special:").unwrap_or(display.as_str());
        format!("[{name}]")
    } else {
        display
    }
}

pub(super) fn workspace_is_special(workspace: &HyprWorkspace) -> bool {
    workspace.id.map(|id| id < 0).unwrap_or(false)
        || workspace
            .name
            .as_deref()
            .map(|name| name.starts_with("special:"))
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::hyprland::HyprWorkspace;

    #[test]
    fn marks_active_workspace_in_summary() {
        let workspaces = vec![
            HyprWorkspace {
                id: Some(1),
                name: Some("1".to_string()),
                ..HyprWorkspace::default()
            },
            HyprWorkspace {
                id: Some(2),
                name: Some("2".to_string()),
                windows: Some(1),
                ..HyprWorkspace::default()
            },
        ];

        assert_eq!(summarize_workspaces(&workspaces, Some(2)), "1 *2:1");
        assert_eq!(workspace_label(&workspaces[0], Some(1)), "*1");
    }

    #[test]
    fn formats_workspace_details() {
        let workspace = HyprWorkspace {
            id: Some(1),
            name: Some("1".to_string()),
            monitor: Some("DP-1".to_string()),
            windows: Some(2),
            has_fullscreen: Some(true),
        };

        assert_eq!(workspace_details(&workspace), "DP-1, 2 windows, fullscreen");
    }

    #[test]
    fn formats_special_and_empty_workspaces() {
        let workspace = HyprWorkspace {
            id: Some(-99),
            name: Some("special:music".to_string()),
            monitor: Some("HDMI-A-1".to_string()),
            windows: Some(0),
            ..HyprWorkspace::default()
        };

        assert_eq!(workspace_label(&workspace, Some(1)), "[music]");
        assert_eq!(workspace_compact_token(&workspace, Some(1)), "[music]:0");
        assert_eq!(workspace_details(&workspace), "HDMI-A-1, empty, special");
    }

    #[test]
    fn sorts_special_workspaces_last() {
        let mut workspaces = [
            HyprWorkspace {
                id: Some(-99),
                name: Some("special:music".to_string()),
                ..HyprWorkspace::default()
            },
            HyprWorkspace {
                id: Some(1),
                name: Some("1".to_string()),
                ..HyprWorkspace::default()
            },
        ];

        workspaces.sort_by_key(workspace_sort_key);
        assert_eq!(workspaces[0].display_name(), "1");
        assert_eq!(workspaces[1].display_name(), "special:music");
    }
}
