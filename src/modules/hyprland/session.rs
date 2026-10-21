use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::providers::hyprland::{HyprInstance, HyprSessionEnv};
use crate::util::truncate_visible;

use super::monitors::focused_monitor_summary;
use super::workspaces::workspace_is_special;
use super::{count_label, failure_summary, push_failure};

pub struct HyprlandModule;
pub struct HyprHealthModule;
pub struct HyprInstancesModule;
pub struct HyprSplashModule;
impl Module for HyprlandModule {
    fn id(&self) -> ModuleId {
        ModuleId::Hyprland
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        let version = ctx.providers.hyprland.version();
        let workspace = ctx.providers.hyprland.active_workspace();
        let clients = ctx.providers.hyprland.clients();
        let monitors = ctx.providers.hyprland.monitors();
        let workspaces = ctx.providers.hyprland.workspaces();
        let session_env = ctx.providers.hyprland.session_env();

        let mut rows = Vec::new();
        let mut data = json!({});
        data["runtime"] = json!(&session_env);
        let mut failures = Vec::new();

        match version {
            Ok(version) => {
                data["version"] = json!(version);
                rows.push(OutputRow::new("Hyprland", version.display_name()));
            }
            Err(error) => push_failure(&mut failures, error),
        }

        match workspace {
            Ok(workspace) => {
                data["active_workspace"] = json!(workspace);
                rows.push(OutputRow::new("Workspace", workspace.display_name()));
            }
            Err(error) => push_failure(&mut failures, error),
        }

        match &clients {
            Ok(clients) => {
                data["client_count"] = json!(clients.len());
                rows.push(OutputRow::new("Clients", clients.len().to_string()));
            }
            Err(error) => push_failure(&mut failures, error),
        }

        match &monitors {
            Ok(monitors) => {
                data["monitors"] = json!(monitors);
                if let Some(summary) = focused_monitor_summary(monitors) {
                    rows.push(OutputRow::new("Focused", summary));
                }
            }
            Err(error) => push_failure(&mut failures, error),
        }

        match (&monitors, &workspaces, &clients) {
            (Ok(monitors), Ok(workspaces), Ok(clients)) => {
                data["workspaces"] = json!(workspaces);
                rows.push(OutputRow::new(
                    "Session",
                    session_overview(monitors, workspaces, clients.len()),
                ));
            }
            (_, Err(error), _) => push_failure(&mut failures, error),
            _ => {}
        }

        let has_hyprctl_rows = !rows.is_empty();
        if (config.option_bool("show_runtime", false)
            || !has_hyprctl_rows && session_env.is_hyprland_like())
            && let Some(summary) = summarize_hypr_runtime(&session_env)
        {
            rows.push(OutputRow::new("Runtime", summary));
        }

        if !failures.is_empty() {
            rows.push(OutputRow::new("Hyprctl", failure_summary(&failures)));
        }

        if rows.is_empty() {
            ModuleOutput::unavailable(self.id(), failure_summary(&failures))
        } else if failures.is_empty() {
            ModuleOutput::ok(self.id(), rows, data)
        } else {
            ModuleOutput::partial(self.id(), rows, data, failure_summary(&failures))
        }
    }
}

impl Module for HyprHealthModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprHealth
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        let session_env = ctx.providers.hyprland.session_env();
        let instances = ctx.providers.hyprland.instances().ok();
        let wallpaper = ctx.providers.theme.wallpaper_backend().ok();
        let ecosystem = ctx.providers.theme.ecosystem_info().ok();

        let mut warnings = Vec::new();
        let mut rows = Vec::new();

        if !session_env.socket_present {
            warnings.push("command socket missing".to_string());
        }
        if !session_env.event_socket_present {
            warnings.push("event socket missing".to_string());
        }

        if let Some(instances) = instances.as_deref() {
            warnings.extend(hypr_instance_warnings(instances));
        } else if session_env.is_hyprland_like() {
            warnings.push("instance metadata unavailable".to_string());
        }

        if wallpaper.is_none() {
            warnings.push("wallpaper backend missing".to_string());
        }

        let ecosystem = ecosystem.unwrap_or_default();
        if ecosystem.bar.is_none() {
            warnings.push("bar missing".to_string());
        }
        if ecosystem.notifications.is_none() {
            warnings.push("notifications missing".to_string());
        }
        if ecosystem.idle.is_none() {
            warnings.push("idle daemon missing".to_string());
        }
        if ecosystem.lock.is_none() {
            warnings.push("lock daemon missing".to_string());
        }

        let summary = if warnings.is_empty() {
            "ready".to_string()
        } else {
            summarize_health_warnings(&warnings)
        };
        rows.push(OutputRow::new("Health", summary));

        let data = json!({
            "runtime": session_env,
            "instances": instances,
            "wallpaper": wallpaper,
            "ecosystem": ecosystem,
            "warnings": warnings,
        });

        let warnings = data["warnings"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect::<Vec<_>>();

        if warnings.is_empty() {
            ModuleOutput::ok(self.id(), rows, data)
        } else {
            ModuleOutput::partial(self.id(), rows, data, failure_summary(&warnings))
        }
    }
}

impl Module for HyprInstancesModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprInstances
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.instances() {
            Ok(instances) => {
                let compact = config.option_bool("compact", true);
                let warnings = hypr_instance_warnings(&instances);
                let rows = if compact {
                    vec![OutputRow::new(
                        "Instances",
                        summarize_hypr_instances(&instances),
                    )]
                } else {
                    let mut rows = vec![OutputRow::new(
                        "Instances",
                        count_label(instances.len(), "instance", "instances"),
                    )];
                    for instance in &instances {
                        rows.push(OutputRow::new(
                            hypr_instance_label(instance),
                            hypr_instance_details(instance),
                        ));
                    }
                    rows
                };

                if warnings.is_empty() {
                    ModuleOutput::ok(self.id(), rows, json!(instances))
                } else {
                    ModuleOutput::partial(
                        self.id(),
                        rows,
                        json!(instances),
                        failure_summary(&warnings),
                    )
                }
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for HyprSplashModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprSplash
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.splash() {
            Ok(splash) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Splash", splash.clone())],
                json!({ "splash": splash }),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

fn summarize_health_warnings(warnings: &[String]) -> String {
    let mut parts = Vec::new();
    if warnings.iter().any(|warning| warning.contains("stale")) {
        parts.push("stale".to_string());
    }
    if warnings.iter().any(|warning| warning.contains("socket")) {
        parts.push("socket issue".to_string());
    }
    if warnings.iter().any(|warning| warning.contains("wallpaper")) {
        parts.push("wallpaper missing".to_string());
    }
    if warnings.iter().any(|warning| warning.contains("bar")) {
        parts.push("bar missing".to_string());
    }
    if warnings
        .iter()
        .any(|warning| warning.contains("notifications"))
    {
        parts.push("notifications missing".to_string());
    }
    if warnings.iter().any(|warning| warning.contains("idle")) {
        parts.push("idle missing".to_string());
    }
    if warnings.iter().any(|warning| warning.contains("lock")) {
        parts.push("lock missing".to_string());
    }

    if parts.is_empty() {
        return count_label(warnings.len(), "warning", "warnings");
    }

    parts.join(", ")
}

fn summarize_hypr_instances(instances: &[HyprInstance]) -> String {
    let mut parts = vec![count_label(instances.len(), "instance", "instances")];

    if let Some(current) = instances.iter().find(|instance| instance.current) {
        parts.push(format!(
            "current:{}",
            truncate_visible(&current.instance, 8)
        ));
        if let Some(pid) = current.pid {
            parts.push(format!("pid:{pid}"));
        }
        if let Some(wl_socket) = current.wl_socket.as_deref() {
            parts.push(format!("wl:{wl_socket}"));
        }
        if !current.process_running {
            parts.push("stale".to_string());
        }
    }

    parts.join(", ")
}

fn hypr_instance_label(instance: &HyprInstance) -> String {
    let prefix = if instance.current { "*" } else { "" };
    format!("{prefix}{}", truncate_visible(&instance.instance, 8))
}

fn hypr_instance_details(instance: &HyprInstance) -> String {
    let mut parts = Vec::new();

    if let Some(pid) = instance.pid {
        parts.push(format!("pid:{pid}"));
    }
    if let Some(wl_socket) = instance.wl_socket.as_deref() {
        parts.push(format!("wl:{wl_socket}"));
    }
    parts.push(format!(
        "socket:{}",
        if instance.socket_present {
            "ok"
        } else {
            "missing"
        }
    ));
    parts.push(format!(
        "events:{}",
        if instance.event_socket_present {
            "ok"
        } else {
            "missing"
        }
    ));
    if !instance.process_running {
        parts.push("stale".to_string());
    }

    parts.join(" ")
}

fn hypr_instance_warnings(instances: &[HyprInstance]) -> Vec<String> {
    let mut warnings = Vec::new();
    let Some(current) = instances.iter().find(|instance| instance.current) else {
        return warnings;
    };

    if !current.process_running {
        warnings.push("current Hyprland instance is stale or its PID is missing".to_string());
    }
    if !current.socket_present {
        warnings.push("current Hyprland command socket is missing".to_string());
    }
    if !current.event_socket_present {
        warnings.push("current Hyprland event socket is missing".to_string());
    }

    warnings
}

fn summarize_hypr_runtime(session: &HyprSessionEnv) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(signature) = session.instance_signature.as_deref() {
        parts.push(format!("sig:{}", truncate_visible(signature, 8)));
    } else if session.is_hyprland_like() {
        parts.push("sig:missing".to_string());
    }

    if session.socket_path.is_some() {
        parts.push(format!(
            "socket:{}",
            if session.socket_present {
                "ok"
            } else {
                "missing"
            }
        ));
    }

    if session.event_socket_path.is_some() {
        parts.push(format!(
            "events:{}",
            if session.event_socket_present {
                "ok"
            } else {
                "missing"
            }
        ));
    }

    if let Some(display) = session.wayland_display.as_deref() {
        parts.push(format!("display:{display}"));
    }

    if parts.is_empty() {
        return session
            .is_hyprland_like()
            .then(|| "hyprland env detected".to_string());
    }

    Some(parts.join(" "))
}

pub(super) fn session_overview(
    monitors: &[crate::providers::hyprland::HyprMonitor],
    workspaces: &[crate::providers::hyprland::HyprWorkspace],
    _client_count: usize,
) -> String {
    let total_workspaces = workspaces.len();
    let special_count = workspaces
        .iter()
        .filter(|workspace| workspace_is_special(workspace))
        .count();
    let fullscreen = workspaces
        .iter()
        .any(|workspace| workspace.has_fullscreen.unwrap_or(false));
    let total_windows: i32 = workspaces
        .iter()
        .filter_map(|workspace| workspace.windows)
        .sum();

    let mut summary = format!(
        "{} outputs, {} ws, {} wins",
        monitors.len(),
        total_workspaces,
        total_windows
    );

    if special_count > 0 {
        summary.push_str(&format!(" ({} special)", special_count));
    }

    if fullscreen {
        summary.push_str(", fullscreen");
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::hyprland::{HyprInstance, HyprSessionEnv};

    #[test]
    fn summarizes_hypr_runtime_status() {
        let session = HyprSessionEnv {
            instance_signature: Some("abcdef12".to_string()),
            wayland_display: Some("wayland-1".to_string()),
            socket_path: Some("/run/user/1000/hypr/abcdef12/.socket.sock".to_string()),
            event_socket_path: Some("/run/user/1000/hypr/abcdef12/.socket2.sock".to_string()),
            socket_present: true,
            event_socket_present: false,
            ..HyprSessionEnv::default()
        };

        assert_eq!(
            summarize_hypr_runtime(&session).as_deref(),
            Some("sig:abcdef12 socket:ok events:missing display:wayland-1")
        );
    }

    #[test]
    fn summarizes_missing_hypr_runtime_signature() {
        let session = HyprSessionEnv {
            current_desktop: Some("Hyprland".to_string()),
            wayland_display: Some("wayland-1".to_string()),
            ..HyprSessionEnv::default()
        };

        assert_eq!(
            summarize_hypr_runtime(&session).as_deref(),
            Some("sig:missing display:wayland-1")
        );
    }

    #[test]
    fn summarizes_hypr_instances() {
        let instances = vec![
            HyprInstance {
                instance: "abcdef12_1777474148_1543042573".to_string(),
                pid: Some(6089),
                wl_socket: Some("wayland-1".to_string()),
                current: true,
                socket_present: true,
                event_socket_present: true,
                process_running: true,
                ..HyprInstance::default()
            },
            HyprInstance {
                instance: "stale_1777470000_1".to_string(),
                process_running: false,
                ..HyprInstance::default()
            },
        ];

        assert_eq!(
            summarize_hypr_instances(&instances),
            "2 instances, current:abcdef1…, pid:6089, wl:wayland-1"
        );

        let stale_current = vec![HyprInstance {
            instance: "stale".to_string(),
            current: true,
            process_running: false,
            ..HyprInstance::default()
        }];
        assert_eq!(
            summarize_hypr_instances(&stale_current),
            "1 instance, current:stale, stale"
        );
        assert_eq!(
            hypr_instance_warnings(&stale_current),
            vec![
                "current Hyprland instance is stale or its PID is missing".to_string(),
                "current Hyprland command socket is missing".to_string(),
                "current Hyprland event socket is missing".to_string(),
            ]
        );
    }
}
