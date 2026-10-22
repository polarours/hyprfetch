use super::HyprlandProvider;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ProviderError;
use super::hyprctl_message;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprBind {
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub mouse: bool,
    #[serde(default)]
    pub release: bool,
    #[serde(default)]
    pub repeat: bool,
    #[serde(rename = "nonConsuming", default)]
    pub non_consuming: bool,
    #[serde(rename = "hasDescription", default)]
    pub has_description: bool,
    pub description: Option<String>,
    pub modmask: Option<i64>,
    pub submap: Option<String>,
    pub key: Option<String>,
    pub keycode: Option<i64>,
    #[serde(rename = "catchAll", default)]
    pub catch_all: bool,
    pub dispatcher: Option<String>,
    pub arg: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HyprOption {
    pub option: Option<String>,
    pub int: Option<i64>,
    pub float: Option<f64>,
    #[serde(rename = "str")]
    pub str_value: Option<String>,
    pub data: Option<Value>,
}

impl HyprlandProvider {
    pub fn binds(&self) -> Result<Vec<HyprBind>, ProviderError> {
        self.cached(&self.binds, "binds")
    }

    pub fn config_errors(&self) -> Result<Value, ProviderError> {
        self.cached(&self.config_errors, "configerrors")
    }

    pub fn global_shortcuts(&self) -> Result<Value, ProviderError> {
        self.cached(&self.global_shortcuts, "globalshortcuts")
    }

    pub fn layouts(&self) -> Result<Value, ProviderError> {
        self.cached(&self.layouts, "layouts")
    }

    pub fn option(&self, name: &str) -> Result<HyprOption, ProviderError> {
        let raw = self.runner.run("hyprctl", &["-j", "getoption", name])?;
        serde_json::from_str::<HyprOption>(&raw).map_err(|error| {
            if let Some(message) = hyprctl_message(&raw) {
                return ProviderError::command(format!("hyprctl getoption {name}: {message}"));
            }
            ProviderError::parse(format!(
                "failed to parse hyprctl getoption {name} JSON: {error}"
            ))
        })
    }
}
#[cfg(test)]
mod tests {
    use super::super::HyprlandProvider;
    use super::super::MockRunner;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn parses_hyprctl_getoption() {
        let provider = HyprlandProvider::with_runner(Arc::new(MockRunner {
            payloads: HashMap::from([(
                "general:layout".to_string(),
                r#"{ "option": "general:layout", "int": 0, "float": 0.0, "str": "dwindle", "data": "dwindle" }"#
                    .to_string(),
            )]),
        }));

        let option = provider.option("general:layout").unwrap();
        assert_eq!(option.option.as_deref(), Some("general:layout"));
        assert_eq!(option.str_value.as_deref(), Some("dwindle"));
        assert_eq!(option.data, Some(serde_json::json!("dwindle")));
    }
}
