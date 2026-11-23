//! Deep diagnostics (`--doctor`): explain *why* a module failed and how to fix it.
//!
//! Unlike `--diagnostics`, which only lists statuses, the doctor classifies each
//! message and offers an actionable hint. It never fabricates data — it reads the
//! same ModuleOutput the renderer does.

use crate::model::{ModuleOutput, ModuleStatus};

/// A hint distilled from a failure message. `None` means no specific advice.
fn hint_for(message: &str) -> Option<&'static str> {
    let m = message.to_lowercase();
    if m.contains("hyprland_instance_signature") || m.contains("hyprland instance signature") {
        return Some(
            "Run hyprfetch from inside a Hyprland session, or set HYPRLAND_INSTANCE_SIGNATURE to the active instance.",
        );
    }
    if m.contains("socket") && m.contains("missing") {
        return Some(
            "The Hyprland runtime socket is missing. Check $XDG_RUNTIME_DIR/hypr/<instance> and that Hyprland is running.",
        );
    }
    if m.contains("xdg_runtime_dir") {
        return Some(
            "XDG_RUNTIME_DIR is unset. It is normally set by the display manager; export it and retry.",
        );
    }
    if m.contains("hyprctl") || m.contains("command socket") {
        return Some(
            "hyprctl cannot reach the compositor. Verify Hyprland is running and hyprctl is on PATH.",
        );
    }
    if m.contains("daemon")
        || (m.contains("bar")
            || m.contains("notification")
            || m.contains("lock")
            || m.contains("idle"))
            && m.contains("missing")
    {
        return Some(
            "This is a rice/health hint: start the missing daemons (bar, notifications, idle, lock) to make the health row ready. It is not a fetch failure.",
        );
    }
    if m.contains("not found") || m.contains("no ") {
        if m.contains("playerctl") || m.contains("mpris") {
            return Some("Install playerctl and start a media player that exports MPRIS.");
        }
        if m.contains("pactl")
            || m.contains("wpctl")
            || m.contains("pipewire")
            || m.contains("pulse")
        {
            return Some(
                "Install PipeWire/PulseAudio tools (wpctl/pactl) and make sure the audio socket exists in $XDG_RUNTIME_DIR.",
            );
        }
        if m.contains("iw") || m.contains("iwconfig") || m.contains("wifi") {
            return Some("Install iw or wireless_tools to report wireless SSID and signal.");
        }
        if m.contains("daemon")
            || m.contains("bar")
            || m.contains("notification")
            || m.contains("lock")
            || m.contains("idle")
        {
            return Some(
                "This is a rice/health hint: start the missing daemons (bar, notifications, idle, lock) to make the health row ready. It is not a fetch failure.",
            );
        }
    }
    if m.contains("timed out") {
        return Some(
            "An external probe exceeded its deadline. Check the tool it names, or use --fast to skip slow probes.",
        );
    }
    if m.contains("gtk") || m.contains("theme") && m.contains("settings") {
        return Some(
            "No GTK settings.ini found. Install a GTK theme or check $XDG_CONFIG_HOME/gtk-3.0/settings.ini.",
        );
    }
    None
}

/// Render a doctor report.
pub fn render_doctor(outputs: &[ModuleOutput]) -> String {
    let mut out = String::new();
    out.push_str("hyprfetch doctor\n");

    let problems: Vec<&ModuleOutput> = outputs
        .iter()
        .filter(|o| o.status != ModuleStatus::Ok)
        .collect();
    let ok = outputs.len() - problems.len();

    out.push_str(&format!(
        "summary: {} ok, {} need attention ({} modules checked)\n\n",
        ok,
        problems.len(),
        outputs.len()
    ));

    if problems.is_empty() {
        out.push_str("No issues found — every module reported successfully.\n");
        return out;
    }

    for item in &problems {
        let status = match item.status {
            ModuleStatus::Ok => "ok",
            ModuleStatus::Partial => "partial",
            ModuleStatus::Unavailable => "unavailable",
        };
        out.push_str(&format!("[{status}] {}\n", item.id));
        if let Some(message) = item.message.as_deref() {
            out.push_str(&format!("  why: {message}\n"));
            if let Some(hint) = hint_for(message) {
                out.push_str(&format!("  fix: {hint}\n"));
            } else {
                out.push_str("  fix: check the tool or environment this module depends on.\n");
            }
        } else {
            out.push_str("  why: the module reported a problem without a message.\n");
        }
        out.push('\n');
    }

    out.push_str("General tips:\n");
    out.push_str("  - Run inside the Hyprland session you want to inspect.\n");
    out.push_str(
        "  - Install the optional tools named above (hyprctl, playerctl, wpctl/pactl, iw).\n",
    );
    out.push_str("  - Use --fast to skip probes that need a sampling window or a vendor tool.\n");
    out.push_str("  - For raw data, run hyprfetch --json.\n");
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::model::ModuleId;

    #[test]
    fn all_ok_reports_no_issues() {
        let outputs = vec![ModuleOutput::ok(ModuleId::Os, vec![], json!({}))];
        let report = render_doctor(&outputs);
        assert!(report.contains("No issues found"));
    }

    #[test]
    fn hyprland_message_gets_a_targeted_hint() {
        let mut out = ModuleOutput::unavailable(
            ModuleId::Hyprland,
            "hyprctl requires HYPRLAND_INSTANCE_SIGNATURE, but it is missing".to_string(),
        );
        out.status = ModuleStatus::Unavailable;
        let report = render_doctor(&[out]);
        assert!(report.contains("why:"));
        assert!(report.contains("HYPRLAND_INSTANCE_SIGNATURE"));
        assert!(report.contains("fix:"));
    }

    #[test]
    fn daemon_hint_is_not_treated_as_a_failure() {
        let mut out = ModuleOutput::ok(ModuleId::HyprHealth, vec![], json!({}));
        out.status = ModuleStatus::Partial;
        out.message = Some("idle daemon missing; lock daemon missing".to_string());
        let report = render_doctor(&[out]);
        assert!(report.contains("rice/health hint"));
        assert!(report.contains("not a fetch failure"));
    }
}
