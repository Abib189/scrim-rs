//! The right panel's mode switch and tabs (B1): clicks run the same commands agents run, Animate is
//! a placeholder that never touches the document, the choice is remembered, and screen readers get
//! every control by name.

use egui::accesskit::{Node, Role, Toggled};
use figcraft_engine::Session;
use figcraft_engine::cmd::panel::{Mode, PanelView, Tab};
use serde_json::json;

use crate::RightTab;
use crate::layers_drag_tests::Ui;
use crate::live_tests::{Harness, has_text, text_center};

#[test]
fn clicks_switch_tabs_and_modes_and_never_touch_the_document() {
    let (mut h, rect) = Harness::new();
    h.app.session.execute("select.set", &json!({"ids": [rect]})).unwrap();
    let (doc, steps) = (h.app.session.doc().unwrap().clone(), h.app.session.undo_depth());
    let shapes = h.frames(3);
    for t in ["Design", "Animate", "Prototype", "Code", "AI"] {
        assert!(has_text(&shapes, t), "the header shows {t:?}");
    }
    h.click(text_center(&shapes, "Code"));
    let shapes = h.frames(2);
    assert_eq!(h.app.ui.right_tab, RightTab::Code);
    assert_eq!(h.app.session.prefs.panel, PanelView { mode: Mode::Design, tab: Tab::Code });
    assert!(has_text(&shapes, "Copy CSS"), "the selected layer's code");
    h.click(text_center(&shapes, "AI"));
    let shapes = h.frames(2);
    assert_eq!(h.app.ui.right_tab, RightTab::Ai);
    assert!(has_text(&shapes, "Agents") && has_text(&shapes, "Activity"), "the AI tab's sections");
    // Animate: a placeholder, without the tabs; back to Design comes back to the AI tab.
    h.click(text_center(&shapes, "Animate"));
    let shapes = h.frames(2);
    assert!(h.app.ui.panel_animate);
    assert!(has_text(&shapes, "Animation comes in a later stage. This mode is a placeholder: nothing here changes your file."));
    assert!(!has_text(&shapes, "Prototype") && !has_text(&shapes, "Code"), "no tabs in Animate mode");
    h.click(text_center(&shapes, "Back to Design"));
    let shapes = h.frames(2);
    assert!(!h.app.ui.panel_animate && h.app.ui.right_tab == RightTab::Ai);
    h.click(text_center(&shapes, "Prototype"));
    h.frames(2);
    assert_eq!(h.app.ui.right_tab, RightTab::Prototype);
    assert!(h.app.session.doc().unwrap() == &doc && h.app.session.undo_depth() == steps, "switching is never an edit");
}

#[test]
fn agents_switch_them_with_the_same_commands() {
    let (mut h, _) = Harness::new();
    h.frames(2);
    let ctx = h.ctx.clone();
    let run = |h: &mut Harness, cmd: &str| crate::control::handle(&mut h.app, &ctx, "engine.execute", &json!({"command": cmd})).unwrap();
    assert_eq!(run(&mut h, "panel.tab.code"), json!({"mode": "design", "tab": "code"}));
    assert_eq!(h.app.ui.right_tab, RightTab::Code);
    assert_eq!(run(&mut h, "panel.mode.animate"), json!({"mode": "animate", "tab": "code"}));
    let inspect = crate::control::handle(&mut h.app, &ctx, "ui.inspect", &json!({})).unwrap();
    assert_eq!((&inspect["panelMode"], &inspect["rightTab"]), (&json!("animate"), &json!("Code")));
    run(&mut h, "panel.tab.ai");
    assert!(!h.app.ui.panel_animate && h.app.ui.right_tab == RightTab::Ai);
    // The older panel commands keep working and are remembered too.
    run(&mut h, "view.prototype");
    assert_eq!((h.app.ui.right_tab, h.app.session.prefs.panel.tab), (RightTab::Prototype, Tab::Prototype));
    // Dev Mode isn't a tab: it leaves the last choice alone, and toggles back to Design.
    run(&mut h, "view.devMode");
    assert_eq!((h.app.ui.right_tab, h.app.session.prefs.panel.tab), (RightTab::Inspect, Tab::Prototype));
    run(&mut h, "view.devMode");
    assert_eq!(h.app.ui.right_tab, RightTab::Design);
}

#[test]
fn the_next_start_opens_with_the_last_choice() {
    let mut s = Session::new();
    s.prefs.panel = PanelView { mode: Mode::Animate, tab: Tab::Code };
    let u = Ui::new(s);
    assert!(u.app.ui.panel_animate && u.app.ui.right_tab == RightTab::Code);
    let mut s = Session::new();
    s.prefs.panel = PanelView { mode: Mode::Design, tab: Tab::Ai };
    let u = Ui::new(s);
    assert!(!u.app.ui.panel_animate && u.app.ui.right_tab == RightTab::Ai);
}

fn tree(u: &mut Ui) -> Vec<Node> {
    u.ctx.enable_accesskit();
    u.frame(vec![]);
    let out = u.frame_output(vec![]);
    out.platform_output.accesskit_update.expect("an AccessKit update").nodes.into_iter().map(|(_, n)| n).collect()
}

fn find<'a>(nodes: &'a [Node], role: Role, label: &str) -> &'a Node {
    nodes.iter().find(|n| n.role() == role && n.label() == Some(label)).unwrap_or_else(|| panic!("no {role:?} named {label:?}"))
}

#[test]
fn screen_readers_get_the_mode_switch_and_the_tabs() {
    let mut u = Ui::new(Session::new());
    u.app.session.execute("panel.tab.code", &json!({})).unwrap();
    u.app.handle_ui_requests();
    let nodes = tree(&mut u);
    assert_eq!(find(&nodes, Role::Button, "Design mode").toggled(), Some(Toggled::True));
    assert_eq!(find(&nodes, Role::Button, "Animate mode").toggled(), Some(Toggled::False));
    for (tab, selected) in [("Design", false), ("Prototype", false), ("Code", true), ("AI", false)] {
        assert_eq!(find(&nodes, Role::Tab, tab).is_selected(), Some(selected), "{tab}");
    }
    find(&nodes, Role::Pane, "Code");
    u.app.session.execute("panel.mode.animate", &json!({})).unwrap();
    u.app.handle_ui_requests();
    let nodes = tree(&mut u);
    assert_eq!(find(&nodes, Role::Button, "Animate mode").toggled(), Some(Toggled::True));
    find(&nodes, Role::Pane, "Animate");
    assert!(!nodes.iter().any(|n| n.role() == Role::Tab && n.label() == Some("Prototype")), "no tabs in Animate mode");
}

/// B5: every text and mark the redesign draws reads at AA (4.5:1) on what it's drawn on, for each
/// accent preset, in both themes: the mode switch and tab labels (on the panel, the field and the
/// active fill), the accent edge, the AI feed's agent names, badges and notes, and the sample.
#[test]
fn the_new_panels_text_and_marks_read_at_aa() {
    use crate::theme::Tokens;
    use crate::theme::accent::contrast;
    for (_, c) in figcraft_engine::cmd::appearance::PRESETS {
        for base in [Tokens::dark(), Tokens::light()] {
            let t = base.with_accent(egui::Color32::from_rgb(c[0], c[1], c[2]));
            let pairs = [
                ("label on the panel", t.text, t.panel),
                ("secondary label on the panel", t.text_secondary, t.panel),
                ("tertiary note on the panel", t.text_tertiary, t.panel),
                ("active tab label on its fill", t.text, t.hover),
                ("inactive tab label on hover", t.text_secondary, t.hover),
                ("mode segment on the field", t.text_secondary, t.field),
                ("active mode segment", t.text, t.panel),
                ("badge count on the field", t.text, t.field),
                ("agent name (accent) on the panel", t.accent, t.panel),
                ("agent name (accent) on hover", t.accent, t.hover),
                ("accent edge on the panel", t.accent, t.panel),
                ("label on the accent", t.accent_on, t.accent_fill),
            ];
            for (what, fg, bg) in pairs {
                let r = contrast(fg, bg);
                assert!(r >= 4.5, "{c:?} dark={}: {what} is {r:.2}:1", t.dark);
            }
        }
    }
}
