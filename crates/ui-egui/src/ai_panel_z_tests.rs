//! Fix Z: the AI panel's header title (Z3), the transcript's fade (Z4), the composer's icon chips
//! (Z6) and the run card after Undo run (Z7).

use egui::accesskit::Role;
use serde_json::Value;

use crate::follow_agent::chat_ui::tests::{send_allowed, window};
use crate::layers_drag_tests::Ui;
use crate::layers_header::tests::tree;

fn panel(u: &Ui) -> Value {
    crate::follow_agent::inspect(&u.app)["aiPanel"].clone()
}

fn rect(v: &Value) -> [f64; 4] {
    let a: Vec<f64> = v.as_array().unwrap_or_else(|| panic!("a rectangle: {v}")).iter().map(|x| x.as_f64().unwrap()).collect();
    [a[0], a[1], a[2], a[3]]
}

fn themed(dark: bool, w: f32) -> Ui {
    let (mut u, _) = window();
    u.app.session.prefs.theme = if dark { "dark" } else { "light" }.into();
    u.app.ui.right_width = w;
    u.frame(vec![]);
    u
}

/// Z3: a 48-character title is cut with "…" where it doesn't fit; the chevron sits 6 after what
/// shows; Follow and New chat keep 8 apart and 16 from the right; the whole title is the tooltip and
/// in the accessible name. At 240, 320 and 400 px, light and dark.
#[test]
fn a_long_title_is_cut_with_an_ellipsis_and_the_chevron_follows_it() {
    let prompt = "Build a Swiss poster series for the Welle music festival in Hamburg";
    for dark in [false, true] {
        for w in [240.0, 320.0, 400.0] {
            let mut u = themed(dark, w);
            send_allowed(&mut u, prompt);
            u.frame(vec![]);
            let h = panel(&u)["header"].clone();
            let title = h["title"].as_str().unwrap().to_string();
            assert_eq!(title.chars().count(), 48, "{title}");
            let shown = h["titleShown"].as_str().unwrap_or_else(|| panic!("{dark} {w}: titleShown: {h}"));
            assert!(shown.ends_with('…') && title.starts_with(shown.trim_end_matches('…')), "{dark} {w}: {shown:?} of {title:?}");
            let (text, chevron) = (rect(&h["titleRect"]), rect(&h["chevronRect"]));
            assert!((chevron[0] - text[2] - 6.0).abs() < 0.6, "{dark} {w}: the chevron 6 after the text: {text:?} {chevron:?}");
            let (follow, plus) = (rect(&h["followRect"]), rect(&panel(&u)["newChatRect"]));
            assert!(chevron[2] <= follow[0] - 8.0, "{dark} {w}: the chevron clear of Follow: {chevron:?} {follow:?}");
            assert!((plus[0] - follow[2] - 8.0).abs() < 0.6, "{dark} {w}: the buttons 8 apart");
            let nodes = tree(&mut u);
            let pane = nodes.iter().find(|n| n.role() == Role::Pane && n.label() == Some("AI")).and_then(|n| n.bounds()).expect("the AI pane");
            assert!((pane.x1 - plus[2] - 16.0).abs() < 0.6, "{dark} {w}: New chat 16 from the right: {plus:?} in {pane:?}");
            let name = format!("Conversation: {title}");
            assert!(nodes.iter().any(|n| n.label() == Some(name.as_str())), "{dark} {w}: the accessible name has the whole title");
            // Hovered, the tooltip says the whole title.
            let at = egui::pos2(((text[0] + text[2]) / 2.0) as f32, ((text[1] + text[3]) / 2.0) as f32);
            u.frame(vec![egui::Event::PointerMoved(at)]);
            for _ in 0..60 {
                u.frame(vec![]);
            }
            let nodes = tree(&mut u);
            assert!(nodes.iter().any(|n| n.label() == Some(title.as_str()) || n.value() == Some(title.as_str())), "{dark} {w}: the tooltip");
        }
    }
}
