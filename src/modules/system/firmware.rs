//! Firmware, init system, editor and brightness rows.

use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};

pub struct BiosModule;

pub struct BoardModule;

pub struct InitSystemModule;

pub struct EditorModule;

pub struct BrightnessModule;

impl Module for BiosModule {
    fn id(&self) -> ModuleId {
        ModuleId::Bios
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.firmware_info() {
            Some(info) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("BIOS", info.display())],
                json!({ "vendor": info.vendor, "version": info.version }),
            ),
            None => ModuleOutput::unavailable(self.id(), "BIOS info unavailable".to_string()),
        }
    }
}

impl Module for BoardModule {
    fn id(&self) -> ModuleId {
        ModuleId::Board
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.board_info() {
            Some(info) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Board", info.display())],
                json!({ "vendor": info.vendor, "name": info.name }),
            ),
            None => ModuleOutput::unavailable(self.id(), "board info unavailable".to_string()),
        }
    }
}

impl Module for InitSystemModule {
    fn id(&self) -> ModuleId {
        ModuleId::InitSystem
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.init_system() {
            Some(init) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Init", init.clone())],
                json!({ "name": init }),
            ),
            None => ModuleOutput::unavailable(self.id(), "PID 1 unreadable".to_string()),
        }
    }
}

impl Module for EditorModule {
    fn id(&self) -> ModuleId {
        ModuleId::Editor
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.editor() {
            Some(editor) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Editor", editor.clone())],
                json!({ "name": editor }),
            ),
            None => ModuleOutput::unavailable(self.id(), "no editor configured".to_string()),
        }
    }
}

impl Module for BrightnessModule {
    fn id(&self) -> ModuleId {
        ModuleId::Brightness
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.brightness_percent() {
            Some(percent) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Brightness", format!("{percent}%"))],
                json!({ "percent": percent }),
            ),
            None => ModuleOutput::unavailable(self.id(), "no backlight device".to_string()),
        }
    }
}
