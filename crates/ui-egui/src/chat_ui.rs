//! "Describe a change" in the AI tab (R5c, `docs/design/chat-composer.md`): the composer at the
//! bottom of the AI tab, the transcript above it, the first-use confirmation, and the run.
//! - **Send** (`chat.send`, the person's alone) asks the window ([`request`]); the window checks the
//!   endpoint, the agent's binary and its confirmation, then starts the run through
//!   `Services::start_agent` (the desktop app's runner; nothing elsewhere).
//! - **While it runs** ([`tick`], every frame and every logic pass): its output is read with the
//!   profile's parser, each batch of work it lands adds a quiet line, and the elapsed time shows.
//! - **When it ends:** its reply, or the failure in plain words; the tokens or cost it reported, or
//!   "Billed by your agent's plan."; its MCP sessions end as `finish_working` ends them.
//! - **Stop and Esc** (`chat.cancel`) stop the whole process tree; partial edits stay as ordinary
//!   undo steps.
//! - The transcript lives in the window's memory only.

use std::sync::Arc;

use egui::{Context, Id, RichText, Ui};
use figcraft_doc::Document;
use figcraft_engine::cmd::agent_clients::headless::{self, Headless, output};
use figcraft_engine::cmd::chat;
use figcraft_engine::settings::diagnostics::Entry;
use serde_json::{Value, json};

use crate::i18n::{tr, trf, trn};
use crate::services::{AgentEnd, AgentRun, AgentRunEvent, AgentRunSpec};
use crate::theme::{Tokens, regular};
use crate::{FigApp, widgets};

/// How often a run is read and its elapsed time redrawn, in seconds.
const POLL: f64 = 0.25;
/// How often installed agents are looked for again, in seconds.
const LOOK_AGAIN: f64 = 5.0;
/// The transcript's height at most.
const TRANSCRIPT_H: f32 = 220.0;
/// A run that ends this soon with nothing done "stopped before it began" (not signed in…).
const AT_ONCE_SECS: f64 = 5.0;
/// For live checks only: a run's limit in seconds when shorter than the setting's.
pub const LIMIT_VAR: &str = "SCRIM_CHAT_LIMIT_SECS";

/// A line of the transcript.
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    You(String),
    Reply {
        agent: String,
        text: String,
    },
    /// A batch of the agent's work ("Agent: made 12 layers").
    Work(String),
    Note(String),
    Failure(String),
    /// The time it took, and what it cost as reported (or that the agent's plan bills it).
    Cost(String),
}

/// A run in progress.
pub struct Running {
    pub agent: &'static Headless,
    started: f64,
    handle: Box<dyn AgentRun>,
    outcome: output::Outcome,
    pub stopping: bool,
    /// The document it began in, as it was, and the change log's last revision read.
    uid: u64,
    start_doc: Arc<Document>,
    rev: u64,
    /// Its time limit, in seconds.
    limit: u64,
}

/// The composer's state in a window.
#[derive(Default)]
pub struct State {
    pub lines: Vec<Line>,
    pub draft: String,
    /// The agent picked (a headless profile id).
    pub agent: Option<String>,
    pub running: Option<Running>,
    /// A send waiting for the first-use confirmation: agent, binary path, prompt.
    pub confirm: Option<(String, String, String)>,
    /// The installed agents as last looked up (id, path), and when.
    found: Vec<(String, String)>,
    found_at: Option<f64>,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("chat::State").field("lines", &self.lines.len()).field("running", &self.running.is_some()).finish()
    }
}

/// The installed agents (id, binary path), looked up at most every [`LOOK_AGAIN`] seconds.
pub fn installed(app: &mut FigApp, now: f64) -> Vec<(String, String)> {
    let c = &app.ui.agent_view.chat;
    if c.found_at.is_none_or(|t| now - t >= LOOK_AGAIN || now < t) {
        let found: Vec<(String, String)> =
            headless::profiles().iter().filter_map(|p| app.services.find_program(&p.program).map(|path| (p.id.clone(), path))).collect();
        let c = &mut app.ui.agent_view.chat;
        c.found = found;
        c.found_at = Some(now);
    }
    app.ui.agent_view.chat.found.clone()
}

/// The agent the picker shows: the one picked if installed, else the first installed.
fn picked(app: &mut FigApp, now: f64) -> Option<(&'static Headless, String)> {
    let found = installed(app, now);
    let want = app.ui.agent_view.chat.agent.clone();
    let (id, path) = found.iter().find(|(id, _)| Some(id) == want.as_ref()).or_else(|| found.first())?.clone();
    headless::profile(&id).map(|p| (p, path))
}

fn now(app: &FigApp) -> f64 {
    crate::follow_agent::working_log::now(app)
}

/// Why Send can't be pressed now, if it can't.
pub fn send_blocked(app: &mut FigApp, now: f64) -> Option<String> {
    if let Some(r) = &app.ui.agent_view.chat.running {
        return Some(trf("{agent} is working: stop it first", &[("agent", &r.agent.name)]));
    }
    if app.session.agents.mcp.port.is_none() {
        return Some(tr("Scrim's MCP server is off: turn it on in Settings ▸ MCP").to_string());
    }
    if picked(app, now).is_none() {
        return Some(tr("No agent found on this computer").to_string());
    }
    if app.ui.agent_view.chat.draft.trim().is_empty() {
        return Some(tr("Type what to change").to_string());
    }
    None
}

/// The window's part of the chat commands (`chat.send`, `chat.cancel`, `chat.clear`,
/// `chat.allowed`). Returns whether `kind` was one.
pub fn request(app: &mut FigApp, kind: &str, data: &Value) -> bool {
    let s = |k: &str| data.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
    match kind {
        chat::SEND_REQUEST => send(app, &s("agent"), &s("prompt")),
        chat::CANCEL_REQUEST => cancel(app),
        chat::CLEAR_REQUEST => app.ui.agent_view.chat.lines.clear(),
        chat::ALLOWED_REQUEST => {
            if let Some((agent, path, prompt)) = app.ui.agent_view.chat.confirm.take()
                && agent == s("agent")
                && path == s("path")
                && let Some(p) = headless::profile(&agent)
            {
                start(app, p, path, prompt);
            }
        }
        _ => return false,
    }
    true
}

fn line(app: &mut FigApp, l: Line) {
    app.ui.agent_view.chat.lines.push(l);
}

fn send(app: &mut FigApp, agent: &str, prompt: &str) {
    if app.ui.agent_view.chat.running.is_some() {
        return;
    }
    let Some(p) = headless::profile(agent) else { return };
    // Looked up afresh: the program may have been installed or moved since the picker last looked.
    let found = app.services.find_program(&p.program);
    line(app, Line::You(prompt.to_string()));
    app.ui.agent_view.chat.draft.clear();
    if app.session.agents.mcp.port.is_none() {
        line(app, Line::Failure(tr("Scrim's MCP server is off. Turn it on in Settings ▸ MCP, then send again.").to_string()));
        return;
    }
    let Some(path) = found else {
        line(app, Line::Failure(trf("{agent} isn't installed on this computer, or Scrim can't find it.", &[("agent", &p.name)])));
        return;
    };
    if !chat::allowed(&app.session, agent, &path) {
        app.ui.agent_view.chat.confirm = Some((agent.to_string(), path, prompt.to_string()));
        return;
    }
    start(app, p, path, prompt.to_string());
}

/// The run's time limit: the setting, or shorter for a live check ([`LIMIT_VAR`]).
fn limit_secs(app: &FigApp) -> u64 {
    let setting = u64::from(app.session.prefs.settings.chat_time_limit_mins) * 60;
    let env = app.services.agent_client_env();
    let short = env.as_ref().and_then(|e| e.vars.get(LIMIT_VAR)).and_then(|v| v.parse::<u64>().ok()).filter(|s| *s > 0);
    short.map_or(setting, |s| s.min(setting))
}

fn start(app: &mut FigApp, agent: &'static Headless, path: String, prompt: String) {
    let Some(url) = app.session.agents.mcp.url() else {
        line(app, Line::Failure(tr("Scrim's MCP server is off. Turn it on in Settings ▸ MCP, then send again.").to_string()));
        return;
    };
    let m = &app.session.prefs.settings.mcp;
    let token = m.require_auth.then(|| m.token.clone());
    let args = agent.argv(&headless::message(&prompt, &app.session), &url, token.is_some());
    let env = app.services.agent_client_env().unwrap_or_default();
    let var = |k: &str| env.vars.get(k).cloned();
    let home = var(headless::CHILD_HOME_VAR).or_else(|| env.home.as_ref().map(|h| h.to_string_lossy().into_owned()));
    let child_env = headless::child_env(
        var("PATH").as_deref(),
        home.as_deref(),
        var("USER").as_deref(),
        var("LOGNAME").as_deref(),
        token.as_deref(),
        &headless::system_env(),
    );
    let limit = limit_secs(app);
    let spec = AgentRunSpec { program: path, args, env: child_env, limit_secs: limit };
    match app.services.start_agent(spec) {
        Ok(handle) => {
            let Ok(ds) = app.session.ds() else { return };
            let (uid, start_doc) = (ds.uid, ds.doc.clone());
            let rev = app.session.agents.last_edit(uid).unwrap_or(0);
            let started = now(app);
            app.ui.agent_view.chat.running =
                Some(Running { agent, started, handle, outcome: Default::default(), stopping: false, uid, start_doc, rev, limit });
            // Settings ▸ Diagnostics: the agent's name only, never the prompt.
            crate::settings_privacy::record(app, Entry::ChatRun { agent: &agent.name, ended: None });
        }
        Err(e) => line(app, Line::Failure(trf("{agent} couldn't start: {error}", &[("agent", &agent.name), ("error", &e)]))),
    }
}

fn cancel(app: &mut FigApp) {
    let c = &mut app.ui.agent_view.chat;
    if let Some(r) = c.running.as_mut() {
        r.handle.stop();
        r.stopping = true;
    } else if c.confirm.take().is_some() {
        line(app, Line::Note(tr("Not run.").to_string()));
    }
}

/// Each frame and each logic pass: read the run, add the lines of its new work, and end it.
pub fn tick(app: &mut FigApp, ctx: &Context) {
    let Some(r) = app.ui.agent_view.chat.running.as_mut() else { return };
    let mut ended = None;
    for e in r.handle.poll() {
        match e {
            AgentRunEvent::Line(l) => {
                for ev in output::parse(r.agent.output, &l) {
                    r.outcome.add(ev);
                }
            }
            AgentRunEvent::Ended { end, stderr, secs } => ended = Some((end, stderr, secs)),
        }
    }
    work_lines(app);
    match ended {
        Some((end, stderr, secs)) => finish(app, end, &stderr, secs),
        None => ctx.request_repaint_after(std::time::Duration::from_secs_f64(POLL)),
    }
}

/// One quiet line per agent edit logged since the last look: what it made, changed or removed.
fn work_lines(app: &mut FigApp) {
    let Some(r) = app.ui.agent_view.chat.running.as_ref() else { return };
    let (uid, rev, start_doc) = (r.uid, r.rev, r.start_doc.clone());
    let Ok(doc) = app.session.doc().cloned() else { return };
    let mut lines = Vec::new();
    let mut last = rev;
    let changes: Vec<_> = app.session.agents.edits_after(uid, rev).collect();
    for (i, c) in changes.iter().enumerate() {
        last = last.max(c.revision);
        // A batch logs each step, then itself with them all: one line, the batch's.
        let covered = changes.get(i + 1..).into_iter().flatten().any(|later| c.ids.iter().all(|id| later.ids.contains(id)));
        if !c.author.is_agent() || covered {
            continue;
        }
        let made = c.ids.iter().filter(|id| start_doc.node(**id).is_none() && doc.node(**id).is_some()).count();
        let gone = c.ids.iter().filter(|id| doc.node(**id).is_none()).count();
        let changed = c.ids.len() - made - gone;
        let text = if made > 0 {
            trn("Agent: made {n} layers", made, &[])
        } else if changed > 0 {
            trn("Agent: changed {n} layers", changed, &[])
        } else if gone > 0 {
            trn("Agent: removed {n} layers", gone, &[])
        } else {
            continue;
        };
        lines.push(Line::Work(text));
    }
    if let Some(r) = app.ui.agent_view.chat.running.as_mut() {
        r.rev = last;
    }
    app.ui.agent_view.chat.lines.extend(lines);
}

/// "0:42", "12:05".
pub fn elapsed(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

fn finish(app: &mut FigApp, end: AgentEnd, stderr: &[String], secs: f64) {
    let Some(r) = app.ui.agent_view.chat.running.take() else { return };
    let name = r.agent.name.clone();
    let o = &r.outcome;
    let token = app.session.prefs.settings.mcp.require_auth.then(|| app.session.prefs.settings.mcp.token.clone());
    let last = stderr.last().map(|l| headless::redact(l, token.as_deref())).unwrap_or_default();
    // A result the CLI marks as an error ("Not logged in · Please run /login") is its failure's
    // words, not a reply.
    let cli_error = o.finished == Some(true);
    let said = o.reply.clone().filter(|_| cli_error).map(|r| headless::redact(&r, token.as_deref()));
    if let Some(text) = o.reply.as_ref().filter(|_| !cli_error) {
        line(app, Line::Reply { agent: name.clone(), text: text.clone() });
    }
    let (failure, outcome) = match &end {
        AgentEnd::Stopped => (None, "stopped"),
        AgentEnd::TimedOut if r.limit % 60 == 0 => {
            let mins = (r.limit / 60).to_string();
            (Some(trf("Stopped after {n} min, the time limit in Settings ▸ AI & agents.", &[("n", &mins)])), "timed out")
        }
        // A live check's shorter limit (`SCRIM_CHAT_LIMIT_SECS`).
        AgentEnd::TimedOut => (Some(trf("Stopped after {n} s, this run's time limit.", &[("n", &r.limit)])), "timed out"),
        _ if o.scrim_connected == Some(false) => (Some(trf("{agent} couldn't reach Scrim's MCP server.", &[("agent", &name)])), "failed"),
        AgentEnd::Exited(code) if *code != Some(0) => {
            let code = code.map_or_else(|| tr("a signal").to_string(), |c| c.to_string());
            let detail = said
                .clone()
                .or_else(|| o.failure.clone())
                .or_else(|| Some(last.clone()).filter(|l| !l.is_empty()))
                .unwrap_or_else(|| tr("no message").to_string());
            if secs < AT_ONCE_SECS && (o.reply.is_none() || cli_error) && o.tools == 0 {
                (Some(trf("{agent} stopped before it began: {detail}", &[("agent", &name), ("detail", &detail)])), "failed")
            } else {
                (
                    Some(trf("{agent} stopped with an error (exit {code}): {detail}", &[("agent", &name), ("code", &code), ("detail", &detail)])),
                    "failed",
                )
            }
        }
        _ if o.finished == Some(true) || o.failure.is_some() => {
            let detail = said.clone().or_else(|| o.failure.clone()).unwrap_or_else(|| tr("see its reply").to_string());
            (Some(trf("{agent} reported an error: {detail}", &[("agent", &name), ("detail", &detail)])), "failed")
        }
        _ => (None, "finished"),
    };
    match (&end, failure) {
        (AgentEnd::Stopped, _) => line(app, Line::Note(tr("Stopped. What the agent made so far stays: Undo takes it back.").to_string())),
        (_, Some(f)) => line(app, Line::Failure(f)),
        _ => {}
    }
    line(app, Line::Cost(cost_line(o, secs)));
    // Its MCP sessions end now, as finish_working ends them, so the dots stop at once.
    let at = now(app);
    let agent = r.agent;
    app.ui.agent_view.working.finish_where(at, |raw| raw.is_some_and(|n| agent.announces(n)), r.started);
    let code = match end {
        AgentEnd::Exited(c) => c,
        _ => None,
    };
    // How it ended, its exit code and how long it took; never what it said.
    crate::settings_privacy::record(app, Entry::ChatRun { agent: &name, ended: Some((outcome, code, secs.max(0.0).round() as u64)) });
}

/// "0:42 · Billed by your agent's plan.", or the tokens and cost the agent reported.
fn cost_line(o: &output::Outcome, secs: f64) -> String {
    let tokens = match (o.input_tokens, o.output_tokens) {
        (Some(i), Some(out)) => Some(trf("{in} tokens in, {out} out", &[("in", &i), ("out", &out)])),
        (Some(i), None) => Some(trf("{in} tokens in", &[("in", &i)])),
        (None, Some(out)) => Some(trf("{out} tokens out", &[("out", &out)])),
        (None, None) => None,
    };
    let cost = o.cost_usd.map(|c| format!("${c:.2}"));
    let what = match (tokens, cost) {
        (Some(t), Some(c)) => trf("{tokens}, {cost}, as the agent reported", &[("tokens", &t), ("cost", &c)]),
        (Some(t), None) => trf("{tokens}, as the agent reported", &[("tokens", &t)]),
        (None, Some(c)) => trf("{cost}, as the agent reported", &[("cost", &c)]),
        (None, None) => tr("Billed by your agent's plan.").to_string(),
    };
    format!("{} · {what}", elapsed(secs))
}

/// The composer, at the bottom of the AI tab (a panel of its own, so it stays put while the tab
/// scrolls). Nothing when "Describe a change" is off.
pub fn panel(app: &mut FigApp, ui: &mut Ui) {
    if !app.session.prefs.settings.describe_change || app.ui.right_tab != crate::RightTab::Ai {
        return;
    }
    let t = Tokens::get(ui.ctx());
    egui::Panel::bottom(Id::new("describe_change"))
        .frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(widgets::PAD as i8, 8)))
        .show(ui, |ui| composer(app, ui));
}

fn composer(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let now = ui.ctx().input(|i| i.time);
    ui.label(RichText::new(tr("Describe a change")).font(crate::theme::semibold(11.5)).color(t.text));
    ui.add_space(4.0);
    transcript(app, ui, &t);
    let found = installed(app, now);
    if found.is_empty() {
        ui.label(
            RichText::new(tr("No agent found on this computer. Install Claude Code or Codex and sign in, then describe a change here."))
                .font(regular(11.0))
                .color(t.text_secondary),
        );
        ui.horizontal_wrapped(|ui| {
            for p in headless::profiles() {
                if widgets::button(ui, &trf("{agent} docs", &[("agent", &p.name)]), false, true).clicked() {
                    app.services.open_url(&p.docs);
                }
            }
        });
        return;
    }
    // The agent picker: the installed agents with a headless profile.
    let current = picked(app, now);
    let label = current.as_ref().map_or_else(String::new, |(p, _)| p.name.clone());
    let path = current.as_ref().map_or_else(String::new, |(_, path)| path.clone());
    let resp = widgets::dropdown(ui, &label, 160.0, None).on_hover_text(&path);
    crate::a11y::menu_button(
        &resp,
        &trf("Agent: {agent}", &[("agent", &label)]),
        egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&resp)),
    );
    crate::right::menu_popup(&resp).width(200.0).show(|ui| {
        for (id, path) in &found {
            let Some(p) = headless::profile(id) else { continue };
            if ui.button(&p.name).on_hover_text(path).clicked() {
                app.ui.agent_view.chat.agent = Some(id.clone());
            }
        }
    });
    ui.add_space(4.0);
    let field_id = Id::new("describe_change_field");
    let running = app.ui.agent_view.chat.running.is_some();
    // ⌘↩ and Esc are read before the field takes them (it would add a line, or let go of the
    // keyboard).
    let focused = ui.memory(|m| m.has_focus(field_id));
    let (cmd_enter, esc) = if focused {
        ui.input_mut(|i| (i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter), i.key_pressed(egui::Key::Escape)))
    } else {
        (false, ui.input(|i| i.key_pressed(egui::Key::Escape)))
    };
    let r =
        egui::Frame::NONE.fill(t.field).corner_radius(5).inner_margin(egui::Margin::same(6)).stroke(egui::Stroke::new(1.0, t.text_secondary)).show(
            ui,
            |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut app.ui.agent_view.chat.draft)
                        .id(field_id)
                        .frame(egui::Frame::NONE)
                        .font(regular(11.5))
                        .hint_text(tr("Describe a change…"))
                        .desired_rows(2)
                        .desired_width(f32::INFINITY),
                )
            },
        );
    let field = r.inner;
    crate::a11y::text(&field, tr("Describe a change"), &app.ui.agent_view.chat.draft);
    let blocked = send_blocked(app, now);
    ui.add_space(6.0);
    let mut send = false;
    let mut stop = false;
    ui.horizontal(|ui| {
        if running {
            stop = widgets::button(ui, tr("Stop"), true, true).clicked();
        } else {
            send = widgets::button(ui, tr("Send"), true, blocked.is_none())
                .on_hover_text(format!("{}  {}", tr("Send"), if cfg!(target_os = "macos") { "⌘↩" } else { "Ctrl+Enter" }))
                .clicked();
        }
        if !running && !app.ui.agent_view.chat.lines.is_empty() && widgets::button(ui, tr("Clear"), false, true).clicked() {
            app.run("chat.clear", json!({}));
        }
    });
    // Why Send can't go, or how long the run has taken.
    let status = match &app.ui.agent_view.chat.running {
        Some(r) if r.stopping => tr("Stopping…").to_string(),
        Some(r) => trf("{agent} is working · {time}", &[("agent", &r.agent.name), ("time", &elapsed(now - r.started))]),
        None => blocked.clone().unwrap_or_default(),
    };
    if !status.is_empty() {
        ui.label(RichText::new(status).font(regular(10.5)).color(t.text_secondary));
    }
    if cmd_enter && blocked.is_none() {
        send = true;
    }
    // Esc stops a run from the field, or anywhere in the tab while one runs.
    if esc && running && (focused || field.has_focus() || !ui.ctx().memory(|m| m.focused().is_some())) {
        stop = true;
    }
    if send {
        let (agent, prompt) = (current.as_ref().map(|(p, _)| p.id.clone()).unwrap_or_default(), app.ui.agent_view.chat.draft.clone());
        app.run("chat.send", json!({"prompt": prompt, "agent": agent}));
    }
    if stop {
        app.run("chat.cancel", json!({}));
    }
}

fn transcript(app: &FigApp, ui: &mut Ui, t: &Tokens) {
    let lines = &app.ui.agent_view.chat.lines;
    if lines.is_empty() {
        return;
    }
    egui::ScrollArea::vertical().id_salt("describe_change_transcript").max_height(TRANSCRIPT_H).stick_to_bottom(true).show(ui, |ui| {
        for l in lines {
            let (who, text, color) = match l {
                Line::You(s) => (Some(tr("You").to_string()), s.as_str(), t.text),
                Line::Reply { agent, text } => (Some(agent.clone()), text.as_str(), t.text),
                Line::Work(s) | Line::Note(s) | Line::Cost(s) => (None, s.as_str(), t.text_secondary),
                Line::Failure(s) => (None, s.as_str(), t.danger),
            };
            if let Some(who) = who {
                ui.label(RichText::new(who).font(crate::theme::semibold(10.5)).color(t.text_secondary));
            }
            ui.label(RichText::new(text).font(regular(11.0)).color(color));
            ui.add_space(4.0);
        }
    });
    ui.add_space(6.0);
}

/// Settings ▸ AI & agents › Describe a change (R5d): the switch, the time limit, and the agents
/// allowed to run, each with its program, with Forget.
pub fn settings(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    crate::settings_dialog::heading(ui, tr("Describe a change"));
    let on = app.session.prefs.settings.describe_change;
    if widgets::checkbox(ui, on, tr("Describe a change in the AI tab")) {
        app.run("pref.describeChange", json!({"on": !on}));
    }
    widgets::label(
        ui,
        tr(
            "Runs Claude Code or Codex, already installed and signed in, on this canvas through Scrim's MCP server. Your agent's plan pays for it; Scrim keeps no keys.",
        ),
        t.text_secondary,
    );
    ui.add_space(6.0);
    widgets::label(ui, tr("Stop a run after"), t.text);
    let labels: Vec<String> = chat::TIME_LIMITS
        .iter()
        .map(|m| if *m == chat::DEFAULT_TIME_LIMIT { trf("{n} min (default)", &[("n", m)]) } else { trf("{n} min", &[("n", m)]) })
        .collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    let current = chat::TIME_LIMITS.iter().position(|m| *m == app.session.prefs.settings.chat_time_limit_mins).unwrap_or(1);
    if let Some(i) = widgets::segmented_text(ui, &labels, current) {
        app.run("pref.chatTimeLimit", json!({"minutes": chat::TIME_LIMITS[i]}));
    }
    widgets::label(ui, tr("What the agent made by then stays, and Undo takes it back."), t.text_secondary);
    ui.add_space(6.0);
    let allowed = app.session.prefs.settings.chat_allowed.clone();
    if allowed.is_empty() {
        widgets::label(ui, tr("No agent has been allowed to run yet: each asks the first time."), t.text_secondary);
        return;
    }
    widgets::label(ui, tr("Allowed to run"), t.text);
    for a in &allowed {
        let name = headless::profile(&a.agent).map_or(a.agent.as_str(), |p| p.name.as_str());
        widgets::label(ui, &format!("{name}: {}", a.path), t.text_secondary);
    }
    if widgets::button(ui, tr("Forget"), false, true).on_hover_text(tr("Each agent asks again before it next runs")).clicked() {
        app.run("chat.forget-agents", json!({}));
    }
}

/// The first-use confirmation: the exact command and the binary's path. Only the person answers it.
pub fn dialogs(app: &mut FigApp, ctx: &Context) {
    let Some((agent, path, _)) = app.ui.agent_view.chat.confirm.clone() else { return };
    let Some(p) = headless::profile(&agent) else { return };
    let t = Tokens::get(ctx);
    let url = app.session.agents.mcp.url().unwrap_or_default();
    let command = p.display(&path, &url, app.session.prefs.settings.mcp.require_auth);
    let (mut go, mut cancel, mut open) = (false, false, true);
    crate::dialogs::window(&trf("Run {agent}?", &[("agent", &p.name)]), ctx).open(&mut open).show(ctx, |ui| {
        ui.set_width(460.0);
        ui.label(RichText::new(trf("Scrim will run {agent} on this computer to make your change, with only Scrim's MCP server and its tools. You're asked once for this agent and this program, and again if the program changes.", &[("agent", &p.name)])).font(regular(11.0)).color(t.text_secondary));
        ui.add_space(6.0);
        ui.label(RichText::new(trf("Program: {path}", &[("path", &path)])).font(regular(11.0)).color(t.text));
        ui.add_space(4.0);
        ui.label(RichText::new(&command).monospace().color(t.text));
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            go = widgets::button(ui, tr("Run"), true, true).clicked();
            cancel = widgets::button(ui, tr("Cancel"), false, true).clicked();
        });
    });
    if go {
        app.run("chat.allow-agent", json!({"agent": agent, "path": path}));
    } else if cancel || !open {
        app.run("chat.cancel", json!({}));
    }
}

#[cfg(test)]
#[path = "chat_settings_tests.rs"]
mod settings_tests;
#[cfg(test)]
#[path = "chat_ui_tests.rs"]
pub(crate) mod tests;
