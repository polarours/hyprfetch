use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::HyprlandProvider;
use super::ProviderError;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprWorkspace {
    pub id: Option<i32>,
    pub name: Option<String>,
    pub monitor: Option<String>,
    pub windows: Option<i32>,
    #[serde(rename = "hasfullscreen")]
    pub has_fullscreen: Option<bool>,
    /// The title of the window that was last focused in this workspace.
    #[serde(rename = "lastwindowtitle")]
    pub last_window_title: Option<String>,
}

impl HyprWorkspace {
    pub fn display_name(&self) -> String {
        self.name
            .clone()
            .or_else(|| self.id.map(|id| id.to_string()))
            .unwrap_or_else(|| "unknown".to_string())
    }
}

impl HyprlandProvider {
    pub fn active_workspace(&self) -> Result<HyprWorkspace, ProviderError> {
        self.cached(&self.active_workspace, "activeworkspace")
    }

    pub fn workspaces(&self) -> Result<Vec<HyprWorkspace>, ProviderError> {
        self.cached(&self.workspaces, "workspaces")
    }

    pub fn workspace_rules(&self) -> Result<Value, ProviderError> {
        self.cached(&self.workspace_rules, "workspacerules")
    }
}
