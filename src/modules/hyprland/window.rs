use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::util::truncate_visible;

pub struct HyprActiveWindowModule;
pub struct HyprCursorModule;

impl Module for HyprActiveWindowModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprActiveWindow
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.active_window() {
            Ok(window) => {
                let compact = config.option_bool("compact", true);
                if compact {
                    let display = summarize_active_window(&window, 72);
                    return ModuleOutput::ok(
                        self.id(),
                        vec![OutputRow::new("Window", display)],
                        json!({ "window": window }),
                    );
                }

                let mut rows = Vec::new();
                if let Some(title) = window.title.as_deref().filter(|title| !title.is_empty()) {
                    rows.push(OutputRow::new("Title", title));
                }
                if let Some(class) = window.class.as_deref().filter(|class| !class.is_empty()) {
                    rows.push(OutputRow::new("Class", class));
                }
                if let Some(workspace) = window.workspace.as_ref() {
                    rows.push(OutputRow::new("Workspace", workspace.display_name()));
                }
                rows.push(OutputRow::new("State", window_state(&window)));

                ModuleOutput::ok(self.id(), rows, json!({ "window": window }))
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for HyprCursorModule {
    fn id(&self) -> ModuleId {
        ModuleId::HyprCursor
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.hyprland.cursor_position() {
            Ok(position) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new(
                    "Cursor",
                    format!("{}, {}", position.x, position.y),
                )],
                json!(position),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

fn summarize_active_window(
    window: &crate::providers::hyprland::HyprWindow,
    max_chars: usize,
) -> String {
    let title = window
        .title
        .as_deref()
        .filter(|title| !title.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| window.display_name());
    let mut metadata = Vec::new();

    if let Some(class) = window.class.as_deref().filter(|class| !class.is_empty())
        && window
            .title
            .as_deref()
            .filter(|title| !title.is_empty())
            .is_some()
    {
        metadata.push(class.to_string());
    }
    if let Some(workspace) = window.workspace.as_ref() {
        metadata.push(format!("ws:{}", workspace.display_name()));
    }

    let state = window_state(window);
    if state != "tiled" {
        metadata.push(state);
    }

    if metadata.is_empty() {
        return truncate_visible(&title, max_chars);
    }

    let metadata = format!(" [{}]", metadata.join(", "));
    let metadata_width = metadata.chars().count();
    if metadata_width >= max_chars {
        return truncate_visible(&metadata, max_chars);
    }

    let title_budget = max_chars.saturating_sub(metadata_width);
    format!("{}{}", truncate_visible(&title, title_budget), metadata)
}

fn window_state(window: &crate::providers::hyprland::HyprWindow) -> String {
    let mut states = Vec::new();

    if window.is_fullscreen() {
        states.push("fullscreen");
    }
    if window.floating {
        states.push("floating");
    }
    if window.pinned {
        states.push("pinned");
    }
    if window.hidden {
        states.push("hidden");
    }
    if !window.mapped {
        states.push("unmapped");
    }

    if states.is_empty() {
        return "tiled".to_string();
    }

    states.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::hyprland::{HyprWindow, HyprWorkspace};

    #[test]
    fn formats_active_window_state_summary() {
        let window = HyprWindow {
            class: Some("kitty".to_string()),
            title: Some("codex - hyprfetch".to_string()),
            workspace: Some(HyprWorkspace {
                id: Some(1),
                name: Some("1".to_string()),
                ..HyprWorkspace::default()
            }),
            fullscreen: Some(1),
            mapped: true,
            ..HyprWindow::default()
        };

        assert_eq!(window_state(&window), "fullscreen");
        assert_eq!(
            summarize_active_window(&window, 72),
            "codex - hyprfetch [kitty, ws:1, fullscreen]"
        );
    }

    #[test]
    fn preserves_window_metadata_when_title_is_long() {
        let window = HyprWindow {
            class: Some("firefox".to_string()),
            title: Some(
                "This is a very long browser title that should not hide workspace state"
                    .to_string(),
            ),
            workspace: Some(HyprWorkspace {
                id: Some(2),
                name: Some("2".to_string()),
                ..HyprWorkspace::default()
            }),
            floating: true,
            mapped: true,
            ..HyprWindow::default()
        };

        let summary = summarize_active_window(&window, 48);
        assert!(summary.ends_with("[firefox, ws:2, floating]"));
        assert!(summary.chars().count() <= 48);
    }
}
