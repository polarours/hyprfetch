pub mod hyprland;
pub mod system;
pub mod theme;

use std::sync::Arc;

use hyprland::{CommandRunner, HyprlandProvider, RealCommandRunner};
pub use system::SystemProvider;
use theme::ThemeProvider;

#[derive(Clone)]
pub struct Providers {
    pub system: SystemProvider,
    pub theme: ThemeProvider,
    pub hyprland: HyprlandProvider,
}

impl Providers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build with a system provider configured for fast or normal collection.
    pub fn with_system(system: SystemProvider) -> Self {
        Self {
            system,
            ..Self::default()
        }
    }

    pub fn with_hypr_runner(runner: Arc<dyn CommandRunner>) -> Self {
        Self {
            system: SystemProvider::new(),
            theme: ThemeProvider::new(),
            hyprland: HyprlandProvider::with_runner(runner),
        }
    }
}

impl Default for Providers {
    fn default() -> Self {
        Self::with_hypr_runner(Arc::new(RealCommandRunner::default()))
    }
}
