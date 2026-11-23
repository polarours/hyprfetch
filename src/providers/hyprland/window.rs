use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use super::HyprlandProvider;
use super::ProviderError;
use super::default_true;
use super::workspace::HyprWorkspace;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprWindow {
    pub class: Option<String>,
    pub title: Option<String>,
    pub workspace: Option<HyprWorkspace>,
    #[serde(default)]
    pub floating: bool,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default = "default_true")]
    pub mapped: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default, deserialize_with = "deserialize_fullscreen")]
    pub fullscreen: Option<i32>,
}

/// Hyprland has reported `fullscreen` both as a JSON boolean (current
/// versions) and as an integer 0/1 (older versions and some fixtures). Accept
/// either so a modern compositor does not poison the cached window parse.
fn deserialize_fullscreen<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    let flag = match value {
        Some(Value::Bool(flag)) => flag,
        Some(Value::Number(number)) => number.as_i64().is_some_and(|value| value != 0),
        _ => false,
    };
    Ok(Some(i32::from(flag)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_boolean_fullscreen_field() {
        let raw = r#"{"class":"kitty","title":"t","fullscreen":true}"#;
        let window: HyprWindow = serde_json::from_str(raw).unwrap();
        assert!(window.is_fullscreen());
    }

    #[test]
    fn parses_a_legacy_integer_fullscreen_field() {
        let raw = r#"{"class":"kitty","title":"t","fullscreen":1}"#;
        let window: HyprWindow = serde_json::from_str(raw).unwrap();
        assert!(window.is_fullscreen());

        let raw = r#"{"class":"kitty","title":"t","fullscreen":0}"#;
        let window: HyprWindow = serde_json::from_str(raw).unwrap();
        assert!(!window.is_fullscreen());
    }
}

impl HyprWindow {
    pub fn display_name(&self) -> String {
        match (self.class.as_deref(), self.title.as_deref()) {
            (Some(class), Some(title)) if !title.is_empty() => format!("{class} - {title}"),
            (Some(class), _) => class.to_string(),
            (_, Some(title)) if !title.is_empty() => title.to_string(),
            _ => "none".to_string(),
        }
    }

    pub fn is_fullscreen(&self) -> bool {
        self.fullscreen.unwrap_or_default() > 0
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprClient {
    pub address: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprCursorPosition {
    pub x: i64,
    pub y: i64,
}

impl HyprlandProvider {
    pub fn splash(&self) -> Result<String, ProviderError> {
        self.splash.get_or_init(|| self.fetch_splash()).clone()
    }

    pub fn active_window(&self) -> Result<HyprWindow, ProviderError> {
        self.cached(&self.active_window, "activewindow")
    }

    pub fn clients(&self) -> Result<Vec<HyprClient>, ProviderError> {
        self.cached(&self.clients, "clients")
    }

    pub fn cursor_position(&self) -> Result<HyprCursorPosition, ProviderError> {
        self.cached(&self.cursor_position, "cursorpos")
    }
}
