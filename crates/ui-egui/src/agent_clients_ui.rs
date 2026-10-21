//! Connect other agents (Stage J2, `docs/design/connect-any-agent.md`): every MCP client with a
//! profile in `figcraft_engine::cmd::agent_clients`, in the AI tab and Settings ▸ MCP (one
//! implementation, below Connect Claude Code and Claude Desktop, which keep their own flows).
//!
//! - **The list.** Clients found on this computer come first (their program on the `PATH`, or their
//!   configuration folder); the rest are under More. Each row shows:
//!   - the status read from the client's own configuration: Configured, Not configured, Out of
//!     date, Can't read. Never "Connected", which only the endpoint can see;
//!   - how the profile was verified, and when;
//!   - Connect.
//! - **The panel** shows exactly what will happen, with the live address:
//!   - the client's own command (an argument list, run without a shell), or the file and its entry;
//!   - Run or Write, and Copy.
//!   - Copy only when Scrim can't do it itself: the program isn't installed, the file's path isn't
//!     documented, the format is YAML, or the client has no way to take the token.
//! - **One confirmation** before anything runs or is written (`mcp.write-client`,
//!   `mcp.reregister-client`, `mcp.remove-old-client-entry`). They are dialogs, which an agent's
//!   commands never open. A token written in plain text is said in the panel and the confirmation.
//! - **Writes** are `agent_clients::apply`: a missing file is made, an invalid one is left alone,
//!   only the scrim entry changes, a backup is kept, and the file is replaced by a rename.
//! - Scrim never restarts or closes the client: it says to restart it.
//! - **Any other agent:** the address, JSON and TOML entries and the stdio command to copy, and why
//!   an agent in the cloud can't reach this computer.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

use egui::{Context, RichText, Ui};
use figcraft_engine::cmd::agent_clients::{self as ac, KEY};
use figcraft_engine::cmd::claude_desktop::Change;
use serde_json::{Value, json};

use crate::i18n::{tr, trf};
use crate::theme::{Tokens, regular};
use crate::{FigApp, ProgramRun, widgets};

/// A confirmation being asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Confirm {
    Write,
    Reregister,
    RemoveOld,
}

/// How Scrim makes a change to a client.
#[derive(Clone, Debug, PartialEq)]
enum How {
    /// Run the client's program (its path), with each argument list in turn.
    Run(String, Vec<Vec<String>>),
    /// Make these changes to its file.
    Write(PathBuf, Vec<Change>),
    /// Scrim can't: the person copies.
    Copy,
}

/// A client's program running.
struct Running {
    client: String,
    confirm: Confirm,
    program: String,
    rx: ProgramRun,
    rest: VecDeque<Vec<String>>,
}

/// The section's state (one per window, kept in `connect_flow::Flow`).
#[derive(Default)]
pub struct Others {
    /// The client whose panel is open.
    open: Option<String>,
    /// More is expanded.
    more: bool,
    /// The card for any other agent is expanded.
    any: bool,
    confirm: Option<(String, Confirm)>,
    running: Option<Running>,
    /// What the last change said, for which client, and that client's state right after it: shown
    /// while the state stays so.
    outcome: Option<(String, Result<String, String>, String)>,
    /// Each client's status as last read, and when (egui seconds): read again every two seconds.
    seen: HashMap<String, (f64, Value)>,
    /// The clients found on this computer, and when they were looked for.
    found: Option<(f64, Vec<String>)>,
}

fn now_secs() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Client `id`'s report (the cached read, or a fresh one); `None` while the endpoint is stopped.
fn report(app: &mut FigApp, id: &str, now: f64) -> Option<Value> {
    if let Some((at, v)) = app.ui.connect.others.seen.get(id)
        && now - at < 2.0
    {
        return Some(v.clone());
    }
    let v = ac::report(&app.session, id).ok()?;
    app.ui.connect.others.seen.insert(id.to_string(), (now, v.clone()));
    Some(v)
}

/// The clients found on this computer: a program on the `PATH`, or (in the desktop app) a folder of
/// their own. Looked for again every ten seconds; nothing is run to find them.
fn found(app: &mut FigApp, now: f64) -> Vec<String> {
    if let Some((at, v)) = &app.ui.connect.others.found
        && now - at < 10.0
    {
        return v.clone();
    }
    let env = app.session.agents.mcp.client_env.clone();
    let v: Vec<String> = ac::profiles()
        .iter()
        .filter(|p| p.builtin.is_none())
        .filter(|p| {
            p.detect.programs.iter().any(|n| app.services.find_program(n).is_some())
                || env
                    .as_ref()
                    .is_some_and(|e| p.detect.dirs.iter().filter_map(|d| ac::resolve(d, e)).any(|d| figcraft_engine::file_access::exists(&d)))
        })
        .map(|p| p.id.clone())
        .collect();
    app.ui.connect.others.found = Some((now, v.clone()));
    v
}

fn strings(v: &Value) -> Option<Vec<String>> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
}

fn name(r: &Value) -> String {
    r["display"].as_str().unwrap_or_default().to_string()
}

fn old(r: &Value) -> Vec<String> {
    strings(&r["old"]).unwrap_or_default()
}

/// How Scrim would make change `c` to the client in report `r`: its own command when it is
/// installed (the first choice, as its docs describe), else its file when Scrim may write it and
/// its docs name it, else Copy.
fn how(app: &FigApp, r: &Value, c: Confirm) -> How {
    let program = r["cli"]["program"].as_str().and_then(|n| app.services.find_program(n));
    let file = r["file"]["path"].as_str().filter(|_| r["file"]["writable"] == true && r["file"]["documented"] == true).map(PathBuf::from);
    match c {
        Confirm::Write | Confirm::Reregister => {
            if let (Some(p), Some(add)) = (&program, strings(&r["cli"]["add"])) {
                match (c, strings(&r["cli"]["remove"])) {
                    (Confirm::Write, _) => return How::Run(p.clone(), vec![add]),
                    (_, Some(remove)) => return How::Run(p.clone(), vec![remove, add]),
                    _ => {}
                }
            }
            match file {
                Some(path) => How::Write(path, vec![Change::Set(KEY.into(), r["file"]["entry"].clone())]),
                None => How::Copy,
            }
        }
        Confirm::RemoveOld => {
            let keys = old(r);
            if let Some(path) = file {
                return How::Write(path, keys.into_iter().map(Change::Remove).collect());
            }
            let cli = r["id"].as_str().and_then(ac::profile).and_then(|p| p.cli.as_ref());
            match (program, cli) {
                (Some(p), Some(cli)) => {
                    let runs: Vec<Vec<String>> = keys.iter().filter_map(|k| ac::cli_remove(cli, k)).collect();
                    if runs.is_empty() || runs.len() < keys.len() { How::Copy } else { How::Run(p, runs) }
                }
                _ => How::Copy,
            }
        }
    }
}

/// Whether `how` puts the token itself into the client's configuration.
fn plain_token(r: &Value, how: &How) -> bool {
    r["token"]["on"] == true
        && match how {
            How::Run(..) => r["token"]["cliPlain"] == true,
            How::Write(..) => r["token"]["filePlain"] == true,
            How::Copy => false,
        }
}

/// The command lines of a run, as the person would type them.
fn lines(r: &Value, runs: &[Vec<String>]) -> String {
    let program = r["cli"]["program"].as_str().unwrap_or_default();
    runs.iter().map(|a| ac::command_line(program, a)).collect::<Vec<_>>().join("\n")
}

/// Open client `id`'s panel (`mcp.connect-client`).
fn open(app: &mut FigApp, id: &str) {
    let o = &mut app.ui.connect.others;
    o.open = Some(id.to_string());
    o.outcome = None;
    o.seen.remove(id);
    if !o.found.as_ref().is_some_and(|(_, f)| f.iter().any(|x| x == id)) {
        o.more = true;
    }
}

/// Act on the section's requests (`{client}`); `false` when `kind` isn't one.
pub fn request(app: &mut FigApp, kind: &str, data: &Value) -> bool {
    let Some(id) = data.get("client").and_then(Value::as_str).map(str::to_string) else { return false };
    let c = match kind {
        ac::CONNECT_REQUEST => {
            open(app, &id);
            return true;
        }
        ac::WRITE_DIALOG => Confirm::Write,
        ac::REREGISTER_DIALOG => Confirm::Reregister,
        ac::REMOVE_OLD_DIALOG => Confirm::RemoveOld,
        _ => return false,
    };
    open(app, &id);
    app.ui.connect.others.confirm = Some((id, c));
    true
}

/// What happened, said once the change is done.
fn finish(app: &mut FigApp, id: &str, c: Confirm, result: Result<(), String>, backup: Option<PathBuf>) {
    app.ui.connect.others.seen.remove(id);
    let r = ac::report(&app.session, id).unwrap_or(Value::Null);
    let after = r["state"].as_str().unwrap_or_default().to_string();
    let n = name(&r);
    let kept = backup.and_then(|b| b.file_name().map(|f| f.to_string_lossy().into_owned()));
    let said = match (result, c) {
        (Err(e), _) => Err(e),
        (Ok(()), Confirm::RemoveOld) => Ok(trf("The old entry was removed from {name}.", &[("name", &n)])),
        (Ok(()), _) => Ok(match kept {
            Some(k) => {
                trf("Scrim was added to {name}. Restart {name} to use it. The previous file was kept as {file}.", &[("name", &n), ("file", &k)])
            }
            None => trf("Scrim was added to {name}. Restart {name} to use it.", &[("name", &n)]),
        }),
    };
    app.ui.connect.others.outcome = Some((id.to_string(), said, after));
}

/// Do what the person confirmed.
fn act(app: &mut FigApp, id: &str, c: Confirm, r: &Value) {
    match how(app, r, c) {
        How::Run(program, runs) => {
            let mut rest: VecDeque<Vec<String>> = runs.into();
            let Some(first) = rest.pop_front() else { return };
            let rx = app.services.run_program(&program, &first);
            app.ui.connect.others.running = Some(Running { client: id.to_string(), confirm: c, program, rx, rest });
        }
        How::Write(path, changes) => {
            let Some(f) = ac::profile(id).and_then(|p| p.file.as_ref()) else { return };
            let (mut backup, mut result) = (None, Ok(()));
            for ch in &changes {
                match ac::apply(f, &path, ch, now_secs()) {
                    // The first backup is the file as the person had it.
                    Ok(w) => backup = backup.or(w.backup),
                    Err(e) => {
                        result = Err(e);
                        break;
                    }
                }
            }
            finish(app, id, c, result, backup);
        }
        How::Copy => {}
    }
}

/// Take a finished program's output and go on with the next, or say what happened.
fn poll(app: &mut FigApp, ctx: &Context) {
    let Some(run) = &app.ui.connect.others.running else { return };
    let out = match run.rx.try_recv() {
        Ok(out) => out,
        Err(std::sync::mpsc::TryRecvError::Empty) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }
        Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(tr("The command stopped without an answer.").to_string()),
    };
    let Some(mut run) = app.ui.connect.others.running.take() else { return };
    ctx.request_repaint();
    let failed = match out {
        Ok(o) if o.success() => None,
        Ok(o) => Some(if o.stderr.trim().is_empty() { o.stdout.trim().to_string() } else { o.stderr.trim().to_string() }),
        Err(e) => Some(e),
    };
    if let Some(e) = failed {
        finish(app, &run.client, run.confirm, Err(e), None);
    } else if let Some(next) = run.rest.pop_front() {
        run.rx = app.services.run_program(&run.program, &next);
        app.ui.connect.others.running = Some(run);
    } else {
        finish(app, &run.client, run.confirm, Ok(()), None);
    }
}

fn mono(ui: &mut Ui, text: &str) {
    let t = Tokens::get(ui.ctx());
    egui::Frame::NONE
        .fill(t.field)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::same(8))
        .outer_margin(egui::Margin::symmetric(16, 0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).monospace().color(t.text));
        });
}

/// The status row's words for a state.
fn state_label(state: &str) -> &'static str {
    match state {
        "configured" => tr("Configured"),
        "out of date" => tr("Out of date"),
        "unreadable" => tr("Can't read its configuration"),
        "not configured" => tr("Not configured"),
        _ => tr("Not checked"),
    }
}

/// How the profile was verified, and when.
fn verification(r: &Value) -> String {
    let date = r["verified"].as_str().unwrap_or_default();
    match r["verification"].as_str() {
        Some("verified-live") => trf("Verified live · {date}", &[("date", &date)]),
        Some("docs-only") => trf("From its documentation only · {date}", &[("date", &date)]),
        _ => tr("Untested").to_string(),
    }
}

/// The section: the clients found, More, and the card for any other agent.
pub fn section(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    poll(app, ui.ctx());
    widgets::section_header(ui, tr("Connect other agents"), &[], false);
    if app.session.agents.mcp.url().is_none() {
        widgets::note(ui, tr("The MCP endpoint isn't running: restart it from Settings, under MCP."), t.text_secondary);
        return;
    }
    let now = ui.ctx().input(|i| i.time);
    let here = found(app, now);
    for id in &here {
        row(app, ui, id, now);
    }
    let rest: Vec<String> = ac::profiles().iter().filter(|p| p.builtin.is_none() && !here.contains(&p.id)).map(|p| p.id.clone()).collect();
    let more = app.ui.connect.others.more;
    crate::right::row(ui, 32.0, |ui| {
        let label = if more { tr("Fewer").to_string() } else { trf("More ({count})", &[("count", &rest.len())]) };
        if widgets::button(ui, &label, false, !rest.is_empty()).clicked() {
            app.ui.connect.others.more = !more;
        }
        if widgets::button(ui, tr("Other agent"), false, true).clicked() {
            app.ui.connect.others.any = !app.ui.connect.others.any;
        }
    });
    if more {
        for id in &rest {
            row(app, ui, id, now);
        }
    }
    if app.ui.connect.others.any {
        any_agent(app, ui);
    }
}

/// One client: its status, how it was verified, and Connect (or its panel).
fn row(app: &mut FigApp, ui: &mut Ui, id: &str, now: f64) {
    let t = Tokens::get(ui.ctx());
    let Some(r) = report(app, id, now) else { return };
    let n = name(&r);
    let state = r["state"].as_str().unwrap_or_default().to_string();
    let label = state_label(&state);
    let color = match state.as_str() {
        "configured" => t.success,
        "out of date" => t.warning,
        "unreadable" => t.danger,
        _ => t.text_secondary,
    };
    let open = app.ui.connect.others.open.as_deref() == Some(id);
    crate::right::row(ui, 28.0, |ui| {
        let (dot, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().circle_filled(dot.center(), 4.0, color);
        let resp = ui.label(RichText::new(format!("{n} · {label}")).color(t.text).font(regular(12.0)));
        crate::a11y::status(&resp, label, &trf("{name}: {state}", &[("name", &n), ("state", &label)]));
        if !open && widgets::button(ui, tr("Connect"), false, true).clicked() {
            app.run("mcp.connect-client", json!({"client": id}));
        }
    });
    widgets::note(ui, &verification(&r), t.text_secondary);
    if open {
        panel(app, ui, &r);
    }
}

/// Why Scrim can't make the change itself, when it can't.
fn why_copy(app: &FigApp, r: &Value, n: &str) -> Option<String> {
    let program = r["cli"]["program"].as_str();
    if let Some(p) = program
        && app.services.find_program(p).is_none()
        && r["cli"]["add"].is_array()
    {
        return Some(trf("{name}'s {program} command wasn't found on this computer.", &[("name", &n), ("program", &p)]));
    }
    if r["file"].is_object() && r["file"]["writable"] == true {
        if r["file"]["path"].is_null() {
            return Some(tr("This version of Scrim can't read or change other apps' configurations.").to_string());
        }
        if r["file"]["documented"] == false {
            return Some(trf("{name}'s documentation doesn't name this file on this system, so Scrim doesn't write it.", &[("name", &n)]));
        }
    }
    None
}

/// A client's panel: what will happen, Run or Write and Copy, and what happened.
fn panel(app: &mut FigApp, ui: &mut Ui, r: &Value) {
    let t = Tokens::get(ui.ctx());
    let id = r["id"].as_str().unwrap_or_default().to_string();
    let n = name(r);
    let state = r["state"].as_str().unwrap_or_default().to_string();
    let c = if state == "out of date" { Confirm::Reregister } else { Confirm::Write };
    let h = how(app, r, c);
    match &h {
        How::Run(_, runs) => {
            widgets::note(ui, &trf("Scrim will run {name}'s own command, which adds Scrim for your user:", &[("name", &n)]), t.text_secondary);
            mono(ui, &lines(r, runs));
        }
        How::Write(path, _) => {
            widgets::note(ui, &trf("Scrim will add this entry to {name}'s configuration file:", &[("name", &n)]), t.text_secondary);
            mono(ui, &path.to_string_lossy());
            ui.add_space(4.0);
            mono(ui, r["file"]["text"].as_str().unwrap_or_default());
        }
        How::Copy => {
            widgets::note(ui, &trf("Add this to {name} yourself:", &[("name", &n)]), t.text_secondary);
            mono(ui, &ac::copy_text(r));
            if let Some(why) = why_copy(app, r, &n) {
                widgets::note(ui, &why, t.text_secondary);
            }
        }
    }
    ui.add_space(6.0);
    for note in strings(&r["notes"]).unwrap_or_default() {
        widgets::note(ui, &note, t.text_secondary);
    }
    if plain_token(r, &h) {
        widgets::note(
            ui,
            &trf(
                "MCP authentication is on: the token itself is written into {name}'s configuration, where anything that can read it can see it.",
                &[("name", &n)],
            ),
            t.warning,
        );
    } else if r["token"]["on"] == true && h != How::Copy {
        widgets::note(
            ui,
            &trf(
                "MCP authentication is on: {name} reads the token from the {var} environment variable. Set it to this window's token where {name} runs.",
                &[("name", &n), ("var", &ac::TOKEN_VAR)],
            ),
            t.text_secondary,
        );
    }
    match state.as_str() {
        "unreadable" => widgets::note(
            ui,
            &trf(
                "{name}'s configuration can't be read, so Scrim won't change it: {error}. Fix or move the file, then try again.",
                &[("name", &n), ("error", &r["error"].as_str().unwrap_or_default())],
            ),
            t.danger,
        ),
        "configured" => widgets::note(ui, &trf("{name} already has Scrim at this address.", &[("name", &n)]), t.text),
        "out of date" => {
            widgets::note(ui, tr("Registered address is out of date"), t.warning);
            let (old, new) = (r["registered"].as_str().unwrap_or_default(), r["url"].as_str().unwrap_or_default());
            widgets::note(
                ui,
                &trf("{name} points to {old}; this window is at {new}.", &[("name", &n), ("old", &old), ("new", &new)]),
                t.text_secondary,
            );
        }
        _ => {}
    }
    let busy = app.ui.connect.others.running.is_some();
    let can = h != How::Copy && matches!(state.as_str(), "not configured" | "out of date" | "unknown");
    crate::right::row(ui, 32.0, |ui| {
        if can {
            let (label, cmd) = match (c, &h) {
                (Confirm::Reregister, _) => (tr("Re-register"), "mcp.reregister-client"),
                (_, How::Run(..)) => (tr("Run"), "mcp.write-client"),
                _ => (tr("Write"), "mcp.write-client"),
            };
            if widgets::button(ui, label, true, !busy).clicked() {
                app.run(cmd, json!({"client": id}));
            }
        }
        if widgets::button(ui, tr("Copy"), !can, true).clicked() {
            app.run("mcp.copy-client-entry", json!({"client": id}));
        }
        if let Some(doc) = r["docs"][0].as_str()
            && widgets::button(ui, tr("Documentation"), false, true).clicked()
        {
            app.services.open_url(doc);
        }
        if widgets::button(ui, tr("Close"), false, !busy).clicked() {
            app.ui.connect.others.open = None;
        }
    });
    if busy {
        widgets::note(ui, &trf("Running {name}'s command…", &[("name", &n)]), t.text_secondary);
    }
    match &app.ui.connect.others.outcome {
        // (In the text colour, like the other connect flows.)
        Some((who, Ok(m), after)) if *who == id && *after == state => widgets::note(ui, m, t.text),
        Some((who, Err(e), _)) if *who == id => widgets::note(ui, &trf("It didn't work: {error}", &[("error", e)]), t.danger),
        _ => {}
    }
    let keys = old(r);
    if !keys.is_empty() && state != "unreadable" {
        widgets::note(
            ui,
            &trf("{name} still has the old {keys} entry from before the rename.", &[("name", &n), ("keys", &keys.join(", "))]),
            t.text_secondary,
        );
        if how(app, r, Confirm::RemoveOld) != How::Copy {
            crate::right::row(ui, 32.0, |ui| {
                if widgets::button(ui, tr("Remove the old entry"), false, !busy).clicked() {
                    app.run("mcp.remove-old-client-entry", json!({"client": id}));
                }
            });
        }
    }
}

/// The card for any MCP client: the address, the JSON and TOML entries, the stdio command.
fn any_agent(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    widgets::note(ui, tr("Any MCP client on this computer can use this canvas. Give it one of these:"), t.text_secondary);
    let url = app.session.agents.mcp.url().unwrap_or_default();
    let parts: [(&str, Option<&str>); 4] =
        [(tr("Address"), None), ("JSON", Some("json")), ("TOML", Some("toml")), (tr("Command (stdio)"), Some("stdio"))];
    for (label, format) in parts {
        let text = match format {
            Some(f) => ac::generic(&app.session, f).unwrap_or_default(),
            None => url.clone(),
        };
        crate::right::row(ui, 28.0, |ui| {
            ui.label(RichText::new(label).color(t.text).font(regular(12.0)));
            if widgets::button(ui, tr("Copy"), false, true).clicked() {
                let _ = match format {
                    Some(f) => app.run("mcp.copy-agent-snippet", json!({"format": f})),
                    None => app.run("mcp.copy-address", json!({})),
                };
            }
        });
        mono(ui, text.trim_end());
        ui.add_space(4.0);
    }
    if app.session.prefs.settings.mcp.require_auth {
        widgets::note(
            ui,
            &trf(
                "MCP authentication is on: these read the token from the {var} environment variable. Set it where the agent runs.",
                &[("var", &ac::TOKEN_VAR)],
            ),
            t.text_secondary,
        );
    }
    widgets::note(
        ui,
        tr(
            "Agents that run in the cloud (in a browser, or on another company's server) can't reach this computer's local address. They would need a relay, which Scrim doesn't have yet.",
        ),
        t.text_secondary,
    );
}

/// The confirmations. Only the person can answer them.
pub fn dialogs(app: &mut FigApp, ctx: &Context) {
    poll(app, ctx);
    let Some((id, c)) = app.ui.connect.others.confirm.clone() else { return };
    let t = Tokens::get(ctx);
    // Read the file again: what is asked about is what is there now.
    app.ui.connect.others.seen.remove(&id);
    let Some(r) = report(app, &id, ctx.input(|i| i.time)) else {
        app.ui.connect.others.confirm = None;
        return;
    };
    let h = how(app, &r, c);
    let state = r["state"].as_str().unwrap_or_default();
    let keys = old(&r);
    // Nothing to do (or nothing it may do): don't ask.
    let pointless = h == How::Copy
        || state == "unreadable"
        || app.ui.connect.others.running.is_some()
        || match c {
            Confirm::Write => !matches!(state, "not configured" | "unknown"),
            Confirm::Reregister => state != "out of date",
            Confirm::RemoveOld => keys.is_empty(),
        };
    if pointless {
        app.ui.connect.others.confirm = None;
        return;
    }
    let n = name(&r);
    let title = match c {
        Confirm::Write => trf("Add Scrim to {name}?", &[("name", &n)]),
        Confirm::Reregister => trf("Re-register Scrim in {name}?", &[("name", &n)]),
        Confirm::RemoveOld => tr("Remove the old entry?").to_string(),
    };
    let (body, detail) = match (&h, c) {
        (How::Run(_, runs), Confirm::RemoveOld) => {
            (trf("Scrim will run {name}'s own command to remove the old {keys} entry.", &[("name", &n), ("keys", &keys.join(", "))]), lines(&r, runs))
        }
        (How::Run(_, runs), Confirm::Reregister) => (
            trf(
                "Scrim will run {name}'s own command to replace its scrim entry, which points to {old}, with this window's address.",
                &[("name", &n), ("old", &r["registered"].as_str().unwrap_or_default())],
            ),
            lines(&r, runs),
        ),
        (How::Run(_, runs), _) => {
            (trf("Scrim will run {name}'s own command, without a shell. It changes nothing else.", &[("name", &n)]), lines(&r, runs))
        }
        (How::Write(path, _), Confirm::RemoveOld) => (
            trf(
                "Scrim will remove the old {keys} entry from {name}'s configuration. It keeps a copy of the file first.",
                &[("name", &n), ("keys", &keys.join(", "))],
            ),
            path.to_string_lossy().into_owned(),
        ),
        (How::Write(path, _), Confirm::Reregister) => (
            trf(
                "Scrim will replace {name}'s scrim entry, which points to {old}, with this window's address. It keeps a copy of the file first.",
                &[("name", &n), ("old", &r["registered"].as_str().unwrap_or_default())],
            ),
            path.to_string_lossy().into_owned(),
        ),
        (How::Write(path, _), _) => (
            tr("Scrim will add its entry to this file. It keeps a copy of the file first, and changes nothing else in it.").to_string(),
            path.to_string_lossy().into_owned(),
        ),
        (How::Copy, _) => return,
    };
    let plain = c != Confirm::RemoveOld && plain_token(&r, &h);
    let go_label = match c {
        _ if plain => tr("Write the token"),
        Confirm::Write if matches!(h, How::Run(..)) => tr("Run"),
        Confirm::Write => tr("Write"),
        Confirm::Reregister => tr("Re-register"),
        Confirm::RemoveOld => tr("Remove"),
    };
    let (mut go, mut cancel, mut open) = (false, false, true);
    crate::dialogs::window(&title, ctx).open(&mut open).show(ctx, |ui| {
        ui.set_width(440.0);
        ui.label(RichText::new(&body).font(regular(11.0)).color(t.text_secondary));
        ui.add_space(6.0);
        ui.label(RichText::new(&detail).monospace().color(t.text));
        if plain {
            ui.add_space(6.0);
            let warn = trf(
                "MCP authentication is on, and {name} has no documented way to read the token from elsewhere: the token itself will be stored in its configuration as plain text.",
                &[("name", &n)],
            );
            ui.label(RichText::new(warn).font(regular(11.0)).color(t.warning));
        }
        ui.add_space(6.0);
        ui.label(RichText::new(trf("Scrim doesn't restart {name}: restart it yourself afterwards.", &[("name", &n)])).font(regular(11.0)).color(t.text_secondary));
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            go = widgets::button(ui, go_label, true, true).clicked();
            cancel = widgets::button(ui, tr("Cancel"), false, true).clicked();
        });
    });
    if go {
        app.ui.connect.others.confirm = None;
        act(app, &id, c, &r);
    } else if cancel || !open {
        app.ui.connect.others.confirm = None;
    }
}

#[cfg(test)]
#[path = "agent_clients_ui_tests.rs"]
mod tests;
