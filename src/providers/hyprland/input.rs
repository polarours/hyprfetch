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
    /// The pointer speed a mouse reports (Hyprland's `defaultSpeed`).
    #[serde(rename = "defaultSpeed")]
    pub default_speed: Option<f64>,
    /// The scroll factor a mouse reports; `1.0` is the default and a negative
    /// value means natural scrolling.
    #[serde(rename = "scrollFactor")]
    pub scroll_factor: Option<f64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprDevices {
    #[serde(default)]
    pub keyboards: Vec<HyprInputDevice>,
    #[serde(default)]
    pub mice: Vec<HyprInputDevice>,
    #[serde(default)]
    pub tablets: Vec<HyprInputDevice>,
    /// Hyprland calls the group `touch`; some versions name it `touchpads`.
    #[serde(default, alias = "touchpads")]
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
