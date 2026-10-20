//! Settings ▸ Privacy and Diagnostics (C3d): the statement, the events the app records (a failed
//! command, a revert, the MCP server and clients, a font fallback with family names only), and the
//! section's Copy, Clear and retention.

use egui::accesskit::{Node, Role};
use egui::{Event, Modifiers, PointerButton, pos2};
use figcraft_engine::Session;
use serde_json::json;

use crate::layers_drag_tests::Ui;

fn tree(u: &mut Ui) -> Vec<Node> {
    u.ctx.enable_accesskit();
    u.frame(vec![]);
    let mut out = u.frame_output(vec![]);
    out.textures_delta.clear();
    out.platform_output.accesskit_update.expect("an AccessKit update").nodes.into_iter().map(|(_, n)| n).collect()
}

fn texts(u: &mut Ui) -> String {
    u.frame(vec![]);
    let shapes = u.frame_output(vec![]).shapes;
    crate::import_report_tests::shown_texts(shapes).join("\n")
}

fn click(u: &mut Ui, n: &Node) {
    let b = n.bounds().expect("bounds");
    let p = pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32);
    u.frame(vec![Event::PointerMoved(p)]);
    u.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    u.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
    u.frame(vec![]);
}

fn kinds(u: &Ui) -> Vec<(String, String)> {
    u.app.session.prefs.settings.diagnostics.iter().map(|e| (e.kind.clone(), e.detail.clone())).collect()
}

#[test]
fn privacy_says_there_is_no_telemetry() {
    let mut u = Ui::new(Session::new());
    u.app.run("settings.open", json!({"section": "privacy"}));
    let shown = texts(&mut u);
    assert!(shown.contains("Scrim sends no telemetry"), "{shown}");
    assert!(shown.contains("127.0.0.1"));
}

#[test]
fn the_app_records_failures_by_id_and_category_only() {
    let mut u = Ui::new(Session::new());
    u.app.run("text.set", json!({"ids": [999_999], "text": "PRIVATE WORDS"}));
    u.app.run("no.such.command", json!({}));
    let k = kinds(&u);
    assert_eq!(k.len(), 2, "{k:?}");
    assert!(k[0].0 == "error" && k[0].1.starts_with("text.set: "), "{k:?}");
    assert_eq!(k[1].1, "a command: unknown command");
    assert!(!format!("{k:?}").contains("PRIVATE"));
    // A revert is a reload.
    crate::settings_privacy::noted(&mut u.app, "file.revert", None);
    assert_eq!(kinds(&u).last().map(|e| e.0.as_str()), Some("reload"));
}

#[test]
fn the_mcp_server_and_its_clients_are_recorded() {
    let mut u = Ui::new(Session::new());
    // The test services have no endpoint: it can't start.
    crate::mcp_ui::restart(&mut u.app);
    crate::mcp_ui::client_seen(&mut u.app, &json!({"name": "claude-code", "version": "2.1.296", "now": 1})).unwrap();
    let k = kinds(&u);
    assert_eq!(k[0], ("mcpServer".to_string(), "couldn't start".to_string()));
    assert_eq!(k[1], ("mcpClient".to_string(), "claude-code 2.1.296 connected".to_string()));
}

#[test]
fn a_font_fallback_is_recorded_with_family_names_only() {
    figcraft_text::scan_system_fonts();
    if !figcraft_text::installed_fonts_known() {
        eprintln!("skipped: system fonts opted out");
        return;
    }
    let mut s = Session::new();
    s.execute("text.create", &json!({"text": "PRIVATE TEXT", "x": 0, "y": 0, "props": {"fontFamily": "Scrim Missing Diag"}})).unwrap();
    let path = std::env::temp_dir().join(format!("scrim-diag-fonts-{}.figcraft", std::process::id()));
    let path = path.to_string_lossy().into_owned();
    s.execute("file.saveAs", &json!({"path": path})).unwrap();
    let mut u = Ui::new(Session::new());
    u.app.run_user("file.open", json!({"path": path}));
    let _ = std::fs::remove_file(&path);
    u.frame(vec![]);
    let k = kinds(&u);
    let fonts: Vec<_> = k.iter().filter(|e| e.0 == "fontFallback").collect();
    assert_eq!(fonts.len(), 1, "{k:?}");
    assert_eq!(fonts[0].1, "fonts not on this computer, a fallback used: Scrim Missing Diag");
    assert!(!format!("{k:?}").contains("PRIVATE") && !format!("{k:?}").contains("scrim-diag-fonts"));
}

#[test]
fn the_diagnostics_section_lists_copies_and_clears() {
    let mut u = Ui::new(Session::new());
    u.app.run("no.such.command", json!({}));
    u.app.run("settings.open", json!({"section": "diagnostics"}));
    let shown = texts(&mut u);
    assert!(shown.contains("a command: unknown command") && shown.contains("one event"), "{shown}");
    let nodes = tree(&mut u);
    let find =
        |label: &str| nodes.iter().find(|n| n.role() == Role::Button && n.label() == Some(label)).cloned().unwrap_or_else(|| panic!("{label}"));
    let (copy, clear) = (find("Copy"), find("Clear"));
    click(&mut u, &copy);
    assert_eq!(u.app.session.prefs.settings.diagnostics.len(), 1, "Copy ran without a failure of its own");
    click(&mut u, &clear);
    assert!(u.app.session.prefs.settings.diagnostics.is_empty());
    assert!(texts(&mut u).contains("Nothing yet."));
}
