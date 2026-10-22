//! Rendering the collected modules.
//!
//! The layout decides how the rows are arranged, the style decides how
//! they are painted and mod.rs ties both together.

use std::env;
use std::fs;
use std::io::{self, IsTerminal};
use unicode_width::UnicodeWidthStr;

use anyhow::Result;

use crate::config::RuntimeConfig;
use crate::model::{ModuleId, ModuleOutput, ModuleStatus};

mod layout;
mod style;

use layout::render_text_with_width;

const MIN_SPLIT_VALUE_WIDTH: usize = 24;

const MIN_COLUMN_VALUE_WIDTH: usize = 20;

const DENSE_PRIORITY_WIDTH: usize = 24;

const DENSE_SEPARATOR: &str = " ";

const DEFAULT_HEADER_WIDTH: usize = 40;

const MAX_HEADER_WIDTH: usize = 44;

const MIN_HEADER_WIDTH: usize = 8;

#[derive(Debug, Clone)]
struct RenderRow {
    key: String,
    value: String,
    status: ModuleStatus,
}

#[derive(Debug, Clone, Copy)]
struct InfoRenderOptions {
    key_width: usize,
    value_width: Option<usize>,
    max_line_width: Option<usize>,
    include_header: bool,
}

pub fn render(outputs: &[ModuleOutput], runtime: &RuntimeConfig) -> Result<String> {
    if runtime.diagnostics {
        return Ok(render_diagnostics(outputs));
    }

    if runtime.json {
        return render_json(outputs);
    }

    Ok(render_text(outputs, runtime))
}

pub fn render_json(outputs: &[ModuleOutput]) -> Result<String> {
    let rendered = serde_json::to_string_pretty(outputs)?;
    Ok(format!("{rendered}\n"))
}

pub fn render_diagnostics(outputs: &[ModuleOutput]) -> String {
    let mut output = String::new();
    output.push_str("hyprfetch diagnostics\n");

    let ok_count = outputs
        .iter()
        .filter(|output| output.status == ModuleStatus::Ok)
        .count();
    let partial_count = outputs
        .iter()
        .filter(|output| output.status == ModuleStatus::Partial)
        .count();
    let unavailable_count = outputs
        .iter()
        .filter(|output| output.status == ModuleStatus::Unavailable)
        .count();

    output.push_str(&format!(
        "summary: {ok_count} ok, {partial_count} partial, {unavailable_count} unavailable\n"
    ));

    for output_item in outputs {
        let status = match output_item.status {
            ModuleStatus::Ok => "ok",
            ModuleStatus::Partial => "partial",
            ModuleStatus::Unavailable => "unavailable",
        };
        output.push_str(&format!("{}: {status}", output_item.id));
        if let Some(message) = output_item.message.as_deref() {
            output.push_str(&format!(" - {message}"));
        }
        output.push('\n');
    }

    output
}

pub fn render_text(outputs: &[ModuleOutput], runtime: &RuntimeConfig) -> String {
    render_text_with_width(outputs, runtime, detect_terminal_width())
}

fn output_priority(id: &str) -> usize {
    let Ok(id) = id.parse::<ModuleId>() else {
        return 1;
    };

    match id {
        ModuleId::Os
        | ModuleId::Host
        | ModuleId::Kernel
        | ModuleId::Uptime
        | ModuleId::Packages
        | ModuleId::Memory
        | ModuleId::Hyprland
        | ModuleId::HyprHealth
        | ModuleId::HyprInstances
        | ModuleId::HyprActiveWindow
        | ModuleId::HyprCursor
        | ModuleId::HyprWorkspaceHistory
        | ModuleId::HyprWorkspaces
        | ModuleId::HyprMonitors => 0,
        ModuleId::Shell
        | ModuleId::Terminal
        | ModuleId::Cpu
        | ModuleId::Theme
        | ModuleId::HyprRice
        | ModuleId::HyprConfig
        | ModuleId::HyprRules => 1,
        ModuleId::Gpu
        | ModuleId::Swap
        | ModuleId::Disk
        | ModuleId::Battery
        | ModuleId::Locale
        | ModuleId::Network
        | ModuleId::Temperature
        | ModuleId::Audio
        | ModuleId::Datetime
        | ModuleId::Colorscheme
        | ModuleId::Wallpaper
        | ModuleId::HyprEcosystem
        | ModuleId::HyprSplash
        | ModuleId::HyprDevices
        | ModuleId::HyprBinds
        | ModuleId::HyprShortcuts
        | ModuleId::HyprLayouts
        | ModuleId::HyprLayers => 2,
    }
}

fn output_label(id: &str) -> String {
    id.parse::<ModuleId>()
        .map(|module| module.label().to_string())
        .unwrap_or_else(|_| id.to_string())
}

fn header_title() -> Option<String> {
    let user = env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .ok()
        .filter(|value| !value.trim().is_empty());
    let host = fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            env::var("HOSTNAME")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });

    match (user, host) {
        (Some(user), Some(host)) => Some(format!("{user}@{host}")),
        (Some(user), None) => Some(user),
        (None, Some(host)) => Some(host),
        (None, None) => None,
    }
}

fn header_width(title: &str, max_line_width: Option<usize>) -> usize {
    let max_width = max_line_width
        .unwrap_or(DEFAULT_HEADER_WIDTH)
        .clamp(MIN_HEADER_WIDTH, MAX_HEADER_WIDTH);
    UnicodeWidthStr::width(title).clamp(MIN_HEADER_WIDTH, max_width)
}

fn detect_terminal_width() -> Option<usize> {
    query_terminal_width().or_else(|| {
        env::var("COLUMNS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|width| *width > 0)
    })
}

#[cfg(unix)]
fn query_terminal_width() -> Option<usize> {
    use libc::{STDOUT_FILENO, TIOCGWINSZ, ioctl, winsize};

    if !io::stdout().is_terminal() {
        return None;
    }

    let mut size = winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let result = unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut size) };
    (result == 0 && size.ws_col > 0).then_some(size.ws_col as usize)
}

#[cfg(not(unix))]
fn query_terminal_width() -> Option<usize> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::model::{ModuleId, ModuleOutput, OutputRow};
    use serde_json::json;

    #[test]
    fn assigns_lower_priority_to_auxiliary_modules() {
        assert!(output_priority("hyprland") < output_priority("wallpaper"));
        assert!(output_priority("theme") < output_priority("hypr-devices"));
    }

    #[test]
    fn renders_diagnostics_with_module_statuses() {
        let outputs = vec![
            ModuleOutput::ok(
                ModuleId::Os,
                vec![OutputRow::new("OS", "Arch Linux")],
                json!({}),
            ),
            ModuleOutput::partial(
                ModuleId::Hyprland,
                vec![OutputRow::new("Hyprland", "0.48.1")],
                json!({}),
                "clients unavailable",
            ),
            ModuleOutput::unavailable(ModuleId::Wallpaper, "no wallpaper backend detected"),
        ];

        let rendered = render_diagnostics(&outputs);
        assert!(rendered.contains("summary: 1 ok, 1 partial, 1 unavailable"));
        assert!(rendered.contains("os: ok"));
        assert!(rendered.contains("hyprland: partial - clients unavailable"));
        assert!(rendered.contains("wallpaper: unavailable - no wallpaper backend detected"));
    }
}
