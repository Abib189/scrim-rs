//! The Code tab (`docs/design/panel-redesign.md`, B1–B2), and the code section it shares with the
//! Inspect tab: the selected layer's code from the existing generators (`codegen::CodeLang`), in px
//! or rem, with Copy. Reading code never changes the document.

use egui::{CornerRadius, Id, Ui};
use figcraft_doc::{Document, NodeId};
use figcraft_engine::codegen::CodeLang;

use crate::FigApp;
use crate::i18n::{tr, trf};
use crate::icons::Icon;
use crate::right::{menu_item, menu_popup, row};
use crate::theme::Tokens;
use crate::widgets;

/// The Code tab: the first selected layer's code, or what to do without a selection.
pub fn tab(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let Ok(ds) = app.session.ds() else { return };
    let doc = ds.doc.clone();
    let Some(first) = ds.selection.first().copied() else {
        widgets::section_header(ui, tr("Code"), &[], false);
        widgets::note(ui, tr("Select a layer to see its code"), t.text_secondary);
        return;
    };
    let name = doc.node(first).map(|n| n.name.clone()).unwrap_or_default();
    widgets::section_header(ui, &name, &[], false);
    section(app, ui, &doc, first);
}

/// The language chosen (shared by the Code and Inspect tabs, so switching keeps it).
pub fn lang(ui: &Ui) -> CodeLang {
    ui.data(|d| d.get_temp::<usize>(Id::new("inspect_lang"))).and_then(|i| CodeLang::ALL.get(i).copied()).unwrap_or_default()
}

/// The code of `first`: the language and unit menus, the code, and Copy. Returns the language.
pub fn section(app: &mut FigApp, ui: &mut Ui, doc: &Document, first: NodeId) -> CodeLang {
    let t = Tokens::get(ui.ctx());
    let lang_id = Id::new("inspect_lang");
    let lang = lang(ui);
    // A component tried with other property values: its code replaces the layer's below.
    let played = crate::dev_mode::playground_section(app, ui, doc, first, lang);
    widgets::separator(ui);
    let copy = widgets::section_header(ui, tr("Code"), &[(Icon::Export, tr("Copy"))], false).is_some();
    // Pixels or rem (of a 16 px root) for the web languages.
    let rem_id = Id::new("inspect_rem");
    let web = lang.is_web();
    let rem = web && ui.data(|d| d.get_temp::<bool>(rem_id)).unwrap_or(false);
    row(ui, 32.0, |ui| {
        let resp = widgets::dropdown(ui, lang.label(), 120.0, Some(Icon::Code));
        menu_popup(&resp).show(|ui| {
            for (i, l) in CodeLang::ALL.iter().enumerate() {
                if menu_item(ui, l.label(), "") {
                    ui.data_mut(|d| d.insert_temp(lang_id, i));
                    ui.close();
                }
            }
        });
        if web {
            let resp = widgets::dropdown(ui, if rem { "rem" } else { "px" }, 56.0, None);
            menu_popup(&resp).show(|ui| {
                for (on, label) in [(false, "px"), (true, "rem")] {
                    if menu_item(ui, label, "") {
                        ui.data_mut(|d| d.insert_temp(rem_id, on));
                        ui.close();
                    }
                }
            });
        }
    });
    let code = played.unwrap_or_else(|| crate::dev_mode::code(app, ui, first, lang));
    let code = if rem { figcraft_engine::codegen::px_to_rem(&code, 16.0) } else { code };
    egui::Frame::NONE
        .fill(t.field)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::same(10))
        .outer_margin(egui::Margin::symmetric(16, 0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(&code).monospace().color(t.text));
        });
    ui.add_space(8.0);
    let mut copy_clicked = copy;
    row(ui, 32.0, |ui| {
        copy_clicked |= ui.button(trf("Copy {value}", &[("value", &lang.label())])).clicked();
    });
    if copy_clicked {
        app.services.clipboard_set_text(&code);
        app.toast(trf("Copied {value}", &[("value", &lang.label())]));
    }
    lang
}
