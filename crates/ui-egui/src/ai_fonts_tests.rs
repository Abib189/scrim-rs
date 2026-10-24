//! Fix AA1: the AI panel's SemiBold and italic roles draw in real Inter SemiBold and Inter Italic,
//! its regular roles in Inter Regular, in the test harness as in the app (the same
//! `theme::install_fonts`), with no role falling back to another face.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::follow_agent::chat_ui::Line;
use crate::follow_agent::chat_ui::tests::{feed, send_allowed, window};
use crate::layers_drag_tests::Ui;
use crate::theme::{INTER_KEYS, ITALIC, SEMIBOLD, regular, semibold};

/// The roles the design draws in SemiBold, in Regular, and in italic.
const SEMIBOLD_ROLES: [&str; 7] = ["title", "emptyHeading", "agentName", "cardHeading", "activeRow", "replyBold", "replyHeading"];
const REGULAR_ROLES: [&str; 12] =
    ["emptyLine", "banner", "bubble", "cardNote", "cardButton", "row", "reply", "note", "field", "placeholder", "chip", "status"];

fn fonts(u: &Ui) -> BTreeMap<String, Value> {
    let f = crate::follow_agent::inspect(&u.app)["aiPanel"]["fonts"].clone();
    f.as_object().map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default()
}

/// Every text role the panel draws, gathered over its states: empty, mid-run (the banner, your
/// message, the card with an active row) and ended (the agent's Markdown reply, a note).
fn every_role() -> (Ui, BTreeMap<String, Value>) {
    use crate::follow_agent::tests::agent;
    use crate::services::{AgentEnd, AgentRunEvent};
    let (mut u, script) = window();
    let mut all = fonts(&u);
    send_allowed(&mut u, "Build a poster called Welle");
    u.frame(vec![]);
    let f = agent(&mut u, "frame.create", json!({"x": 400, "y": 0, "width": 300, "height": 200, "name": "Poster"}));
    agent(&mut u, "text.create", json!({"text": "Welle", "parent": f["id"]}));
    // The steps open, so the active row is drawn (Fix AB2: closed at first).
    u.app.run("chat.toggle-run-steps", json!({}));
    u.frame(vec![]);
    all.extend(fonts(&u));
    feed(&script, vec![AgentRunEvent::Ended { end: AgentEnd::Exited(Some(0)), stderr: vec![], secs: 42.0 }]);
    u.frame(vec![]);
    u.app
        .ui
        .agent_view
        .chat
        .lines
        .push(Line::Reply { agent: "Claude Code".into(), text: "# Welle\n**Bold**, *italic*, `code` and text.\n- a bullet".into() });
    u.frame(vec![]);
    u.frame(vec![]);
    all.extend(fonts(&u));
    (u, all)
}

/// The face a family draws with first (its font key).
fn first_face(u: &Ui, family: &str) -> String {
    let fam = match family {
        "proportional" => egui::FontFamily::Proportional,
        "monospace" => egui::FontFamily::Monospace,
        other => egui::FontFamily::Name(other.into()),
    };
    u.ctx.fonts(|f| f.definitions().families.get(&fam).and_then(|l| l.first().cloned())).unwrap_or_default()
}

/// Every SemiBold role uses the SemiBold family, led by Inter SemiBold; a Regular role never does
/// and is led by Inter Regular; Markdown italic uses the italic family, led by Inter Italic.
#[test]
fn each_role_draws_in_its_own_inter_face() {
    let (u, all) = every_role();
    let family = |role: &str| all.get(role).unwrap_or_else(|| panic!("{role} not drawn: {all:?}"))["family"].as_str().unwrap_or_default().to_string();
    for role in SEMIBOLD_ROLES {
        assert_eq!(family(role), SEMIBOLD, "{role}");
        assert_eq!(first_face(&u, &family(role)), INTER_KEYS[1], "{role}: Inter SemiBold, not a fallback");
    }
    for role in REGULAR_ROLES {
        assert_eq!(family(role), "proportional", "{role}");
        assert_eq!(first_face(&u, &family(role)), INTER_KEYS[0], "{role}: Inter Regular");
    }
    assert_eq!(family("replyItalic"), ITALIC);
    assert_eq!(first_face(&u, ITALIC), INTER_KEYS[2], "Inter Italic");
    assert_eq!(family("replyCode"), "monospace", "code stays monospace");
}

/// The weights are real: the same words lay out wider in SemiBold than in Regular, and the italic
/// family is Inter's italic file. With the regular face under the SemiBold name (as before Fix
/// AA1) the widths would be equal.
#[test]
fn semibold_and_italic_are_real_faces() {
    let (u, _) = every_role();
    // At 100 px, so pixel rounding doesn't hide the difference (Inter SemiBold is about 3% wider).
    let width = |f: egui::FontId| u.ctx.fonts_mut(|fs| fs.layout_no_wrap("mmmmmmmmmmWWWWWWWWWW".into(), f, egui::Color32::WHITE).size().x);
    let (r, sb) = (width(regular(100.0)), width(semibold(100.0)));
    assert!(sb > r * 1.015, "SemiBold {sb} is wider than Regular {r}");
    // Italic has Regular's widths: its face is the italic file itself.
    let italic_file = figcraft_text::CRAFT_FONTS
        .iter()
        .chain(figcraft_text::bundled::BUNDLED_FONTS)
        .find(|f| f.family == "Inter" && f.style == "Italic")
        .map(|f| f.bytes.as_ptr());
    let face = u.ctx.fonts(|f| f.definitions().font_data.get(INTER_KEYS[2]).map(|d| d.font.as_ptr()));
    assert!(face.is_some() && face == italic_file, "the italic family is Inter's italic file");
    // The bundled variable font is set at 600 for SemiBold (unless craft-fonts supplies a static face).
    let coords = u.ctx.fonts(|f| f.definitions().font_data.get(INTER_KEYS[1]).map(|d| d.tweak.coords.as_ref().to_vec()));
    let coords = coords.unwrap_or_default();
    if figcraft_text::CRAFT_FONTS.iter().all(|f| f.family != "Inter") {
        assert_eq!(coords.iter().map(|(_, v)| *v).collect::<Vec<_>>(), [600.0], "wght 600");
    }
}

/// The test harness has the app's font set: the same faces under the same families as a fresh
/// `install_fonts`, the call the app makes on its first frame.
#[test]
fn the_harness_font_set_is_the_apps() {
    let (u, _) = window();
    let app = egui::Context::default();
    crate::theme::install_fonts(&app, false, 0);
    app.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
    let heads = |c: &egui::Context| {
        c.fonts(|f| {
            let d = f.definitions();
            [egui::FontFamily::Proportional, egui::FontFamily::Name(SEMIBOLD.into()), egui::FontFamily::Name(ITALIC.into())]
                .map(|fam| d.families.get(&fam).and_then(|l| l.first().cloned()).unwrap_or_default())
        })
    };
    assert_eq!(heads(&u.ctx), heads(&app));
    assert_eq!(heads(&app), INTER_KEYS.map(str::to_string), "Inter Regular, SemiBold and Italic lead");
}

/// No text the panel draws falls back past Inter: every character of its words is in Inter (its
/// italic face for the italic words).
#[test]
fn no_role_falls_back_to_another_face() {
    let (u, _) = every_role();
    let p = crate::follow_agent::inspect(&u.app)["aiPanel"].clone();
    let mut words: Vec<String> = vec![p["header"]["titleShown"].as_str().unwrap_or_default().into()];
    for k in ["heading", "note"] {
        words.push(p["card"][k].as_str().unwrap_or_default().into());
    }
    for r in p["card"]["rows"].as_array().cloned().unwrap_or_default() {
        words.push(r["shown"].as_str().unwrap_or_default().into());
    }
    for b in p["replies"][0]["blocks"].as_array().cloned().unwrap_or_default().iter().filter(|b| b["kind"] != "code") {
        words.push(b["text"].as_str().unwrap_or_default().into());
    }
    words.extend(
        ["Describe what you want to create or change.", "Select layers", "Undo run", "Following is on", "Describe a change…"].map(String::from),
    );
    // The faces they're drawn in are Inter's, not a fallback that happens to have the glyphs.
    for (family, key) in [("proportional", INTER_KEYS[0]), (SEMIBOLD, INTER_KEYS[1]), (ITALIC, INTER_KEYS[2])] {
        assert_eq!(first_face(&u, family), key, "{family}");
    }
    let regular = figcraft_text::find_face("Inter", "Regular").expect("Inter");
    for w in &words {
        assert!(!w.is_empty(), "{words:?}");
        for c in w.chars().filter(|c| !c.is_whitespace()) {
            assert!(regular.has_char(c), "{c:?} of {w:?} isn't in Inter");
            assert!(u.ctx.fonts_mut(|f| f.has_glyph(&semibold(13.0), c)), "{c:?} in SemiBold");
        }
    }
    let it = figcraft_text::find_face("Inter", "Italic").expect("Inter Italic");
    assert!("italic".chars().all(|c| it.has_char(c)));
}
