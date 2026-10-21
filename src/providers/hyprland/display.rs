use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::HyprlandProvider;
use super::ProviderError;
use super::workspace::HyprWorkspace;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprMonitor {
    pub name: String,
    pub description: Option<String>,
    pub width: i32,
    pub height: i32,
    #[serde(rename = "refreshRate")]
    pub refresh_rate: f64,
    pub scale: f64,
    #[serde(default)]
    pub focused: bool,
    #[serde(rename = "activeWorkspace")]
    pub active_workspace: Option<HyprWorkspace>,
    #[serde(rename = "dpmsStatus")]
    pub dpms_status: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprLayerSurface {
    pub address: Option<String>,
    pub namespace: Option<String>,
    pub layer: Option<i32>,
    pub monitor: Option<i32>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub w: Option<i32>,
    pub h: Option<i32>,
    pub keyboard_interactive: Option<Value>,
    pub exclusive: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprLayerMonitor {
    #[serde(default)]
    pub levels: BTreeMap<String, Vec<HyprLayerSurface>>,
}

pub type HyprLayers = BTreeMap<String, HyprLayerMonitor>;

impl HyprlandProvider {
    pub fn monitors(&self) -> Result<Vec<HyprMonitor>, ProviderError> {
        self.cached(&self.monitors, "monitors")
    }

    pub fn layers(&self) -> Result<HyprLayers, ProviderError> {
        self.cached(&self.layers, "layers")
    }
}
