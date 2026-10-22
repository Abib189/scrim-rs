//! Settings ▸ General (C3): appearance (theme, accent), animations, your name in shared rooms,
//! keeping unsaved work, snapping, then the rows the Preferences window had (editing, files,
//! sharing, language). Every control runs a registry command.

use egui::Ui;
use serde_json::json;

use crate::i18n::{self, tr, tr_noop, trf};
use crate::settings_dialog::heading;
use crate::theme::Tokens;
use crate::{FigApp, widgets};

pub fn show(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let prefs = app.session.prefs.clone();

    heading(ui, tr("Appearance"));
    ui.horizontal(|ui| {
        ui.label(tr("Theme"));
        // The tab bar's sun and moon run the same command (`pref.theme`).
        let themes = [("system", tr_noop("Auto")), ("light", tr_noop("Light")), ("dark", tr_noop("Dark"))];
        let labels: Vec<&str> = themes.iter().map(|(_, l)| tr(l)).collect();
        let current = themes.iter().position(|(id, _)| prefs.theme.eq_ignore_ascii_case(id)).unwrap_or(0);
        if let Some(i) = widgets::segmented_text(ui, &labels, current) {
            app.run("pref.theme", json!({"theme": themes[i].0}));
        }
    });
    ui.add_space(6.0);
    crate::dialogs::appearance_prefs::row(app, ui);

    heading(ui, tr("Animations"));
    ui.horizontal(|ui| {
        let modes = [("system", tr_noop("Follow system")), ("on", tr_noop("On")), ("off", tr_noop("Off"))];
        let labels: Vec<&str> = modes.iter().map(|(_, l)| tr(l)).collect();
        let current = modes.iter().position(|(id, _)| prefs.settings.motion == *id).unwrap_or(0);
        if let Some(i) = widgets::segmented_text(ui, &labels, current) {
            app.run("pref.motion", json!({"motion": modes[i].0}));
        }
    });
    if prefs.settings.motion == "system" && app.services.reduce_motion() {
        widgets::label(ui, tr("Your system asks for reduced motion: animations stay off."), t.text_secondary);
    }

    heading(ui, tr("Agents at work"));
    if widgets::checkbox(ui, prefs.settings.follow_agents, tr("Follow agents as they work")) {
        app.run("camera.follow-agents", json!({"on": !prefs.settings.follow_agents}));
    }
    widgets::label(ui, tr("The camera goes to an agent's edits as they land. Your own scroll, zoom or edit pauses it."), t.text_secondary);
    if widgets::checkbox(ui, prefs.settings.reveal_agent_work, tr("Reveal agent work")) {
        app.run("pref.revealAgentWork", json!({}));
    }
    // Each speed with its timing (Fix M3), one a row: too long for a segmented control.
    ui.label(tr("Reveal speed"));
    let speeds = [
        ("fast", tr_noop("Fast (0.2 s per layer, about 1 s per step)")),
        ("normal", tr_noop("Normal (0.5 s per layer, about 4 s per step)")),
        ("slow", tr_noop("Slow (0.7 s per layer, about 6 s per step)")),
    ];
    let current = speeds.iter().position(|(id, _)| prefs.settings.reveal_speed == *id).unwrap_or(1);
    for (i, (id, label)) in speeds.iter().enumerate() {
        if widgets::radio(ui, i == current, tr(label)) && i != current {
            app.run("pref.revealSpeed", json!({"speed": id}));
        }
    }
    if widgets::checkbox(ui, prefs.settings.working_indicator, tr("Show working indicator")) {
        app.run("pref.workingIndicator", json!({}));
    }
    widgets::label(
        ui,
        tr("New layers fade in as an agent adds them, and dots above the artboard show it is working. Animations Off shows them still."),
        t.text_secondary,
    );
    // The grace (Fix Q): 15, 30 (the default), 60 or 120 s. The person's choice is kept through
    // migrations; Fix Q2's hint scrolls here.
    let grace_row = ui.label(tr("Keep the working indicator after the last agent request")).rect;
    ui.horizontal(|ui| {
        use figcraft_engine::settings::{AGENT_GRACE_SECS, DEFAULT_AGENT_GRACE_SECS};
        let labels: Vec<String> =
            AGENT_GRACE_SECS
                .iter()
                .map(|s| {
                    if *s == DEFAULT_AGENT_GRACE_SECS {
                        trf("{seconds} s (default)", &[("seconds", s)])
                    } else {
                        trf("{seconds} s", &[("seconds", s)])
                    }
                })
                .collect();
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let current = AGENT_GRACE_SECS.iter().position(|s| *s == prefs.settings.agent_grace_secs).unwrap_or(1);
        if let Some(i) = widgets::segmented_text(ui, &labels, current) {
            app.run("pref.agentGrace", json!({"seconds": AGENT_GRACE_SECS[i]}));
        }
    });
    if std::mem::take(&mut app.ui.agent_view.focus_grace) {
        ui.scroll_to_rect(grace_row, Some(egui::Align::Center));
    }
    widgets::label(ui, tr("An agent that says it has finished, or disconnects, stops showing as working at once."), t.text_secondary);

    heading(ui, tr("Shared rooms"));
    ui.horizontal(|ui| {
        ui.label(tr("Your name"));
        if let Some(name) = widgets::text_field(ui, egui::Id::new("pref_user_name"), &prefs.user_name, 200.0, tr("You")) {
            app.run("pref.userName", json!({"name": name}));
        }
    });
    widgets::label(ui, tr("Shown beside your cursor to the others in a shared file, and on your comments."), t.text_secondary);

    let mut toggles: Vec<(&str, serde_json::Value)> = Vec::new();
    // The browser keeps no files: no recovery copies or saving there.
    if !cfg!(target_arch = "wasm32") {
        heading(ui, tr("Unsaved work"));
        if widgets::checkbox(ui, prefs.settings.preserve_unsaved, tr("Keep recovery copies of unsaved work")) {
            toggles.push(("pref.preserveUnsaved", json!({})));
        }
        let note = if prefs.settings.preserve_unsaved {
            tr("If Scrim quits unexpectedly, your unsaved files open again the next time.")
        } else {
            tr("Off: if Scrim quits unexpectedly, unsaved changes are lost. Copies kept already stay.")
        };
        widgets::label(ui, note, if prefs.settings.preserve_unsaved { t.text_secondary } else { t.warning });
        if widgets::checkbox(ui, prefs.auto_save, tr("Save files automatically")) {
            toggles.push(("pref.autoSave", json!({})));
        }
    }

    heading(ui, tr("Snapping"));
    let view = app.session.ds().map(|d| d.view.clone()).ok();
    let snap_objects = view.as_ref().is_none_or(|v| v.snap_objects);
    let snap_pixel = view.as_ref().is_some_and(|v| v.snap_pixel);
    if widgets::checkbox(ui, snap_objects, tr("Snap to objects")) {
        toggles.push(("view.snapObjects", json!({})));
    }
    if widgets::checkbox(ui, snap_pixel, tr("Snap to pixel grid")) {
        toggles.push(("view.snapPixel", json!({})));
    }

    heading(ui, tr("Editing"));
    for (label, val, cmd) in [
        (tr("Keep tool selected after use"), prefs.keep_tool, "pref.keepTool"),
        (tr("Highlight layers on hover"), prefs.highlight_on_hover, "pref.highlightHover"),
        (tr("Rename duplicated layers"), prefs.rename_duplicates, "pref.renameDuplicates"),
        (tr("Show dimensions on objects"), prefs.show_dimensions, "pref.showDimensions"),
        (tr("Use number keys for opacity"), prefs.number_keys_opacity, "pref.numberKeysOpacity"),
    ] {
        if widgets::checkbox(ui, val, label) {
            toggles.push((cmd, json!({})));
        }
    }
    ui.horizontal(|ui| {
        ui.label(tr("Nudge amount"));
        let mut small = prefs.nudge_small;
        let mut big = prefs.nudge_big;
        let a = crate::theme::add_input(ui, egui::DragValue::new(&mut small).range(0.1..=100.0));
        let b = crate::theme::add_input(ui, egui::DragValue::new(&mut big).range(1.0..=1000.0));
        if a.changed() || b.changed() {
            app.run("pref.nudge", json!({"small": small, "big": big}));
        }
    });
    for (c, p) in toggles {
        app.run(c, p);
    }

    heading(ui, tr("Sharing"));
    ui.horizontal(|ui| {
        ui.label(tr("Server for online sharing"));
        // Where Share ▸ Online keeps files and join codes are asked; no key here (preferences are
        // saved as text).
        let name = tr("Server for online sharing");
        if let Some(url) = widgets::text_field_named(ui, egui::Id::new("pref_publish_server"), &prefs.publish_server, 220.0, name) {
            app.run("pref.publishServer", json!({"url": url}));
        }
        let usual = prefs.publish_server == figcraft_engine::live::DEFAULT_SERVER;
        if widgets::button(ui, tr("Reset"), false, !usual).on_hover_text(tr("Back to the default: no server, unless this build names one")).clicked()
        {
            app.run("pref.publishServer", json!({}));
        }
    });
    if prefs.publish_server.is_empty() {
        widgets::label(
            ui,
            tr("No server is set, so nothing is uploaded. Share ▸ Online asks for one first; the local network needs none."),
            t.text_secondary,
        );
    }
    // Where the app registers itself as the opener of invite links (Windows).
    if let Some(on) = app.session.invite_links_on() {
        if widgets::checkbox(ui, on, tr("Open invite links in Scrim")) {
            app.run("collab.registerLinks", json!({"on": !on}));
        }
        widgets::label(ui, tr("figcraft:// links from a browser, chat or email open this app and join the session."), t.text_secondary);
    }

    heading(ui, tr("Language"));
    ui.horizontal(|ui| {
        // Each language by its own name; System follows the OS (the browser on the web).
        let langs = || {
            [("system", tr("System"))].into_iter().chain(i18n::languages().iter().filter(|l| crate::cjk::can_show(l.code)).map(|l| (l.code, l.name)))
        };
        let on = |code: &str| app.session.prefs.language.eq_ignore_ascii_case(code);
        let selected = langs().find(|(code, _)| on(code)).map_or(tr("System"), |(_, name)| name);
        let resp = widgets::dropdown(ui, selected, 220.0, None);
        let mut pick = None;
        crate::right::menu_popup(&resp).show(|ui| {
            for (code, name) in langs() {
                if crate::right::menu_check(ui, on(code), name, "") {
                    pick = Some(code);
                    ui.close();
                }
            }
        });
        if let Some(code) = pick {
            app.run("pref.language", json!({"language": code}));
            ui.ctx().request_repaint();
        }
    });
    if let Some(lang) = i18n::current().filter(|l| l.draft) {
        let note = trf("{language}: draft translation, waiting for a native speaker's review.", &[("language", &lang.name)]);
        widgets::label(ui, &note, t.text_secondary);
    }
}
