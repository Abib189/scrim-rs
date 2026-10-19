//! Share ▸ Online when no server is set (rebrand step A5, `docs/design/rebrand.md`): Scrim builds in
//! no server, so nothing is uploaded anywhere until the person chooses one. Online then asks for a
//! server's address; Local network needs none. Choosing a server is the `pref.publishServer`
//! command, so an agent can do the same.

use egui::Ui;
use serde_json::json;

use crate::FigApp;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::widgets;

/// The Online / Local network choice. `Some(local)` once sharing can start; `None` while Online
/// waits for a server (the address field and its button are shown instead).
pub fn mode(app: &mut FigApp, ui: &mut Ui, t: &Tokens, server: &str) -> Option<bool> {
    if let Some(i) = widgets::segmented_text(ui, &[tr("Online"), tr("Local network")], usize::from(app.ui.live.local)) {
        app.ui.live.local = i == 1;
    }
    if app.ui.live.local || !server.is_empty() {
        return Some(app.ui.live.local);
    }
    widgets::label(
        ui,
        tr(
            "To share online, enter the address of a server you trust (wss://…), or share on the local network. Nothing is uploaded until you choose a server.",
        ),
        t.text_secondary,
    );
    let id = egui::Id::new("live_server_address_draft");
    let mut draft: String = ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_default();
    ui.horizontal(|ui| {
        // The field commits on Enter (which also uses the address) or when it loses focus.
        let mut chosen = false;
        if let Some(typed) = widgets::text_field(ui, egui::Id::new("live_server_address"), &draft, 200.0, tr("Server address")) {
            draft = typed;
            chosen = ui.input(|i| i.key_pressed(egui::Key::Enter));
        }
        chosen |= widgets::button(ui, tr("Use this server"), true, true).clicked();
        if chosen && !draft.trim().is_empty() {
            app.run("pref.publishServer", json!({"url": draft.trim()}));
        }
    });
    ui.data_mut(|d| d.insert_temp(id, draft));
    None
}
