//! The grace period's self-diagnosis (Fix Q2, `docs/design/working-state.md` › Fix Q2).
//! - **Noticed:** a client's working state ended because the grace period ran out, and the same
//!   client made a request within two minutes ([`super::working_state::CAME_BACK_WITHIN`]): the
//!   setting is shorter than its pauses between steps.
//! - **Kept:** a `working` event in Settings ▸ Diagnostics, "grace too short: {client} came back
//!   after {n} s (the setting is {s} s)", fixed words, no content.
//! - **Said once per run** in the AI tab: "The indicator ended before the agent's next step. A
//!   longer setting keeps it on.", with a button that opens Settings ▸ General at the setting.
//! - **Nothing changes the setting on its own.**

use egui::Ui;
use figcraft_engine::settings::diagnostics::Entry;
use serde_json::json;

use crate::FigApp;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::widgets;

/// Whether the hint shows, and whether it has this run.
#[derive(Default, Debug)]
pub struct State {
    pub showing: bool,
    pub shown: bool,
}

/// Every frame, after the working state's tick: the clients that came back, into Diagnostics, and
/// the hint the first time.
pub fn note(app: &mut FigApp) {
    let back = app.ui.agent_view.working.came_back();
    if back.is_empty() {
        return;
    }
    let setting = app.session.agent_grace();
    for b in &back {
        let after = b.after.max(0.0).round() as u64;
        crate::settings_privacy::record(app, Entry::GraceTooShort { client: b.client.as_deref(), after, setting });
    }
    let h = &mut app.ui.agent_view.grace_hint;
    if !h.shown {
        h.shown = true;
        h.showing = true;
    }
}

/// The AI tab's hint row, while it shows: its words and the button to the setting.
pub fn row(app: &mut FigApp, ui: &mut Ui) {
    if !app.ui.agent_view.grace_hint.showing {
        return;
    }
    let t = Tokens::get(ui.ctx());
    widgets::note(ui, tr("The indicator ended before the agent's next step. A longer setting keeps it on."), t.warning);
    crate::right::row(ui, 32.0, |ui| {
        if widgets::button(ui, tr("Open the setting"), false, true).clicked() {
            open_setting(app);
        }
    });
}

/// Settings ▸ General, scrolled to the grace; the hint is done.
pub fn open_setting(app: &mut FigApp) {
    app.ui.agent_view.grace_hint.showing = false;
    app.ui.agent_view.focus_grace = true;
    crate::settings_dialog::request(app, "dialog.settings", &json!({"section": "general"}));
}

#[cfg(test)]
#[path = "grace_hint_tests.rs"]
mod tests;
