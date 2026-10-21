//! Follow the agent (Stage I, `docs/design/follow-agent.md`): the camera goes to an agent's edits
//! as they land, and stays with them.
//! - **Whose edits:** the change log's entries whose author is an agent, the single source of truth
//!   (`agents::Change`). The person's edits never move the camera; they pause following.
//! - **Where:** the union of the changed layers on the page of the newest one; a new top-level frame
//!   whole; more than [`MANY`] layers, their top-level frames. The move is D2's: `camera.focus-layer
//!   {ids, fit}` (the least move, or a zoom out just enough to fit) and its flight, which jumps with
//!   Animations off or reduced motion.
//! - **One smooth move per burst:** the first edit flies at once; while edits keep coming, at most
//!   one new flight per [`FLIGHT_EVERY`], to the latest union, starting from where the camera is.
//! - **The person wins:** once an agent's work has been seen in the file, a press, scroll or pinch on
//!   the canvas, their own edit, or any camera move that isn't follow's or an agent's own pauses
//!   following (not off). Before any agent work there is nothing to pause. The paused chip, the AI
//!   tab's Resume or `camera.follow-agents {on: true}` resumes it and flies to the latest work.
//! - **Never mid-gesture:** with the pointer down, a text edit open, a canvas interaction or
//!   someone else's flight in progress, the move waits and goes after.
//! - Nothing here writes to the document: the camera is view state.
//!
//! The reveal of new layers ([`reveal`]) and the working indicator ([`working_indicator`]) read the
//! same edits; under load they degrade in a fixed order ([`Budget`]).

use std::collections::BTreeSet;
use std::sync::Arc;

use egui::{Context, CornerRadius, Id, Rect, Sense, Stroke, Ui, pos2, vec2};
use figcraft_doc::{Document, NodeId};
use figcraft_engine::cmd::camera::follow_state;
use serde_json::json;

use crate::FigApp;
use crate::i18n::{tr, trf};
use crate::theme::{Tokens, ink, regular, semibold};

#[path = "reveal.rs"]
pub mod reveal;
#[path = "working_indicator.rs"]
pub mod working_indicator;

/// While edits keep coming, at most one new flight this often (seconds).
pub const FLIGHT_EVERY: f64 = 0.4;
/// More changed layers than this in a burst frame their top-level frames instead.
pub const MANY: usize = 60;
/// Presses this close to the canvas's bottom edge are on the toolbar or the help button.
const TOOLBAR_BAND: f32 = 72.0;

/// An edit as the window saw it land: its layers created, changed and deleted since the document it
/// last saw.
#[derive(Clone, Debug, Default)]
pub struct Landed {
    pub agent: bool,
    /// The agent's route ("mcp (http)", "control channel").
    pub route: String,
    /// An undo or redo.
    pub undo: bool,
    pub created: Vec<NodeId>,
    pub changed: Vec<NodeId>,
    pub deleted: Vec<NodeId>,
}

/// The window's agent view: what it has seen of the change log, and following.
#[derive(Default)]
pub struct State {
    /// The document (its `uid`) and the newest logged revision read.
    seen: Option<(u64, u64)>,
    /// The document as last seen, to tell created, changed and deleted layers apart.
    last_doc: Option<Arc<Document>>,
    /// The document before this frame's new edits (the reveal compares moves with it).
    pub(crate) before: Option<Arc<Document>>,
    /// When the last frame began (the reveal and the indicator read the gap).
    pub(crate) last_frame: Option<f64>,
    /// Requests were answered while the window drew no frames (`control::drain_hidden`): what
    /// landed then has no entrance. An idle window also draws no frames, so the gap can't tell.
    pub(crate) landed_hidden: bool,
    follow: Follow,
    pub reveal: reveal::State,
    pub indicator: working_indicator::State,
    pub budget: Budget,
}

/// A frame's time budget (60 Hz) and the canvas render's (two frames): over either for three
/// frames in a row while something animates, the next step down is taken.
pub const FRAME_BUDGET_MS: f64 = 1000.0 / 60.0;
pub const RENDER_BUDGET_MS: f64 = 1000.0 / 30.0;

/// Degrading under load, in a fixed order, a step at a time and only while still over budget:
/// 1. the dot wave becomes the static row; 2. the outline's pulse stops; 3. the reveal's stagger is
///    capped at 0.6 s. The layer fade itself is never dropped for load (only for a hidden window,
///    badly dropped frames, the setting or reduced motion). Back to 0 after 2 s with nothing
///    animating.
#[derive(Default, Debug)]
pub struct Budget {
    pub level: u8,
    over: u8,
    stepped: f64,
    active_at: f64,
    /// The canvas render time last seen: only a new render counts (the number stays until then).
    last_render: f64,
    /// Tests pin the level (debug builds' frame times aren't the app's).
    pub(crate) frozen: bool,
}

fn budget(app: &mut FigApp, now: f64) {
    let active = app.ui.agent_view.reveal.active() || app.ui.agent_view.indicator.artboard().is_some();
    let b = &mut app.ui.agent_view.budget;
    let new_render = app.ui.render_ms != b.last_render;
    b.last_render = app.ui.render_ms;
    let over = app.ui.frame_ms > FRAME_BUDGET_MS || (new_render && app.ui.render_ms > RENDER_BUDGET_MS);
    if b.frozen {
        return;
    }
    if !active {
        b.over = 0;
        if now - b.active_at > 2.0 {
            b.level = 0;
        }
        return;
    }
    b.active_at = now;
    b.over = if over { b.over.saturating_add(1) } else { 0 };
    if b.over >= 3 && b.level < 3 && now - b.stepped >= 0.25 {
        b.level += 1;
        b.stepped = now;
        b.over = 0;
    }
}

#[derive(Default)]
struct Follow {
    /// The page and layers to go to next, gathered since the last flight.
    pending: Option<(NodeId, BTreeSet<NodeId>)>,
    /// The latest work (a resume goes there).
    latest: Option<(NodeId, BTreeSet<NodeId>)>,
    /// When the last flight started.
    last_flight: Option<f64>,
    /// The view follow (or an agent) left the camera at: document, page, centre and zoom. Anything
    /// else is the person's doing.
    expected: Option<(u64, NodeId, [f64; 3])>,
    /// A flight of follow's own is in progress.
    flying: bool,
    /// The state last frame (a change to `following` is a resume).
    was: &'static str,
    /// Who worked last, and the artboard (top-level frame) of their latest work.
    agent: String,
    artboard: Option<NodeId>,
    /// The paused chip, where it was drawn (a press on it isn't a press on the canvas).
    chip: Option<Rect>,
}

/// A person's name for the agent: Claude's MCP clients read "Claude", another client its own name,
/// anything else "An agent". Never hard-coded to one agent.
pub fn agent_name(app: &FigApp, route: &str) -> String {
    if route.starts_with("mcp")
        && let Some((name, _, _)) = app.session.agents.mcp.clients.last()
    {
        return if name.to_ascii_lowercase().contains("claude") { "Claude".into() } else { name.clone() };
    }
    tr("An agent").to_string()
}

/// The edits logged since last frame. Nothing new costs one comparison and allocates nothing.
fn scan(app: &mut FigApp) -> Vec<Landed> {
    let Ok(ds) = app.session.ds() else { return Vec::new() };
    let (uid, doc) = (ds.uid, ds.doc.clone());
    let newest = app.session.agents.last_edit(uid).unwrap_or(0);
    let st = &mut app.ui.agent_view;
    let since = match st.seen {
        Some((u, r)) if u == uid => r,
        // Another file (or the first frame): its history isn't new work.
        _ => {
            st.seen = Some((uid, newest));
            st.last_doc = Some(doc);
            st.follow = Follow { was: st.follow.was, ..Follow::default() };
            return Vec::new();
        }
    };
    if newest <= since {
        st.last_doc = Some(doc);
        return Vec::new();
    }
    let before = st.last_doc.replace(doc.clone()).unwrap_or_else(|| doc.clone());
    st.before = Some(before.clone());
    st.seen = Some((uid, newest));
    // A layer created by one of these edits is "changed" in the later ones that list it.
    let mut made: BTreeSet<NodeId> = BTreeSet::new();
    app.session
        .agents
        .edits_after(uid, since)
        .map(|c| {
            let mut l = Landed {
                agent: c.author.is_agent(),
                route: match &c.author {
                    figcraft_engine::agents::Author::Agent(r) => r.clone(),
                    figcraft_engine::agents::Author::Person => String::new(),
                },
                undo: matches!(c.command.as_str(), "edit.undo" | "edit.redo"),
                ..Landed::default()
            };
            for &id in &c.ids {
                match (before.contains(id), doc.contains(id)) {
                    (false, true) if made.insert(id) => l.created.push(id),
                    (false, true) | (true, true) => l.changed.push(id),
                    (true, false) => l.deleted.push(id),
                    (false, false) => {}
                }
            }
            l
        })
        .collect()
}

/// Where an agent's edits are: the page of the newest live layer, and the layers to frame there.
pub fn target(doc: &Document, landed: &[Landed]) -> Option<(NodeId, BTreeSet<NodeId>)> {
    let live = |id: &NodeId| doc.node(*id).is_some_and(|n| !n.is_page());
    let newest = landed.iter().rev().filter(|l| l.agent).flat_map(|l| l.created.iter().chain(&l.changed).rev()).find(|id| live(id))?;
    let page = doc.page_of(*newest)?;
    let ids: BTreeSet<NodeId> = landed
        .iter()
        .filter(|l| l.agent)
        .flat_map(|l| l.created.iter().chain(&l.changed))
        .copied()
        .filter(|id| live(id) && doc.page_of(*id) == Some(page))
        .collect();
    let created: BTreeSet<NodeId> = landed.iter().filter(|l| l.agent).flat_map(|l| l.created.iter().copied()).collect();
    let framed: BTreeSet<NodeId> = if ids.len() > MANY {
        // A big batch: the artboards, not every leaf.
        ids.iter().map(|id| doc.top_level_frame(*id).unwrap_or(*id)).collect()
    } else {
        // A new top-level frame whole; otherwise leave out layers listed only as the parent of
        // another changed layer (their children changed).
        ids.iter()
            .copied()
            .filter(|id| {
                (created.contains(id) && doc.top_level_frame(*id) == Some(*id)) || !ids.iter().any(|o| o != id && doc.ancestors(*o).contains(id))
            })
            .collect()
    };
    (!framed.is_empty()).then_some((page, framed))
}

/// Every frame, before the canvas: what's new in the log, then following.
pub fn before_canvas(app: &mut FigApp, ctx: &Context) {
    let now = ctx.input(|i| i.time);
    let gap = app.ui.agent_view.last_frame.replace(now).map_or(0.0, |t| (now - t).max(0.0));
    let landed = scan(app);
    follow(app, ctx, now, &landed);
    reveal::tick(app, ctx, now, gap, &landed);
    working_indicator::tick(app, ctx, now, gap, &landed);
    app.ui.agent_view.landed_hidden = false;
    budget(app, now);
}

/// Every frame, after the canvas: the paused chip, and the working indicator's AccessKit node.
pub fn after_canvas(app: &mut FigApp, ui: &mut Ui) {
    chip(app, ui);
    working_indicator::accessibility(app, ui);
}

/// "Claude is working on Poster": the working indicator's name for screen readers, from the same
/// agent and artboard as the AI tab's "Following …".
pub fn working_text(app: &FigApp) -> Option<String> {
    let id = app.ui.agent_view.indicator.artboard()?;
    let name = app.session.doc().ok()?.node(id)?.name.clone();
    let agent = &app.ui.agent_view.follow.agent;
    Some(trf("{agent} is working on {artboard}", &[("agent", agent), ("artboard", &name)]))
}

/// For `ui.inspect`: following, the reveal, the indicator and the degradation step.
pub fn inspect(app: &FigApp) -> serde_json::Value {
    let v = &app.ui.agent_view;
    json!({
        "follow": follow_state(&app.session),
        "revealing": v.reveal.entering().count(),
        "indicator": v.indicator.drawn.get().map(|(r, cols)| json!({"columns": cols, "rect": [r.min.x, r.min.y, r.max.x, r.max.y]})),
        "outline": v.indicator.outlined.get().is_some(),
        "working": working_text(app),
        "degraded": v.budget.level,
    })
}

/// The view now: document, page, centre and zoom.
fn view_now(app: &FigApp) -> Option<(u64, NodeId, [f64; 3])> {
    app.session.ds().ok().map(|ds| (ds.uid, ds.page, [ds.view.cx, ds.view.cy, ds.view.zoom]))
}

/// An agent moved the camera itself (`view.set`, `camera.focus-layer` over MCP or the control
/// channel): its own move, not the person's, so following doesn't pause.
pub fn agent_moved_camera(app: &mut FigApp) {
    app.ui.agent_view.follow.expected = view_now(app);
}

fn pause(app: &mut FigApp) {
    app.session.agents.follow_paused = true;
    let f = &mut app.ui.agent_view.follow;
    f.pending = None;
    if f.flying {
        app.ui.camera.stop();
        f.flying = false;
    }
}

/// The person pressed, scrolled or pinched on the canvas this frame (not on the toolbar or the chip).
fn person_on_canvas(app: &FigApp, ctx: &Context) -> bool {
    let r = app.canvas.rect;
    let chip = app.ui.agent_view.follow.chip;
    ctx.input(|i| {
        let over = i.pointer.hover_pos().filter(|p| r.contains(*p) && p.y < r.max.y - TOOLBAR_BAND && !chip.is_some_and(|c| c.contains(*p)));
        over.is_some() && (i.pointer.any_pressed() || i.events.iter().any(|e| matches!(e, egui::Event::MouseWheel { .. } | egui::Event::Zoom(_))))
    })
}

/// Something the person is in the middle of: a move now would pull the canvas from under them.
fn mid_gesture(app: &FigApp, ctx: &Context) -> bool {
    ctx.input(|i| i.pointer.any_down())
        || app.session.ds().is_ok_and(|ds| ds.text_edit.is_some() || ds.interaction.is_some())
        || (app.ui.camera.flying() && !app.ui.agent_view.follow.flying)
}

fn follow(app: &mut FigApp, ctx: &Context, now: f64, landed: &[Landed]) {
    let state = follow_state(&app.session);
    // The latest work, whatever the state (a resume goes there; the AI tab names it).
    if let Ok(doc) = app.session.doc().cloned()
        && let Some((page, ids)) = target(&doc, landed)
    {
        let name = landed.iter().rev().find(|l| l.agent).map(|l| agent_name(app, &l.route)).unwrap_or_default();
        let f = &mut app.ui.agent_view.follow;
        f.agent = name;
        f.artboard = ids.iter().next_back().and_then(|id| doc.top_level_frame(*id));
        f.latest = Some((page, ids.clone()));
        f.pending = match f.pending.take() {
            Some((p, mut old)) if p == page => {
                old.extend(ids);
                Some((p, old))
            }
            _ => Some((page, ids)),
        };
    }
    let f = &mut app.ui.agent_view.follow;
    if state == "following" && f.was != "following" {
        // Resumed (or turned on): to the latest work.
        f.pending = f.latest.clone();
        f.last_flight = None;
        f.expected = None;
    }
    f.was = state;
    if state != "following" {
        f.pending = None;
        return;
    }
    if !crate::camera_flight::sized(app) {
        app.ui.agent_view.follow.expected = None;
        return;
    }
    // The person wins: their edit, their input on the canvas, or a camera move of theirs.
    let person_edit = landed.iter().any(|l| !l.agent);
    let view = view_now(app);
    let f = &mut app.ui.agent_view.follow;
    let moved = if f.flying {
        if app.ui.camera.flying() {
            false
        } else {
            f.flying = false;
            view != f.expected
        }
    } else if app.ui.camera.flying() {
        false
    } else {
        let moved = f.expected.is_some_and(|e| Some(e) != view);
        if f.expected.is_none() {
            f.expected = view;
        }
        moved
    };
    // Only once there is an agent's work to follow: before that, the person's own work is just theirs.
    let following_work = app.ui.agent_view.follow.latest.is_some();
    if following_work && (person_edit || moved || person_on_canvas(app, ctx)) {
        pause(app);
        return;
    }
    let f = &app.ui.agent_view.follow;
    let Some((_, ids)) = f.pending.clone() else { return };
    if mid_gesture(app, ctx) {
        return;
    }
    if let Some(last) = f.last_flight
        && now - last < FLIGHT_EVERY
    {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(FLIGHT_EVERY - (now - last)));
        return;
    }
    fly(app, &ids, now);
}

/// Move to `ids` with D2's camera (`camera.focus-layer {ids, fit}` and its flight). The command's
/// function runs directly: it moves the view only, so it adds no revision or journal entry.
fn fly(app: &mut FigApp, ids: &BTreeSet<NodeId>, now: f64) {
    let f = &mut app.ui.agent_view.follow;
    f.pending = None;
    f.last_flight = Some(now);
    let Some(cmd) = figcraft_engine::find_command("camera.focus-layer") else { return };
    let p = json!({"ids": ids.iter().map(|i| i.0).collect::<Vec<_>>(), "fit": true});
    if (cmd.run)(&mut app.session, &p).is_err() {
        return;
    }
    let target = view_now(app);
    app.handle_ui_requests();
    let f = &mut app.ui.agent_view.follow;
    f.expected = target;
    f.flying = app.ui.camera.flying();
}

/// What the AI tab says: "Following Claude · Poster", paused, or nothing when it's off.
pub fn status(app: &FigApp) -> Option<String> {
    let f = &app.ui.agent_view.follow;
    let artboard = f.artboard.and_then(|id| app.session.doc().ok().and_then(|d| d.node(id).map(|n| n.name.clone())));
    match follow_state(&app.session) {
        "following" if f.agent.is_empty() => Some(tr("Following agents: waiting for their next edit").to_string()),
        "following" => Some(match artboard {
            Some(a) => trf("Following {agent} · {artboard}", &[("agent", &f.agent), ("artboard", &a)]),
            None => trf("Following {agent}", &[("agent", &f.agent)]),
        }),
        "paused" => Some(tr("Paused: you moved the camera or edited").to_string()),
        _ => None,
    }
}

/// The agent and the artboard of its latest work (the working indicator names them).
pub fn working_on(app: &FigApp) -> (String, Option<NodeId>) {
    let f = &app.ui.agent_view.follow;
    (f.agent.clone(), f.artboard)
}

/// The paused chip at the top of the canvas: one click resumes.
fn chip(app: &mut FigApp, ui: &mut Ui) {
    app.ui.agent_view.follow.chip = None;
    if follow_state(&app.session) != "paused" || app.canvas.rect.width() < 200.0 {
        return;
    }
    let t = Tokens::get(ui.ctx());
    let label = tr("Follow agent");
    let galley = ui.painter().layout_no_wrap(label.to_string(), semibold(11.0), t.text);
    let size = vec2(galley.size().x + 34.0, 26.0);
    let r = Rect::from_center_size(pos2(app.canvas.rect.center().x, app.canvas.rect.min.y + 12.0 + size.y / 2.0), size);
    let resp = ui.interact(r, Id::new("follow_agent_chip"), Sense::click()).on_hover_text(tr("Follow the agent again and go to its latest work"));
    crate::a11y::button(&resp, label);
    let p = ui.painter();
    p.rect_filled(r.translate(vec2(0.0, 1.0)), CornerRadius::same(13), egui::Color32::from_black_alpha(24));
    p.rect_filled(r, CornerRadius::same(13), if resp.hovered() { t.hover } else { t.panel });
    p.rect_stroke(r, CornerRadius::same(13), Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
    p.circle_filled(pos2(r.min.x + 13.0, r.center().y), 4.0, ink::fill());
    p.galley(pos2(r.min.x + 23.0, r.center().y - galley.size().y / 2.0), galley, t.text);
    if resp.has_focus() {
        p.rect_stroke(r, CornerRadius::same(13), Stroke::new(2.0, t.focus), egui::StrokeKind::Inside);
    }
    app.ui.agent_view.follow.chip = Some(r);
    if resp.clicked() {
        app.run("camera.follow-agents", json!({"on": true}));
    }
}

/// The AI tab's Follow row: the switch, Resume while paused, and what it is doing.
/// Shown once there's an agent to follow (an MCP client connected, or an agent's edit seen), and
/// whenever following is paused or off (so it can be turned back on here). Settings ▸ General and
/// the Actions palette always have it.
pub fn ai_section(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let on = app.session.prefs.settings.follow_agents;
    let paused = follow_state(&app.session) == "paused";
    let now = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).ok();
    let connected = figcraft_engine::cmd::mcp::status(&app.session, now)["state"] == "connected";
    if follow_state(&app.session) == "following" && !connected && app.ui.agent_view.follow.agent.is_empty() {
        return;
    }
    let (s, color) = match status(app) {
        Some(s) => (s, if paused { t.text_secondary } else { t.text }),
        None => (tr("Off: the camera stays where you leave it while agents work.").to_string(), t.text_secondary),
    };
    // The switch and Resume while paused; then what it is doing now.
    crate::right::row(ui, 30.0, |ui| {
        if crate::widgets::checkbox(ui, on, tr("Follow agent")) {
            app.run("camera.follow-agents", json!({"on": !on}));
        }
        if paused && crate::widgets::button(ui, tr("Resume"), true, true).clicked() {
            app.run("camera.follow-agents", json!({"on": true}));
        }
    });
    crate::right::row(ui, 20.0, |ui| {
        let resp = ui.add(egui::Label::new(egui::RichText::new(&s).color(color).font(regular(11.0))).truncate()).on_hover_text(&s);
        crate::a11y::status(&resp, &s, &s);
    });
}

#[cfg(test)]
#[path = "follow_agent_tests.rs"]
pub(crate) mod tests;
