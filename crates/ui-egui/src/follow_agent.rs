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

use egui::{Context, Rect, Ui};
use figcraft_doc::{Document, NodeId};
use figcraft_engine::cmd::camera::follow_state;
use figcraft_engine::settings::diagnostics::Pause;
use serde_json::json;

use crate::FigApp;
use crate::i18n::{tr, trf};
use crate::theme::{Tokens, regular};

#[path = "camera_log.rs"]
pub mod camera_log;
#[path = "chat_ui.rs"]
pub mod chat_ui;
#[path = "follow_end.rs"]
pub mod follow_end;
#[path = "grace_hint.rs"]
pub mod grace_hint;
#[path = "reveal.rs"]
pub mod reveal;
#[path = "working_indicator.rs"]
pub mod working_indicator;
#[path = "working_log.rs"]
pub mod working_log;
#[path = "working_state.rs"]
pub mod working_state;

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
    /// The MCP client it came from (its `clientInfo.name`), when the route knows.
    pub client: Option<String>,
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
    /// Whether agents are working (Stage I.1): the one source everything here reads.
    pub working: working_state::State,
    pub budget: Budget,
    /// A pause's end: the chip's and the paused row's fade out (Fix N).
    pub end: follow_end::State,
    /// Settings ▸ General scrolls to the grace once (Fix Q2's hint).
    pub focus_grace: bool,
    /// The grace's self-diagnosis hint (Fix Q2).
    pub grace_hint: grace_hint::State,
    /// The working state's clock, which carries on while the window is hidden (R3a).
    pub clock: working_log::Clock,
    /// The cause of the run's end, until the next frame takes it (R3a: it can end between frames).
    pub(crate) ended: Option<figcraft_engine::settings::diagnostics::Cause>,
    /// The platform was asked to keep the app awake (agents are working).
    pub(crate) awake: bool,
    /// "Describe a change": the composer, its transcript and its run (R5).
    pub chat: chat_ui::State,
    /// Who moved the camera, change by change (Fix S4).
    pub cameras: camera_log::Log,
}

impl State {
    /// A flight of following's own is in progress (the camera log names its frames, Fix S4).
    pub(crate) fn follow_flying(&self) -> bool {
        self.follow.flying
    }
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
    let active = app.ui.agent_view.reveal.active() || app.ui.agent_view.indicator.artboard().is_some() || app.ui.agent_view.working.working();
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
    /// An agent's request moved the camera and its effects are still settling (a file it opened
    /// fitted on the canvas's first sizing, its focus's flight): until then, every move is its own
    /// (R3b).
    agent_settling: bool,
    /// A flight of follow's own is in progress.
    flying: bool,
    /// The state last frame (a change to `following` is a resume).
    was: &'static str,
    /// An agent's edit has been seen; the name of who made the latest (`None`: it announced none),
    /// and the artboard (top-level frame) of their latest work.
    seen_agent: bool,
    agent: Option<String>,
    artboard: Option<NodeId>,
    /// The paused chip, where it was drawn (a press on it isn't a press on the canvas).
    chip: Option<Rect>,
    /// Why following last paused, until it resumes (Fix L2's diagnostics, `ui.inspect`).
    paused_by: Option<Pause>,
}

/// The name a person sees for the agent that made an edit (Stage J): the MCP client it came from
/// (`client`, by its session), else over MCP the newest client to connect, mapped to its friendly
/// name (`agents::client_display_name`) or shown as announced. `None`: no name was announced, and
/// the caller says "an agent". Never a guess, and never one agent's name for another.
pub fn agent_name(app: &FigApp, route: &str, client: Option<&str>) -> Option<String> {
    let newest = || route.starts_with("mcp").then(|| app.session.agents.mcp.clients.last().map(|c| c.0.clone())).flatten();
    client.map(str::to_string).or_else(newest).and_then(|raw| figcraft_engine::agents::client_display_name(&raw))
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
                client: c.client.clone(),
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
    working_log::frame(app, now);
    camera_log::before_canvas(app);
    let at = working_log::clock(app, ctx);
    let landed = scan(app);
    // A plugin's edits keep it working too (MCP clients and the control channel are noted per request).
    for l in landed.iter().filter(|l| l.agent && l.route.starts_with("plugin")) {
        app.ui.agent_view.working.touch(working_state::key(None, None, &l.route), None, at);
    }
    // Each transition, with its cause, into Settings ▸ Diagnostics (Fix M0); requests between
    // frames have already done the same (R3a).
    working_log::settle(app, ctx, at);
    // The run's end, if it ended since the last frame (Fix N clears a pause then).
    let ended = working_log::take_ended(app);
    follow_end::settle(app, now, ended);
    chat_ui::tick(app, ctx);
    follow(app, ctx, now, &landed);
    reveal::tick(app, ctx, now, gap, &landed);
    working_indicator::tick(app, ctx, now, &landed);
    app.ui.agent_view.landed_hidden = false;
    budget(app, now);
}

/// Every frame, after the canvas: the paused chip, and the working indicator's AccessKit node.
pub fn after_canvas(app: &mut FigApp, ui: &mut Ui) {
    follow_end::chip(app, ui);
    working_indicator::accessibility(app, ui);
}

/// The grace period (Settings ▸ General), in seconds.
pub fn grace(app: &FigApp) -> f64 {
    f64::from(app.session.agent_grace())
}

/// The time of the frame being drawn (requests answered between frames use it too).
pub fn now(app: &FigApp) -> f64 {
    app.ui.agent_view.last_frame.unwrap_or(0.0)
}

/// The name of who is working (the most recently active client), else of the latest edit's.
pub fn agent_now(app: &FigApp) -> Option<String> {
    let w = &app.ui.agent_view.working;
    // A client that gave no name: J1's rule, the latest edit's (over MCP, the newest to connect).
    let named = w.working().then(|| w.name(now(app), grace(app))).flatten();
    named.or_else(|| app.ui.agent_view.follow.agent.clone())
}

/// "Codex is working on Poster" (or "… is working" before its first edit): the working state's
/// words, for the indicator's AccessKit status and the AI tab. `None` when no one is working.
pub fn working_text(app: &FigApp) -> Option<String> {
    if !app.ui.agent_view.working.working() {
        return None;
    }
    let agent = agent_now(app);
    let artboard = app.ui.agent_view.indicator.artboard().and_then(|id| app.session.doc().ok()?.node(id).map(|n| n.name.clone()));
    Some(match (agent, artboard) {
        (Some(a), Some(b)) => trf("{agent} is working on {artboard}", &[("agent", &a), ("artboard", &b)]),
        (None, Some(b)) => trf("An agent is working on {artboard}", &[("artboard", &b)]),
        (Some(a), None) => trf("{agent} is working", &[("agent", &a)]),
        (None, None) => tr("An agent is working").to_string(),
    })
}

/// `finish_working` (the engine's `agent.finish-working`): the calling client is done.
pub fn finish(app: &mut FigApp, data: &serde_json::Value) {
    let s = |k: &str| data.get(k).and_then(serde_json::Value::as_str);
    let key = working_state::key(s("session"), s("client"), s("route").unwrap_or_default());
    // The time of the request being answered (R3a: a hidden window's frames don't move).
    let now = working_log::now(app);
    app.ui.agent_view.working.finish(&key, now);
}

/// "Codex finished": said once a run of work ends, for [`working_state::FINISHED_FOR`].
pub fn finished_text(app: &FigApp) -> Option<String> {
    let name = app.ui.agent_view.working.finished(now(app))?;
    Some(match name {
        Some(n) => trf("{agent} finished", &[("agent", &n)]),
        None => tr("An agent finished").to_string(),
    })
}

/// For `ui.inspect`: following, the reveal, the indicator and the degradation step.
pub fn inspect(app: &FigApp) -> serde_json::Value {
    let v = &app.ui.agent_view;
    json!({
        "follow": follow_state(&app.session),
        "pausedBy": v.follow.paused_by.map(Pause::text),
        "chip": follow_end::inspect(app),
        "revealing": v.reveal.entering().count(),
        "reveals": v.reveal.diagnostics(),
        "indicator": v.indicator.drawn.get().map(|(r, cols)| json!({"columns": cols, "rect": [r.min.x, r.min.y, r.max.x, r.max.y]})),
        "outline": v.indicator.outlined.get().is_some(),
        "working": working_text(app),
        "workingClients": v.working.count(now(app), grace(app)),
        // The dots' opacity now (0..1): a fade out and in between steps shows here (Fix M0).
        "workingAlpha": v.working.alpha(now(app), !app.reduce_motion()),
        "finished": finished_text(app),
        "degraded": v.budget.level,
        "cameraLog": camera_log::inspect(app),
    })
}

/// The view now: document, page, centre and zoom.
fn view_now(app: &FigApp) -> Option<(u64, NodeId, [f64; 3])> {
    app.session.ds().ok().map(|ds| (ds.uid, ds.page, [ds.view.cx, ds.view.cy, ds.view.zoom]))
}

/// An agent moved the camera itself (`view.set`, `camera.focus-layer`, `page.select`, `file.open`
/// over MCP or the control channel): its own move, not the person's, so following doesn't pause.
/// What follows from it on later frames is its own too (R3b): the new file's fit on the canvas's
/// first sizing, the flight its focus starts. The view it settles at is the one expected.
pub fn agent_moved_camera(app: &mut FigApp) {
    let f = &mut app.ui.agent_view.follow;
    f.expected = None;
    f.agent_settling = true;
}

fn pause(app: &mut FigApp, why: Pause) {
    app.session.agents.follow_paused = true;
    crate::settings_privacy::record(app, figcraft_engine::settings::diagnostics::Entry::FollowPaused(why));
    let f = &mut app.ui.agent_view.follow;
    f.paused_by = Some(why);
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
    let (over, acted) = ctx.input(|i| {
        let over = i.pointer.hover_pos().filter(|p| r.contains(*p) && p.y < r.max.y - TOOLBAR_BAND && !chip.is_some_and(|c| c.contains(*p)));
        (over, i.pointer.any_pressed() || i.events.iter().any(|e| matches!(e, egui::Event::MouseWheel { .. } | egui::Event::Zoom(_))))
    });
    // Not when a modal or a dialog covers the canvas there (K2). Asked outside `input`: it reads
    // egui's memory, which can't be locked inside it.
    acted && over.is_some_and(|p| !crate::dialogs::modal_input::canvas_covered(ctx, p))
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
        let name = landed.iter().rev().find(|l| l.agent).and_then(|l| agent_name(app, &l.route, l.client.as_deref()));
        let f = &mut app.ui.agent_view.follow;
        f.seen_agent = true;
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
    let resumed = state == "following" && f.was == "paused";
    if state == "following" && f.was != "following" {
        // Resumed (or turned on): to the latest work.
        f.pending = f.latest.clone();
        f.last_flight = None;
        f.expected = None;
        f.paused_by = None;
    }
    f.was = state;
    if state != "following" {
        f.pending = None;
        return;
    }
    if resumed {
        follow_end::resumed(app);
    }
    if !crate::camera_flight::sized(app) {
        app.ui.agent_view.follow.expected = None;
        return;
    }
    // The person wins: their edit, their input on the canvas, or a camera move of theirs.
    let person_edit = landed.iter().any(|l| !l.agent);
    let view = view_now(app);
    let agents_camera = app.ui.camera.agents();
    let f = &mut app.ui.agent_view.follow;
    let moved = if f.agent_settling || agents_camera {
        // An agent's own move, still settling (R3b): it replaces follow's flight, and the view it
        // comes to rest at is the one expected.
        f.flying = false;
        let settled = !agents_camera && !app.ui.camera.flying();
        f.expected = settled.then_some(view).flatten();
        f.agent_settling = !settled;
        false
    } else if f.flying {
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
    let why = if person_edit {
        Some(Pause::PersonEdit)
    } else if moved {
        Some(Pause::CameraMoved(camera_log::mover(app)))
    } else if person_on_canvas(app, ctx) {
        Some(Pause::CanvasInput)
    } else {
        None
    };
    // A pause lasts only while an agent works (Fix N): with no one working, there's nothing to pause.
    if following_work
        && app.ui.agent_view.working.working()
        && let Some(why) = why
    {
        pause(app, why);
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
    camera_log::note(app, figcraft_engine::settings::diagnostics::Mover::Follow, "");
    let f = &mut app.ui.agent_view.follow;
    f.expected = target;
    f.flying = app.ui.camera.flying();
}

/// What the AI tab says: "Following Claude · Poster", paused, or nothing when it's off.
pub fn status(app: &FigApp) -> Option<String> {
    let f = &app.ui.agent_view.follow;
    let artboard = f.artboard.and_then(|id| app.session.doc().ok().and_then(|d| d.node(id).map(|n| n.name.clone())));
    match follow_state(&app.session) {
        "following" if !f.seen_agent => Some(tr("Following agents: waiting for their next edit").to_string()),
        "following" => {
            let agent = agent_now(app).unwrap_or_else(|| tr("an agent").to_string());
            Some(match artboard {
                Some(a) => trf("Following {agent} · {artboard}", &[("agent", &agent), ("artboard", &a)]),
                None => trf("Following {agent}", &[("agent", &agent)]),
            })
        }
        "paused" => Some(tr("Paused: you moved the camera or edited").to_string()),
        _ => None,
    }
}

/// The agent and the artboard of its latest work (the working indicator names them).
pub fn working_on(app: &FigApp) -> (Option<String>, Option<NodeId>) {
    let f = &app.ui.agent_view.follow;
    (f.agent.clone(), f.artboard)
}

/// The AI tab's Follow row: the switch, Resume while paused, and what it is doing.
/// Shown once there's an agent to follow (an MCP client connected, or an agent's edit seen), and
/// whenever following is paused or off (so it can be turned back on here). Settings ▸ General and
/// the Actions palette always have it.
pub fn ai_section(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let on = app.session.prefs.settings.follow_agents;
    // Paused while an agent works; fading out for FADE once that work has ended (Fix N).
    let paused = follow_end::shown(app);
    let fade = if paused { None } else { follow_end::fading(app, now(app)) };
    let now = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).ok();
    let connected = figcraft_engine::cmd::mcp::status(&app.session, now)["state"] == "connected";
    if follow_state(&app.session) == "following" && !connected && !app.ui.agent_view.follow.seen_agent {
        return;
    }
    let (s, color) = match (fade, status(app)) {
        (Some(a), _) => (tr("Paused: you moved the camera or edited").to_string(), t.text_secondary.gamma_multiply(a)),
        (None, Some(s)) => (s, if paused { t.text_secondary } else { t.text }),
        (None, None) => (tr("Off: the camera stays where you leave it while agents work.").to_string(), t.text_secondary),
    };
    // The switch and Resume while paused; then what it is doing now.
    // Resume first, at the row's end, so the switch (as wide as it may be) can't push it out of a
    // narrow or scrolling panel.
    crate::right::row(ui, 30.0, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if paused && crate::widgets::button(ui, tr("Resume"), true, true).clicked() {
                app.run("camera.follow-agents", json!({"on": true}));
            }
            if let Some(a) = fade {
                // Fading out: painted, not pressable.
                ui.ctx().request_repaint();
                ui.scope(|ui| {
                    ui.multiply_opacity(a);
                    ui.disable();
                    crate::widgets::button(ui, tr("Resume"), true, true);
                });
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if crate::widgets::checkbox(ui, on, tr("Follow agent")) {
                    app.run("camera.follow-agents", json!({"on": !on}));
                }
            });
        });
    });
    crate::right::row(ui, 20.0, |ui| {
        let resp = ui.add(egui::Label::new(egui::RichText::new(&s).color(color).font(regular(11.0))).truncate()).on_hover_text(&s);
        crate::a11y::status(&resp, &s, &s);
    });
}

#[cfg(test)]
#[path = "follow_agent_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "client_names_tests.rs"]
mod client_names_tests;
#[cfg(test)]
#[path = "follow_moves_tests.rs"]
mod follow_moves_tests;
#[cfg(test)]
#[path = "follow_pause_cause_tests.rs"]
mod follow_pause_cause_tests;
