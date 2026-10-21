use super::HyprlandProvider;
use serde::{Deserialize, Serialize};

use super::ProviderError;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprInputDevice {
    pub name: Option<String>,
    pub main: Option<bool>,
    pub layout: Option<String>,
    pub variant: Option<String>,
    pub options: Option<String>,
    pub active_keymap: Option<String>,
    #[serde(rename = "capsLock")]
    pub caps_lock: Option<bool>,
    #[serde(rename = "numLock")]
    pub num_lock: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprDevices {
    #[serde(default)]
    pub keyboards: Vec<HyprInputDevice>,
    #[serde(default)]
    pub mice: Vec<HyprInputDevice>,
    #[serde(default)]
    pub tablets: Vec<HyprInputDevice>,
    #[serde(default)]
    pub touch: Vec<HyprInputDevice>,
    #[serde(default)]
    pub switches: Vec<HyprInputDevice>,
}

impl HyprDevices {
    pub fn total_count(&self) -> usize {
        self.keyboards.len()
            + self.mice.len()
            + self.tablets.len()
            + self.touch.len()
            + self.switches.len()
    }
}

impl HyprlandProvider {
    pub fn devices(&self) -> Result<HyprDevices, ProviderError> {
        self.cached(&self.devices, "devices")
    }
}
