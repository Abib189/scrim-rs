//! The macOS app menu (Fix AI C-2, after upstream FigCraft 73398ced, which installed craft-libs'
//! craft-menu; we use muda, the menu crate under it, directly). winit's default app menu ends the
//! process through AppKit's `terminate:` the moment Quit or ⌘Q is chosen, so unsaved changes were
//! lost without a word. This menu has the same items, and its Quit Scrim (⌘Q) runs the `app.quit`
//! command instead, which asks first when a file has unsaved changes; About Scrim runs
//! `help.about`. The labels are the commands', in the interface language the app started in.
//!
//! If it can't be installed there is no app menu (winit's is turned off), so ⌘Q does nothing
//! (rather than quit without asking); File ▸ Quit Scrim and the Dock's Quit still ask. It's logged.

use std::sync::{Arc, Mutex, PoisonError};

use figcraft_ui_egui::FigApp;
use figcraft_ui_egui::i18n::{command as label, tr};
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use serde_json::json;

const QUIT: &str = "scrim.quit";
const ABOUT: &str = "scrim.about";

/// The command menu item `id` runs.
pub fn command(id: &str) -> Option<&'static str> {
    match id {
        QUIT => Some("app.quit"),
        ABOUT => Some("help.about"),
        _ => None,
    }
}

/// The installed menu, and the items chosen since the last frame.
pub struct AppMenu {
    _menu: Menu,
    chosen: Arc<Mutex<Vec<String>>>,
}

impl AppMenu {
    /// Make it the app's menu; a choice wakes `ctx`. Call on the main thread.
    pub fn install(ctx: &egui::Context) -> muda::Result<Self> {
        let app = Submenu::with_items(
            "Scrim",
            true,
            &[
                &MenuItem::with_id(ABOUT, label("help.about", tr("About Scrim")), true, None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::services(None),
                &PredefinedMenuItem::separator(),
                &PredefinedMenuItem::hide(None),
                &PredefinedMenuItem::hide_others(None),
                &PredefinedMenuItem::show_all(None),
                &PredefinedMenuItem::separator(),
                &MenuItem::with_id(QUIT, label("app.quit", "Quit Scrim"), true, Some(Accelerator::new(Some(Modifiers::SUPER), Code::KeyQ))),
            ],
        )?;
        let menu = Menu::new();
        menu.append(&app)?;
        menu.init_for_nsapp();
        let chosen = Arc::new(Mutex::new(Vec::new()));
        let (queue, ctx) = (chosen.clone(), ctx.clone());
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            queue.lock().unwrap_or_else(PoisonError::into_inner).push(e.id.0);
            ctx.request_repaint();
        }));
        Ok(Self { _menu: menu, chosen })
    }

    /// Run the commands of the items chosen since the last frame.
    pub fn poll(&self, fig: &mut FigApp) {
        let ids = std::mem::take(&mut *self.chosen.lock().unwrap_or_else(PoisonError::into_inner));
        for c in ids.iter().filter_map(|id| command(id)) {
            fig.run(c, json!({}));
        }
    }
}
