use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::util::format_refresh_hz;

use super::workspaces::workspace_name;

pub struct HyprMonitorsModule;
impl Module for HyprMonitorsModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprMonitors
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.monitors() {
            Ok(monitors) => {
                let compact = config.option_bool("compact", true);
                if compact {
                    let summary = monitors
                        .iter()
                        .map(summarize_monitor)
                        .collect::<Vec<_>>()
                        .join(", ");
                    return ModuleOutput::ok(
                        self.id(),
                        vec![OutputRow::new(
                            "Monitors",
                            format!("{} ({summary})", monitors.len()),
                        )],
                        json!({ "monitors": monitors }),
                    );
                }

                let mut rows = vec![OutputRow::new("Monitors", monitors.len().to_string())];
                for monitor in &monitors {
                    rows.push(OutputRow::new(
                        monitor_label(monitor),
                        monitor_details(monitor),
                    ));
                }

                ModuleOutput::ok(self.id(), rows, json!({ "monitors": monitors }))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

pub(super) fn focused_monitor_summary(
    monitors: &[crate::providers::hyprland::HyprMonitor],
) -> Option<String> {
    let monitor = monitors.iter().find(|monitor| monitor.focused)?;
    let mut summary = monitor.name.clone();

    if let Some(workspace) = monitor.active_workspace.as_ref() {
        summary.push_str(&format!(" (ws:{})", workspace.display_name()));
    }

    Some(summary)
}

fn summarize_monitor(monitor: &crate::providers::hyprland::HyprMonitor) -> String {
    let mut summary = monitor_label(monitor);

    if let Some(workspace) = monitor.active_workspace.as_ref() {
        summary.push(':');
        summary.push_str(&workspace_name(workspace));
    }
    summary.push(' ');
    summary.push_str(&monitor_mode_summary(monitor));

    if matches!(monitor.dpms_status, Some(false)) {
        summary.push_str(" off");
    }

    summary
}

fn monitor_label(monitor: &crate::providers::hyprland::HyprMonitor) -> String {
    if monitor.focused {
        format!("*{}", monitor.name)
    } else {
        monitor.name.clone()
    }
}

fn monitor_details(monitor: &crate::providers::hyprland::HyprMonitor) -> String {
    let mut parts = Vec::new();

    if let Some(description) = monitor
        .description
        .as_deref()
        .filter(|description| !description.is_empty())
    {
        parts.push(description.to_string());
    }

    parts.push(format!(
        "{}x{} @ {}",
        monitor.width,
        monitor.height,
        format_refresh_hz(monitor.refresh_rate)
    ));

    if !scale_is_default(monitor.scale) {
        parts.push(format!("scale:{:.2}", monitor.scale));
    }

    if let Some(workspace) = monitor.active_workspace.as_ref() {
        parts.push(format!("ws:{}", workspace_name(workspace)));
    }
    if matches!(monitor.dpms_status, Some(false)) {
        parts.push("dpms:off".to_string());
    }

    parts.join(", ")
}

fn monitor_mode_summary(monitor: &crate::providers::hyprland::HyprMonitor) -> String {
    let mut summary = format!(
        "{}x{}@{}",
        monitor.width,
        monitor.height,
        format_refresh_hz(monitor.refresh_rate)
    );

    if !scale_is_default(monitor.scale) {
        summary.push_str(&format!(" x{:.2}", monitor.scale));
    }

    summary
}

fn scale_is_default(scale: f64) -> bool {
    (scale - 1.0).abs() < 0.01
}

#[cfg(test)]
mod tests {
    use super::super::session::session_overview;
    use super::*;
    use crate::providers::hyprland::{HyprMonitor, HyprWorkspace};

    #[test]
    fn formats_monitor_details_with_workspace_and_dpms() {
        let monitor = HyprMonitor {
            name: "HDMI-A-1".to_string(),
            width: 1920,
            height: 1080,
            refresh_rate: 60.0,
            scale: 1.0,
            dpms_status: Some(false),
            active_workspace: Some(HyprWorkspace {
                id: Some(2),
                name: Some("2".to_string()),
                ..HyprWorkspace::default()
            }),
            ..HyprMonitor::default()
        };

        assert_eq!(
            monitor_details(&monitor),
            "1920x1080 @ 60Hz, ws:2, dpms:off"
        );
    }

    #[test]
    fn summarizes_monitor_compact_output() {
        let monitor = HyprMonitor {
            name: "DP-1".to_string(),
            width: 2560,
            height: 1440,
            refresh_rate: 165.0,
            scale: 1.25,
            focused: true,
            active_workspace: Some(HyprWorkspace {
                id: Some(-99),
                name: Some("special:music".to_string()),
                ..HyprWorkspace::default()
            }),
            dpms_status: Some(false),
            ..HyprMonitor::default()
        };

        assert_eq!(
            summarize_monitor(&monitor),
            "*DP-1:[music] 2560x1440@165Hz x1.25 off"
        );
    }

    #[test]
    fn summarizes_focused_monitor_and_session() {
        let monitors = vec![
            HyprMonitor {
                name: "DP-1".to_string(),
                focused: true,
                active_workspace: Some(HyprWorkspace {
                    id: Some(1),
                    name: Some("1".to_string()),
                    ..HyprWorkspace::default()
                }),
                ..HyprMonitor::default()
            },
            HyprMonitor {
                name: "HDMI-A-1".to_string(),
                ..HyprMonitor::default()
            },
        ];
        let workspaces = vec![
            HyprWorkspace {
                id: Some(1),
                name: Some("1".to_string()),
                windows: Some(2),
                ..HyprWorkspace::default()
            },
            HyprWorkspace {
                id: Some(-99),
                name: Some("special:music".to_string()),
                windows: Some(1),
                has_fullscreen: Some(true),
                ..HyprWorkspace::default()
            },
        ];

        assert_eq!(
            focused_monitor_summary(&monitors).as_deref(),
            Some("DP-1 (ws:1)")
        );
        assert_eq!(
            session_overview(&monitors, &workspaces, 3),
            "2 outputs, 2 ws, 3 wins (1 special), fullscreen"
        );
    }
}
