//! The Code tab (`docs/design/panel-redesign.md`, B1–B2), and the code section it shares with the
//! Inspect tab: the selected layer's code from the existing generators (`codegen::CodeLang`), in px
//! or rem, with Copy, and Copy standalone HTML. Every control runs a command (`code.setFormat`,
//! `code.copy`, `code.copyStandaloneHtml`), so agents switch and copy the same way. Reading code
//! never changes the document.

use egui::{CornerRadius, Ui};
use figcraft_doc::{Document, NodeId};
use figcraft_engine::codegen::CodeLang;
use serde_json::{Value, json};

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
    row(ui, 32.0, |ui| {
        let resp = widgets::button(ui, tr("Copy standalone HTML"), false, true)
            .on_hover_text(tr("A complete web page for this layer: its styles and images inside, nothing fetched from elsewhere"));
        if resp.clicked() {
            app.run("code.copyStandaloneHtml", json!({"id": first.0}));
        }
    });
}

/// Act on a Code tab request from a command; `false` when `kind` isn't one.
pub fn request(app: &mut FigApp, kind: &str, data: &Value) -> bool {
    match kind {
        figcraft_engine::cmd::code::FORMAT_REQUEST => {
            app.ui.dev.code_lang = data["format"].as_str().and_then(CodeLang::from_id).unwrap_or_default();
            app.ui.dev.code_rem = data["unit"] == "rem";
        }
        figcraft_engine::cmd::code::COPY_REQUEST => {
            app.services.clipboard_set_text(data["text"].as_str().unwrap_or_default());
            let what = match data["what"].as_str().unwrap_or_default() {
                "standalone HTML" => tr("standalone HTML"),
                "the connect command" => tr("the connect command"),
                "the font names" => tr("the font names"),
                other => other,
            };
            app.toast(trf("Copied {value}", &[("value", &what)]));
        }
        _ => return false,
    }
    true
}

/// The code of `first`: the language and unit menus, the code, and Copy. Returns the language.
pub fn section(app: &mut FigApp, ui: &mut Ui, doc: &Document, first: NodeId) -> CodeLang {
    let t = Tokens::get(ui.ctx());
    let lang = app.ui.dev.code_lang;
    // A component tried with other property values: its code replaces the layer's below.
    let played = crate::dev_mode::playground_section(app, ui, doc, first, lang);
    widgets::separator(ui);
    let copy = widgets::section_header(ui, tr("Code"), &[(Icon::Export, tr("Copy"))], false).is_some();
    // Pixels or rem (of a 16 px root) for the web languages.
    let web = lang.is_web();
    let rem = web && app.ui.dev.code_rem;
    let unit = if rem { "rem" } else { "px" };
    row(ui, 32.0, |ui| {
        let resp = widgets::dropdown(ui, lang.label(), 120.0, Some(Icon::Code));
        menu_popup(&resp).show(|ui| {
            for l in CodeLang::ALL {
                if menu_item(ui, l.label(), "") {
                    app.run("code.setFormat", json!({"format": l.id(), "unit": if l.is_web() { unit } else { "px" }}));
                    ui.close();
                }
            }
        });
        if web {
            let resp = widgets::dropdown(ui, unit, 56.0, None);
            menu_popup(&resp).show(|ui| {
                for label in ["px", "rem"] {
                    if menu_item(ui, label, "") {
                        app.run("code.setFormat", json!({"format": lang.id(), "unit": label}));
                        ui.close();
                    }
                }
            });
        }
    });
    let code = played.clone().unwrap_or_else(|| crate::dev_mode::code(app, ui, first, lang));
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
        if played.is_some() {
            // A playground variant is the UI's own preview: copied as shown.
            app.services.clipboard_set_text(&code);
            app.toast(trf("Copied {value}", &[("value", &lang.label())]));
        } else {
            app.run("code.copy", json!({"id": first.0, "language": lang.id(), "unit": unit}));
        }
    }
    lang
}

#[cfg(test)]
#[path = "code_tab_tests.rs"]
mod tests;
