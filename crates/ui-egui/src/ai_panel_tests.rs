//! Fix T4: the AI tab is a chat; every connector is in Settings ▸ Agents.
//! - The AI tab has no connector row, status block or warning; every row of the old AI tab
//!   (captured before the change, `docs/design/ai-panel-old-ai-tab.txt`) is in Settings ▸ Agents,
//!   and their commands are in the registry.
//! - `ui.inspect` › `agent.aiPanel` has the header, the popover (search, scope, list, check), the
//!   empty state and the composer; the Follow agents button toggles and shows paused.
//! - A new chat is made on its first send and kept on this computer (never in the document or
//!   Diagnostics); search finds titles and messages; another document's conversation is read-only.
//! - The selection button adds names and ids; the image button only for an agent that takes one;
//!   the grace hint's line only while pending, and it dismisses.
//! - At the panel's minimum width nothing is clipped or scrolls sideways.
//!
//! (⌘↩ and Esc: `chat_ui_tests.rs` › `command_return_sends_and_escape_stops`; a missing CLI's
//! line: `with_no_agent_installed_it_says_so_with_a_link_to_settings_agents`.)

use std::sync::{Arc, Mutex};

use egui::accesskit::{Node, Role};
use serde_json::{Value, json};

use crate::follow_agent::chat_ui::tests::{CLAUDE, Platform, Script, Shared, window};
use crate::layers_drag_tests::Ui;
use crate::layers_header::tests::{click_named, tree};

fn panel(u: &Ui) -> Value {
    crate::follow_agent::inspect(&u.app)["aiPanel"].clone()
}

/// The labelled nodes inside the AI pane (the right panel), as `role\tlabel`.
fn in_ai_pane(nodes: &[Node]) -> Vec<String> {
    let pane = nodes.iter().find(|n| n.role() == Role::Pane && n.label() == Some("AI")).and_then(|n| n.bounds()).expect("the AI pane");
    nodes
        .iter()
        .filter(|n| n.bounds().is_some_and(|b| b.x0 >= pane.x0 - 1.0 && b.x1 <= pane.x1 + 1.0))
        .filter_map(|n| n.label().or(n.value()).filter(|l| !l.is_empty()).map(|l| format!("{:?}\t{l}", n.role())))
        .collect()
}

/// The old AI tab's rows that moved to Settings ▸ Agents.
fn moved_rows() -> Vec<String> {
    let page = include_str!("../../../docs/design/ai-panel-old-ai-tab.txt");
    let start = page.find("[agents]").expect("the section") + "[agents]".len();
    let end = page[start..].find('[').map_or(page.len(), |i| start + i);
    page[start..end].lines().map(str::trim_end).filter(|l| l.contains('\t')).map(str::to_string).collect()
}

#[test]
fn the_ai_tab_has_no_connector_row_status_block_or_warning() {
    let (mut u, _) = window();
    u.app.ui.agent_view.grace_hint.shown = true; // the hint was said, and dismissed: not pending
    let rows = in_ai_pane(&tree(&mut u));
    for old in moved_rows() {
        assert!(!rows.contains(&old), "{old:?} is still in the AI tab");
    }
    for gone in [
        "Agents",
        "Not connected",
        "Connect Claude Code",
        "Connect other agents",
        "The indicator ended before the agent's next step. A longer setting keeps it on.",
    ] {
        assert!(!rows.iter().any(|r| r.ends_with(&format!("\t{gone}"))), "{gone:?} in {rows:?}");
    }
    assert!(!rows.iter().any(|r| r.contains("Open the setting")), "no warning button");
}

#[test]
fn every_row_and_command_of_the_old_ai_tab_is_in_settings_agents() {
    // As captured: the endpoint up, Claude Code installed, Connect other agents › More opened.
    let (mut u, _) = window();
    u.app.run("settings.open", json!({"section": "agents"}));
    u.app.handle_ui_requests();
    u.frame(vec![]);
    let mut seen = std::collections::BTreeSet::new();
    let mut opened_more = false;
    for _ in 0..60 {
        let nodes = tree(&mut u);
        let mut more = None;
        for n in &nodes {
            if let Some(l) = n.label().or(n.value()).filter(|l| !l.is_empty()) {
                seen.insert(format!("{:?}\t{l}", n.role()));
                if l.starts_with("More (") && n.role() == Role::Button {
                    more = n.bounds().map(|b| egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32));
                }
            }
        }
        if let (false, Some(p)) = (opened_more, more.filter(|p| p.y > 160.0 && p.y < 680.0)) {
            opened_more = true;
            crate::follow_agent::tests::click(&mut u, p);
            continue;
        }
        u.frame(vec![egui::Event::PointerMoved(egui::pos2(800.0, 450.0))]);
        u.frame(vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -120.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        }]);
    }
    let missing: Vec<String> = moved_rows().into_iter().filter(|r| !seen.contains(r)).collect();
    assert!(missing.is_empty(), "missing from Settings ▸ Agents: {missing:?}");
    // Their commands: the same registry ids.
    for id in [
        "mcp.connect-claude-code",
        "mcp.connect-client",
        "mcp.write-client",
        "mcp.reregister-client",
        "mcp.copy-client-entry",
        "mcp.copy-agent-snippet",
        "mcp.copy-address",
        "mcp.restart-server",
        "mcp.connect-claude-desktop",
        "mcp.write-claude-desktop",
        "chat.grace-setting",
    ] {
        assert!(figcraft_engine::find_command(id).is_some(), "{id} is in the registry");
    }
}

#[test]
fn ui_inspect_has_the_header_popover_empty_state_and_composer() {
    let (mut u, _) = window();
    let p = panel(&u);
    assert_eq!(p["header"]["title"], "New chat");
    assert_eq!(
        (p["header"]["chevron"].clone(), p["header"]["follow"].clone(), p["header"]["newChat"].clone()),
        (json!(true), json!("following"), json!("New chat"))
    );
    assert_eq!(p["emptyState"], "Describe what you want to create or change.");
    let c = &p["composer"];
    assert_eq!(c["placeholder"], "Describe a change…");
    assert_eq!(c["selectionButton"], "Add the selection to the prompt");
    assert_eq!(c["settingsButton"], "Agent settings");
    assert_eq!((c["agent"].clone(), c["send"].clone(), c["imageButton"].clone()), (json!("Claude Code"), json!("Send"), json!(false)));
    assert!(p["popover"].is_null(), "closed");
    // The chevron opens the popover.
    click_named(&mut u, "Conversation: New chat");
    let pop = panel(&u)["popover"].clone();
    assert_eq!(pop["searchPlaceholder"], "Search conversations…");
    assert_eq!(pop["scopes"], json!(["This document", "All documents"]));
    assert_eq!(pop["scope"], "document");
    assert_eq!(pop["items"], json!([]));
    // Every icon button has its name (and so its tooltip).
    let names = tree(&mut u).into_iter().filter_map(|n| n.label().map(str::to_string)).collect::<Vec<_>>();
    for b in ["Follow agents as they work", "New chat", "Add the selection to the prompt", "Agent settings"] {
        assert!(names.iter().any(|n| n == b), "{b:?} in {names:?}");
    }
    assert!(names.iter().any(|n| n.starts_with("Send")), "Send");
}

#[test]
fn the_follow_button_toggles_and_shows_paused() {
    let (mut u, _) = window();
    click_named(&mut u, "Follow agents as they work");
    assert!(!u.app.session.prefs.settings.follow_agents, "off");
    assert_eq!(panel(&u)["header"]["follow"], "off");
    click_named(&mut u, "Follow agents as they work");
    assert!(u.app.session.prefs.settings.follow_agents, "on");
    // An agent at work, then the person's own zoom: paused.
    let ctx = u.ctx.clone();
    crate::control::handle_route(
        &mut u.app,
        &ctx,
        "mcp (http)",
        "engine.execute",
        &json!({"command": "frame.create", "params": {"x": 3000, "y": 0, "width": 100, "height": 100}}),
    )
    .unwrap();
    crate::follow_agent::tests::frames(&mut u, 40);
    u.app.run("view.zoomIn", json!({}));
    crate::follow_agent::tests::frames(&mut u, 3);
    assert_eq!(panel(&u)["header"]["follow"], "paused");
    click_named(&mut u, "Follow agents as they work: paused, press to resume");
    assert_eq!(figcraft_engine::cmd::camera::follow_state(&u.app.session), "following", "resumed");
}

#[test]
fn a_new_chat_is_made_on_its_first_send_and_kept_on_this_computer() {
    let (mut u, script) = window();
    u.app.run("chat.new", json!({}));
    u.frame(vec![]);
    assert!(u.app.ui.agent_view.chat.store.list.is_empty(), "nothing made before a send");
    assert_eq!(script.lock().unwrap().saves, 0);
    crate::follow_agent::chat_ui::tests::send_allowed(&mut u, "Make a blue poster");
    u.frame(vec![]);
    let st = &u.app.ui.agent_view.chat.store;
    assert_eq!(st.list.len(), 1);
    assert_eq!(st.list[0].title, "Make a blue poster");
    assert_eq!(st.current.as_deref(), Some(st.list[0].id.as_str()));
    assert_eq!(panel(&u)["header"]["title"], "Make a blue poster");
    let kept = script.lock().unwrap().conversations.clone().expect("kept");
    assert!(kept.contains("Make a blue poster"), "{kept}");
    // Never in the document, nor in Diagnostics.
    assert!(!serde_json::to_string(u.app.session.doc().unwrap()).unwrap().contains("blue poster"));
    let diag = u.app.session.prefs.settings.diagnostics.iter().map(|e| e.detail.clone()).collect::<Vec<_>>().join("\n");
    assert!(!diag.contains("blue poster"), "{diag}");
    // A new chat: empty, the first kept.
    u.app.run("chat.cancel", json!({}));
    u.app.ui.agent_view.chat.running = None;
    // The next frame tells the engine the run is over (Fix U3).
    u.frame(vec![]);
    u.app.run("chat.new", json!({}));
    u.frame(vec![]);
    assert_eq!(panel(&u)["header"]["title"], "New chat");
    assert!(u.app.ui.agent_view.chat.lines.is_empty());
    assert_eq!(u.app.ui.agent_view.chat.store.list.len(), 1);
}

/// Two conversations of this document and one of another, kept as the app keeps them.
fn with_conversations() -> (Ui, Shared, String) {
    let (mut u, script) = window();
    let here = crate::follow_agent::chat_ui::store::doc_key(&u.app).unwrap().0;
    let kept = json!([
        {"id": "c1", "title": "Blue poster", "doc": here, "doc_name": "Untitled", "updated": 10, "lines": [{"kind": "You", "text": "Blue poster"}]},
        {"id": "c2", "title": "Make a card", "doc": here, "doc_name": "Untitled", "updated": 20, "lines": [
            {"kind": "You", "text": "Make a card"},
            {"kind": "Reply", "text": {"agent": "Claude Code", "text": "The card has a red border."}}
        ]},
        {"id": "c3", "title": "Landing page", "doc": "/Users/someone/Documents/Scrim/site.figcraft", "doc_name": "site", "updated": 30, "lines": [{"kind": "You", "text": "Landing page"}]}
    ]);
    script.lock().unwrap().conversations = Some(kept.to_string());
    // Read afresh, as at the app's start.
    u.app.ui.agent_view.chat.store = Default::default();
    u.frame(vec![]);
    (u, script, here)
}

#[test]
fn search_finds_titles_and_messages_and_the_scope_shows_other_documents() {
    let (mut u, _, _) = with_conversations();
    u.app.ui.agent_view.chat.store.open = true;
    u.frame(vec![]);
    let titles =
        |u: &Ui| panel(u)["popover"]["items"].as_array().unwrap().iter().map(|i| i["title"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    assert_eq!(titles(&u), ["Make a card", "Blue poster"], "this document's, newest first");
    u.app.ui.agent_view.chat.store.query = "RED BORDER".into();
    u.frame(vec![]);
    assert_eq!(titles(&u), ["Make a card"], "a message's words, any case");
    u.app.ui.agent_view.chat.store.query = "poster".into();
    u.frame(vec![]);
    assert_eq!(titles(&u), ["Blue poster"], "a title's");
    u.app.ui.agent_view.chat.store.query.clear();
    u.app.run("chat.scope", json!({"scope": "all"}));
    u.frame(vec![]);
    let items = panel(&u)["popover"]["items"].clone();
    assert_eq!(items[0]["title"], "Landing page");
    assert_eq!((items[0]["readOnly"].clone(), items[0]["document"].clone()), (json!(true), json!("site")));
}

#[test]
fn another_documents_conversation_opens_read_only() {
    let (mut u, script, _) = with_conversations();
    u.app.run("chat.open", json!({"id": "c3"}));
    u.frame(vec![]);
    let p = panel(&u);
    assert_eq!(p["header"]["title"], "Landing page");
    assert_eq!((p["readOnly"].clone(), p["composer"]["readOnly"].clone()), (json!(true), json!(true)));
    assert_eq!(p["composer"]["status"], "Open that document to continue");
    u.app.run("chat.send", json!({"prompt": "more", "agent": "claude-code"}));
    u.frame(vec![]);
    assert!(script.lock().unwrap().specs.is_empty() && u.app.ui.agent_view.chat.confirm.is_none(), "no run from another document");
    // Its own document's: the composer works again.
    u.app.run("chat.open", json!({"id": "c2"}));
    u.frame(vec![]);
    assert_eq!(panel(&u)["composer"]["readOnly"], false);
    assert_eq!(u.app.ui.agent_view.chat.lines.len(), 2, "its transcript");
}

#[test]
fn the_selection_button_adds_the_layers_names_and_ids() {
    let (mut u, _) = window();
    let poster = u.app.session.selection()[0];
    click_named(&mut u, "Add the selection to the prompt");
    assert_eq!(u.app.ui.agent_view.chat.draft, format!("Selected: Poster ({})", poster.0));
}

#[test]
fn the_image_button_shows_only_for_an_agent_whose_cli_takes_one() {
    let mut s = figcraft_engine::Session::new();
    s.agents.mcp.port = Some(7690);
    let mut u = Ui::new(s);
    let script: Shared =
        Arc::new(Mutex::new(Script { claude: Some(CLAUDE.into()), codex: Some("/opt/agents/bin/codex".into()), ..Default::default() }));
    u.app.services = Box::new(Platform(script.clone()));
    u.app.run("panel.tab.ai", json!({}));
    u.frame(vec![]);
    let names = |u: &mut Ui| tree(u).into_iter().filter_map(|n| n.label().map(str::to_string)).collect::<Vec<_>>();
    assert_eq!(panel(&u)["composer"]["imageButton"], false, "Claude Code runs with no tools that open a path");
    assert!(!names(&mut u).iter().any(|n| n == "Attach an image"));
    u.app.run("chat.agent", json!({"agent": "codex"}));
    u.frame(vec![]);
    assert_eq!(panel(&u)["composer"]["imageButton"], true, "Codex takes --image");
    assert!(names(&mut u).iter().any(|n| n == "Attach an image"));
    // Attached, it goes with the send as a path.
    u.app.run("chat.attach-image", json!({"path": "/tmp/ref.png"}));
    u.frame(vec![]);
    assert_eq!(panel(&u)["composer"]["image"], "ref.png");
    u.app.run("chat.allow-agent", json!({"agent": "codex", "path": "/opt/agents/bin/codex"}));
    u.app.ui.agent_view.chat.confirm = None;
    u.app.run("chat.send", json!({"prompt": "use this", "agent": "codex", "image": "/tmp/ref.png"}));
    u.frame(vec![]);
    let spec = script.lock().unwrap().specs.pop().expect("it ran");
    assert_eq!(&spec.args[..3], ["exec", "--image", "/tmp/ref.png"]);
}

#[test]
fn the_grace_hint_line_shows_only_while_pending_and_dismisses() {
    let (mut u, _) = window();
    assert!(panel(&u)["composer"]["hintLine"].is_null(), "not pending: no line");
    let h = &mut u.app.ui.agent_view.grace_hint;
    (h.shown, h.showing) = (true, true);
    u.frame(vec![]);
    assert_eq!(panel(&u)["composer"]["hintLine"], true);
    click_named(&mut u, "Dismiss");
    assert!(!u.app.ui.agent_view.grace_hint.showing);
    assert!(panel(&u)["composer"]["hintLine"].is_null(), "dismissed");
    // Its link opens the setting.
    u.app.ui.agent_view.grace_hint.showing = true;
    u.frame(vec![]);
    click_named(&mut u, "Open the setting");
    u.app.handle_ui_requests();
    assert!(u.app.ui.preferences && u.app.ui.settings_section == "ai");
}

#[test]
fn at_its_minimum_width_nothing_is_clipped_or_scrolls_sideways() {
    for w in [*crate::RIGHT_PANEL.start(), crate::PANEL_DEFAULT, 320.0] {
        let (mut u, _) = window();
        u.app.ui.right_width = w;
        u.app.ui.agent_view.chat.store.open = true;
        u.frame(vec![]);
        let nodes = tree(&mut u);
        let pane = nodes.iter().find(|n| n.role() == Role::Pane && n.label() == Some("AI")).and_then(|n| n.bounds()).expect("the AI pane");
        for name in ["Follow agents as they work", "New chat", "Add the selection to the prompt", "Agent settings", "Search conversations"] {
            let b = nodes.iter().find(|n| n.label() == Some(name)).and_then(|n| n.bounds()).unwrap_or_else(|| panic!("{name} at {w}"));
            assert!(b.x0 >= pane.x0 - 0.5 && b.x1 <= pane.x1 + 0.5, "{name} inside the {w} px panel: {b:?} in {pane:?}");
        }
        let send = nodes.iter().find(|n| n.label().is_some_and(|l| l.starts_with("Send"))).and_then(|n| n.bounds()).expect("Send");
        assert!(send.x1 <= pane.x1 + 0.5, "Send at {w}");
        let p = panel(&u);
        assert!(p["widest"].as_f64().unwrap() <= p["width"].as_f64().unwrap() + 1.0, "no sideways scroll at {w}: {p}");
    }
}

/// Fix U3: while a composer run goes, New chat is dimmed with the disabled token, can't be clicked
/// or reached by Tab, says "Stop the run first", and `chat.new` (and `chat.clear`) refuse with the
/// same words; `ui.inspect` reports it. Once the run ends it works again.
#[test]
fn new_chat_is_disabled_while_a_run_goes_and_says_stop_the_run_first() {
    let (mut u, script) = window();
    assert_eq!(panel(&u)["header"]["newChatEnabled"], true);
    crate::follow_agent::chat_ui::tests::send_allowed(&mut u, "Make a blue poster");
    u.frame(vec![]);
    assert!(u.app.ui.agent_view.chat.running.is_some(), "a run under way");
    let header = &panel(&u)["header"];
    assert_eq!((header["newChatEnabled"].clone(), header["newChatTooltip"].clone()), (json!(false), json!("Stop the run first")));
    // Dimmed: drawn in the disabled token, never the icon colour.
    let t = crate::theme::Tokens::get(&u.ctx);
    assert_eq!(header["newChatColor"], json!(format!("{:?}", t.disabled)));
    // Not clickable, not focusable, disabled for screen readers.
    let nodes = tree(&mut u);
    let plus = nodes.iter().find(|n| n.label() == Some("New chat")).expect("the New chat button");
    assert!(plus.is_disabled(), "disabled");
    assert!(!plus.supports_action(egui::accesskit::Action::Focus), "Tab skips it");
    assert!(!plus.supports_action(egui::accesskit::Action::Click), "no click");
    click_named(&mut u, "New chat");
    assert_eq!(panel(&u)["header"]["title"], "Make a blue poster", "the conversation kept");
    // The command refuses with the same words, whoever runs it.
    for id in ["chat.new", "chat.clear"] {
        let e = u.app.session.execute(id, &json!({})).expect_err("refused during the run");
        assert!(e.to_string().contains("Stop the run first"), "{id}: {e}");
    }
    // The run ends: New chat works again.
    crate::follow_agent::chat_ui::tests::feed(
        &script,
        vec![crate::services::AgentRunEvent::Ended { end: crate::services::AgentEnd::Exited(Some(0)), stderr: vec![], secs: 1.0 }],
    );
    u.frame(vec![]);
    u.frame(vec![]);
    assert!(u.app.ui.agent_view.chat.running.is_none());
    assert_eq!(panel(&u)["header"]["newChatEnabled"], true);
    let nodes = tree(&mut u);
    let plus = nodes.iter().find(|n| n.label() == Some("New chat")).expect("the New chat button");
    assert!(!plus.is_disabled() && plus.supports_action(egui::accesskit::Action::Focus));
    click_named(&mut u, "New chat");
    assert_eq!(panel(&u)["header"]["title"], "New chat");
}

/// Fix V2: the AI panel header's Follow agents button and Settings ▸ Agents' toggle always show the
/// same state, whichever the person uses, and after a restart (the settings as saved, read back).
#[test]
fn the_follow_button_and_the_settings_toggle_always_agree_and_survive_a_restart() {
    use egui::accesskit::Toggled;
    const NAME: &str = "Follow agents as they work";
    let pressed = |n: &Node| n.toggled().map(|t| t == Toggled::True);
    let centre = |n: &Node| n.bounds().map(|b| egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32)).unwrap();
    // The header button's state (Settings closed).
    let button = |u: &mut Ui| {
        let nodes = tree(u);
        let b = nodes.iter().find(|n| n.role() == Role::Button && n.label() == Some(NAME)).expect("the header button");
        (pressed(b), centre(b))
    };
    // Settings ▸ Agents' toggle, scrolled into view.
    let toggle = |u: &mut Ui| {
        u.app.run("settings.open", json!({"section": "agents"}));
        u.app.handle_ui_requests();
        u.frame(vec![]);
        for _ in 0..40 {
            let nodes = tree(u);
            if let Some(c) = nodes.iter().find(|n| n.role() == Role::CheckBox && n.label() == Some(NAME))
                && (160.0..680.0).contains(&centre(c).y)
            {
                return (pressed(c), centre(c));
            }
            u.frame(vec![egui::Event::PointerMoved(egui::pos2(800.0, 450.0))]);
            u.frame(vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -120.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            }]);
        }
        panic!("the toggle never came into view");
    };
    let close = |u: &mut Ui| click_named(u, "Done");
    let state = |u: &Ui| figcraft_engine::cmd::camera::follow_state(&u.app.session);
    let (mut u, _) = window();
    assert_eq!(button(&mut u).0, Some(true));
    assert_eq!(toggle(&mut u).0, Some(true));
    close(&mut u);
    // The header button turns it off: the toggle says off.
    let (_, at) = button(&mut u);
    crate::follow_agent::tests::click(&mut u, at);
    assert_eq!(state(&u), "off");
    let (on, at) = toggle(&mut u);
    assert_eq!(on, Some(false), "the toggle agrees with the button");
    // The toggle turns it on: the button says on.
    crate::follow_agent::tests::click(&mut u, at);
    assert_eq!(state(&u), "following");
    assert_eq!(toggle(&mut u).0, Some(true));
    close(&mut u);
    assert_eq!(button(&mut u).0, Some(true), "the button agrees with the toggle");
    // Off again from the toggle, then a restart: the saved settings, read as the app reads them.
    let (_, at) = toggle(&mut u);
    crate::follow_agent::tests::click(&mut u, at);
    close(&mut u);
    assert_eq!(button(&mut u).0, Some(false));
    let saved = serde_json::to_value(&u.app.session.prefs).unwrap();
    let (mut again, _) = window();
    again.app.session.prefs = serde_json::from_value(saved).unwrap();
    again.app.session.prefs.settings = std::mem::take(&mut again.app.session.prefs.settings).sanitized();
    again.frame(vec![]);
    assert_eq!(state(&again), "off");
    assert_eq!(button(&mut again).0, Some(false), "the button after a restart");
    assert_eq!(toggle(&mut again).0, Some(false), "the toggle after a restart");
}

/// Fix V3, restyled by Fix X1: off reads plainly (the crosshair in the icon colour on the rest fill,
/// no accent; the tooltip "Following is off. Click to follow agents as they work"; `ui.inspect`'s
/// `followSetting`; X2's status line says so too), and the
/// header button's and Settings toggle's changes are noted with their origin. The reveal's "follow
/// off" line is unchanged (`reveal_skip_tests.rs`).
#[test]
fn following_off_is_plain_in_the_header_and_each_change_is_noted_with_its_origin() {
    const NAME: &str = "Follow agents as they work";
    let last = |u: &Ui| u.app.session.prefs.settings.diagnostics.last().map(|e| e.detail.clone()).unwrap_or_default();
    let (mut u, _) = window();
    let h = panel(&u)["header"].clone();
    assert_eq!((h["followSetting"].clone(), h["followTooltip"].clone()), (json!("ON"), json!(NAME)));
    // Off from the header button.
    let nodes = tree(&mut u);
    let b = nodes.iter().find(|n| n.role() == Role::Button && n.label() == Some(NAME)).unwrap().bounds().unwrap();
    crate::follow_agent::tests::click(&mut u, egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32));
    assert_eq!(last(&u), "follow setting: ON → OFF (made by the person's button)");
    u.frame(vec![]);
    let h = panel(&u)["header"].clone();
    assert_eq!(h["follow"], "off");
    assert_eq!(h["followSetting"], "OFF");
    assert_eq!(h["followTooltip"], "Following is off. Click to follow agents as they work");
    // Drawn off: the crosshair in the icon colour on the rest fill, nothing in the accent (Fix X1).
    let t = crate::theme::Tokens::get(&u.ctx);
    let r = egui::Rect::from_min_max(egui::pos2(b.x0 as f32, b.y0 as f32), egui::pos2(b.x1 as f32, b.y1 as f32));
    let out = u.frame_output(vec![]);
    let mut colours = std::collections::HashSet::new();
    for p in u.ctx.tessellate(out.shapes, out.pixels_per_point) {
        if let egui::epaint::Primitive::Mesh(m) = p.primitive {
            colours.extend(m.vertices.iter().filter(|v| r.shrink(2.0).contains(v.pos) && v.color.a() == 255).map(|v| v.color));
        }
    }
    assert!(colours.contains(&t.icon) && !colours.contains(&t.accent), "{colours:?}");
    // Its name stays the same for screen readers, not pressed.
    let nodes = tree(&mut u);
    let n = nodes.iter().find(|n| n.role() == Role::Button && n.label() == Some(NAME)).unwrap();
    assert_eq!(n.toggled(), Some(egui::accesskit::Toggled::False));
    // On from Settings ▸ Agents' toggle: noted as the toggle's.
    u.app.run("camera.follow-agents", json!({"on": true, "origin": "settings"}));
    assert_eq!(last(&u), "follow setting: OFF → ON (made by the person's Settings toggle)");
    u.frame(vec![]);
    assert_eq!(panel(&u)["header"]["followSetting"], "ON");
}

/// Fix W4: a setting changed in Settings is noted as the person's Settings; the same command from
/// elsewhere (the palette, a menu) as the person's command.
#[test]
fn a_setting_changed_in_settings_is_noted_as_the_persons_settings() {
    const NAME: &str = "Show working indicator";
    let last = |u: &Ui| u.app.session.prefs.settings.diagnostics.last().map(|e| e.detail.clone()).unwrap_or_default();
    let (mut u, _) = window();
    u.app.run("settings.open", json!({"section": "agents"}));
    u.app.handle_ui_requests();
    u.frame(vec![]);
    let find = |u: &mut Ui| {
        let nodes = tree(u);
        nodes.iter().filter(|n| n.role() == Role::CheckBox && n.label() == Some(NAME)).find_map(|n| {
            let b = n.bounds()?;
            let p = egui::pos2(((b.x0 + b.x1) / 2.0) as f32, ((b.y0 + b.y1) / 2.0) as f32);
            (200.0..600.0).contains(&p.y).then_some(p)
        })
    };
    for _ in 0..40 {
        if find(&mut u).is_some() {
            break;
        }
        u.frame(vec![egui::Event::PointerMoved(egui::pos2(800.0, 450.0))]);
        u.frame(vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -120.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        }]);
    }
    // Let the scroll settle, then read where the toggle is.
    crate::follow_agent::tests::frames(&mut u, 30);
    let at = find(&mut u);
    crate::follow_agent::tests::click(&mut u, at.expect("the toggle in view"));
    assert_eq!(last(&u), "setting working_indicator: true → false (made by the person's Settings)");
    assert!(!u.app.session.agents.from_settings, "only while Settings draws");
    u.app.run("pref.workingIndicator", json!({"on": true}));
    assert_eq!(last(&u), "setting working_indicator: false → true (made by the person's command)");
}
