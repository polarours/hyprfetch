use serde::{Deserialize, Serialize};

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
    pub fullscreen: Option<i32>,
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

    pub fn workspace_history(&self) -> Result<String, ProviderError> {
        let option = self.option("animations:workspace:lastforeignfocus")?;
        let ws_name = option
            .str_value
            .or_else(|| {
                option
                    .data
                    .as_ref()
                    .and_then(|v| v.as_str().map(str::to_string))
            })
            .ok_or_else(|| ProviderError::parse("lastforeignfocus has no string value"))?;
        Ok(ws_name)
    }
}
