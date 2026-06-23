use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::providers::theme::ThemeInfo;

pub struct ThemeModule;
pub struct WallpaperModule;

impl Module for ThemeModule {
    fn id(&self) -> ModuleId {
        ModuleId::Theme
    }

    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.theme.theme_info() {
            Ok(theme) => {
                let compact = config.option_bool("compact", false);
                let show_font = config.option_bool("show_font", true);
                if compact {
                    return ModuleOutput::ok(
                        self.id(),
                        vec![OutputRow::new("Theme", summarize_theme(&theme, show_font))],
                        json!(theme),
                    );
                }

                let mut rows = Vec::new();
                if let Some(value) = &theme.gtk_theme {
                    rows.push(OutputRow::new("GTK", value.clone()));
                }
                if let Some(value) = &theme.icon_theme {
                    rows.push(OutputRow::new("Icons", value.clone()));
                }
                if let Some(value) = &theme.cursor_theme {
                    rows.push(OutputRow::new("Cursor", value.clone()));
                }
                if show_font && let Some(value) = &theme.font {
                    rows.push(OutputRow::new("Font", value.clone()));
                }

                if rows.is_empty() {
                    ModuleOutput::unavailable(
                        self.id(),
                        "theme settings did not contain displayable fields",
                    )
                } else {
                    ModuleOutput::ok(self.id(), rows, json!(theme))
                }
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

fn summarize_theme(theme: &ThemeInfo, show_font: bool) -> String {
    let mut parts = Vec::new();

    if let Some(value) = theme.gtk_theme.as_deref() {
        parts.push(format!("gtk:{value}"));
    }
    if let Some(value) = theme.icon_theme.as_deref() {
        parts.push(format!("icons:{value}"));
    }
    if let Some(value) = theme.cursor_theme.as_deref() {
        parts.push(format!("cursor:{value}"));
    }
    if show_font && let Some(value) = theme.font.as_deref() {
        parts.push(format!("font:{value}"));
    }

    parts.join(" ")
}

impl Module for WallpaperModule {
    fn id(&self) -> ModuleId {
        ModuleId::Wallpaper
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.theme.wallpaper_backend() {
            Ok(backend) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Wallpaper", backend.clone())],
                json!({ "backend": backend }),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::summarize_theme;
    use crate::providers::theme::ThemeInfo;

    #[test]
    fn summarizes_theme_in_compact_mode() {
        let theme = ThemeInfo {
            gtk_theme: Some("Adwaita".to_string()),
            icon_theme: Some("Papirus-Dark".to_string()),
            cursor_theme: Some("Bibata".to_string()),
            font: Some("JetBrainsMono 11".to_string()),
        };

        assert_eq!(
            summarize_theme(&theme, true),
            "gtk:Adwaita icons:Papirus-Dark cursor:Bibata font:JetBrainsMono 11"
        );
    }

    #[test]
    fn compact_theme_can_hide_font() {
        let theme = ThemeInfo {
            gtk_theme: Some("Adwaita".to_string()),
            font: Some("JetBrainsMono 11".to_string()),
            ..ThemeInfo::default()
        };

        assert_eq!(summarize_theme(&theme, false), "gtk:Adwaita");
    }
}
