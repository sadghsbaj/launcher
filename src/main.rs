mod frecency;
mod plugin;
mod plugins;
mod router;
mod ui;
pub mod color_picker;
pub mod config;

use frecency::FrecencyStore;
use plugin::LauncherPlugin;
use plugins::{ApplicationsPlugin, CalculatorPlugin, ClipboardPlugin, SystemPlugin, WebSearchPlugin, WindowSwitcherPlugin, TranslatorPlugin};
use router::SearchRouter;
use std::sync::{Arc, Mutex};

fn main() {
    // 0. Load Configuration
    let config = config::load_or_create();

    // 1. Initialize Frecency Database
    let frecency = Arc::new(Mutex::new(FrecencyStore::load()));

    // 2. Initialize Shared Clipboard History
    let clipboard_history = Arc::new(Mutex::new(Vec::<String>::new()));

    // 3. Register Plugins
    let plugins: Vec<Box<dyn LauncherPlugin>> = vec![
        Box::new(ApplicationsPlugin::new()),
        Box::new(CalculatorPlugin::new()),
        Box::new(SystemPlugin::new()),
        Box::new(WebSearchPlugin::new()),
        Box::new(ClipboardPlugin::new(clipboard_history.clone())),
        Box::new(WindowSwitcherPlugin::new()),
        Box::new(TranslatorPlugin::new()),
    ];

    // 4. Assemble Router
    let router = Arc::new(SearchRouter::new(frecency, plugins));

    // 5. Start User Interface
    ui::run_ui(router, clipboard_history, config);
}
