pub mod hyprland;
pub mod system;
pub mod theme;

use crate::config::{ModuleConfig, RuntimeConfig};
use crate::model::{ModuleId, ModuleOutput};
use crate::providers::Providers;

pub struct ModuleContext<'a> {
    pub runtime: &'a RuntimeConfig,
    pub providers: &'a Providers,
}

pub trait Module {
    fn id(&self) -> ModuleId;
    fn run(&self, ctx: &ModuleContext<'_>, config: &ModuleConfig) -> ModuleOutput;
}

pub fn build_module(id: ModuleId) -> Box<dyn Module> {
    match id {
        ModuleId::Os => Box::new(system::OsModule),
        ModuleId::Host => Box::new(system::HostModule),
        ModuleId::Kernel => Box::new(system::KernelModule),
        ModuleId::Packages => Box::new(system::PackagesModule),
        ModuleId::Uptime => Box::new(system::UptimeModule),
        ModuleId::Shell => Box::new(system::ShellModule),
        ModuleId::Terminal => Box::new(system::TerminalModule),
        ModuleId::Cpu => Box::new(system::CpuModule),
        ModuleId::Gpu => Box::new(system::GpuModule),
        ModuleId::Memory => Box::new(system::MemoryModule),
        ModuleId::Swap => Box::new(system::SwapModule),
        ModuleId::Disk => Box::new(system::DiskModule),
        ModuleId::Battery => Box::new(system::BatteryModule),
        ModuleId::Locale => Box::new(system::LocaleModule),
        ModuleId::Network => Box::new(system::NetworkModule),
        ModuleId::Temperature => Box::new(system::TemperatureModule),
        ModuleId::Audio => Box::new(system::AudioModule),
        ModuleId::Datetime => Box::new(system::DatetimeModule),
        ModuleId::Colorscheme => Box::new(system::ColorschemeModule),
        ModuleId::Hyprland => Box::new(hyprland::HyprlandModule),
        ModuleId::HyprHealth => Box::new(hyprland::HyprHealthModule),
        ModuleId::HyprInstances => Box::new(hyprland::HyprInstancesModule),
        ModuleId::HyprSplash => Box::new(hyprland::HyprSplashModule),
        ModuleId::HyprWorkspaces => Box::new(hyprland::HyprWorkspacesModule),
        ModuleId::HyprMonitors => Box::new(hyprland::HyprMonitorsModule),
        ModuleId::HyprActiveWindow => Box::new(hyprland::HyprActiveWindowModule),
        ModuleId::HyprCursor => Box::new(hyprland::HyprCursorModule),
        ModuleId::HyprWorkspaceHistory => Box::new(hyprland::HyprWorkspaceHistoryModule),
        ModuleId::HyprRice => Box::new(hyprland::HyprRiceModule),
        ModuleId::HyprConfig => Box::new(hyprland::HyprConfigModule),
        ModuleId::HyprRules => Box::new(hyprland::HyprRulesModule),
        ModuleId::HyprDevices => Box::new(hyprland::HyprDevicesModule),
        ModuleId::HyprBinds => Box::new(hyprland::HyprBindsModule),
        ModuleId::HyprShortcuts => Box::new(hyprland::HyprShortcutsModule),
        ModuleId::HyprLayouts => Box::new(hyprland::HyprLayoutsModule),
        ModuleId::HyprLayers => Box::new(hyprland::HyprLayersModule),
        ModuleId::HyprEcosystem => Box::new(hyprland::HyprEcosystemModule),
        ModuleId::Theme => Box::new(theme::ThemeModule),
        ModuleId::Wallpaper => Box::new(theme::WallpaperModule),
    }
}
