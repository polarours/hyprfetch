//! The OpenGL row.

use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};

pub struct OpenGlModule;

impl Module for OpenGlModule {
    fn id(&self) -> ModuleId {
        ModuleId::OpenGl
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.opengl() {
            Some(info) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("OpenGL", info.display())],
                json!({ "version": info.version, "renderer": info.renderer }),
            ),
            None => ModuleOutput::unavailable(self.id(), "glxinfo unavailable".to_string()),
        }
    }
}
