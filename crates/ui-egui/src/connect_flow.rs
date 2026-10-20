//! Connect Claude Code (`docs/design/settings-and-connect.md`, C2): one implementation, shown in the
//! AI tab and in Settings ▸ AI & agents and ▸ MCP.
//!
//! The flow:
//! 1. **The command.** "Connect Claude Code" (`mcp.connect-claude-code`) shows the exact command:
//!    `claude mcp add --transport http --scope user scrim http://127.0.0.1:<port>/mcp`. It offers Run and
//!    Copy (`mcp.copy-command`); if Claude Code's `claude` isn't found, Copy only.
//! 2. **One confirmation.** Run (`mcp.run-connect`) asks once.
//! 3. **What's there already.** Only after the confirmation does Scrim run anything. It asks
//!    `claude mcp get scrim` and `claude mcp get figcraft` what Claude Code has. A `scrim` entry
//!    pointing elsewhere is offered for **Replace**, never overwritten silently.
//! 4. **The command runs.** `claude` gets an argument list, never a shell line. Success, or the real
//!    error text, is shown. An old `figcraft` entry can then be removed (`mcp.remove-old-entry`, which
//!    asks again).
//!
//! **Agents can't run anything.** The confirmations are dialogs, and an agent's commands never open
//! one, so an agent can't make Scrim run a program. Nothing here touches the document.

use egui::{Context, RichText, Ui};
use figcraft_engine::cmd::mcp::{REMOVE_OLD_DIALOG, REREGISTER_DIALOG, RUN_DIALOG};
use serde_json::json;

use crate::i18n::{tr, trf};
use crate::theme::{Tokens, regular};
use crate::{FigApp, ProgramOutput, ProgramRun, widgets};

/// An MCP entry Claude Code already has (from `claude mcp get <name>`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    /// `local`, `user` or `project`, when the output says.
    pub scope: Option<String>,
    /// Where it points: its URL, or its command line.
    pub target: String,
}

/// Claude Code's answer to `claude mcp get <name>`: `None` when it has no such entry.
pub fn parse_get(out: &ProgramOutput) -> Option<Entry> {
    if !out.success() {
        return None;
    }
    let mut e = Entry::default();
    for line in out.stdout.lines().map(str::trim) {
        if let Some(v) = line.strip_prefix("Scope:") {
            let v = v.trim().to_ascii_lowercase();
            e.scope = ["local", "user", "project"].iter().find(|s| v.starts_with(*s)).map(|s| s.to_string());
        } else if let Some(v) = line.strip_prefix("URL:").or_else(|| line.strip_prefix("Command:")) {
            e.target = v.trim().to_string();
        }
    }
    Some(e)
}

/// What the flow waits for.
enum Waiting {
    /// `claude mcp get scrim`.
    GetScrim,
    /// `claude mcp get figcraft` (with what `get scrim` found).
    GetOld(Option<Entry>),
    /// Removing the `scrim` entry before adding (Replace).
    Remove,
    /// `claude mcp add …`.
    Add,
    /// Removing the old `figcraft` entry.
    RemoveOld,
}

#[derive(Default)]
enum Stage {
    #[default]
    Ready,
    Running(ProgramRun, Waiting),
    /// A `scrim` entry points elsewhere: Replace it?
    Asking(Entry),
    Done(Result<String, String>),
}

/// The flow's state (one per window).
#[derive(Default)]
pub struct Flow {
    pub open: bool,
    stage: Stage,
    confirm_run: bool,
    confirm_remove_old: bool,
    /// Re-register: the out-of-date entry the confirmation replaces (C1.2).
    confirm_reregister: Option<Entry>,
    /// Claude Code's old `figcraft` entry, when the check found one.
    old: Option<Entry>,
    /// What Claude Code's `scrim` entry says (`mcp_registration`).
    pub(crate) registered: crate::mcp_registration::Check,
}

impl Flow {
    /// The flow is running `claude`.
    pub(crate) fn busy(&self) -> bool {
        matches!(self.stage, Stage::Running(..))
    }
}

/// Open the flow (`mcp.connect-claude-code`).
pub fn open(app: &mut FigApp) {
    let registered = std::mem::take(&mut app.ui.connect.registered);
    app.ui.connect = Flow { open: true, registered, ..Flow::default() };
}

/// Act on the confirmations' requests; `false` when `kind` isn't one.
pub fn request(app: &mut FigApp, kind: &str) -> bool {
    match kind {
        RUN_DIALOG => {
            app.ui.connect.open = true;
            app.ui.connect.confirm_run = true;
        }
        REMOVE_OLD_DIALOG => app.ui.connect.confirm_remove_old = app.ui.connect.old.is_some(),
        REREGISTER_DIALOG => app.ui.connect.confirm_reregister = crate::mcp_registration::out_of_date(app),
        _ => return false,
    }
    true
}

fn status(app: &FigApp) -> serde_json::Value {
    let now = web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).ok();
    figcraft_engine::cmd::mcp::status(&app.session, now)
}

fn run(app: &mut FigApp, args: Vec<String>, waiting: Waiting) {
    match app.services.find_program("claude") {
        Some(claude) => app.ui.connect.stage = Stage::Running(app.services.run_program(&claude, &args), waiting),
        None => app.ui.connect.stage = Stage::Done(Err(tr("Claude Code's claude command wasn't found on this computer.").into())),
    }
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// The arguments that add Scrim, from the endpoint's status.
fn add_args(app: &FigApp) -> Option<Vec<String>> {
    status(app)["args"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
}

fn step(app: &mut FigApp, waiting: Waiting, out: Result<ProgramOutput, String>) {
    let failed = |o: &Result<ProgramOutput, String>| match o {
        Ok(o) if o.success() => None,
        Ok(o) => Some(if o.stderr.trim().is_empty() { o.stdout.trim().to_string() } else { o.stderr.trim().to_string() }),
        Err(e) => Some(e.clone()),
    };
    match waiting {
        Waiting::GetScrim => {
            let scrim = out.as_ref().ok().and_then(parse_get);
            run(app, strings(&["mcp", "get", "figcraft"]), Waiting::GetOld(scrim));
        }
        Waiting::GetOld(scrim) => {
            app.ui.connect.old = out.as_ref().ok().and_then(parse_get);
            let url = status(app)["url"].as_str().unwrap_or_default().to_string();
            match scrim {
                Some(e) if e.target == url && e.scope.as_deref() == Some("user") => {
                    app.ui.connect.stage = Stage::Done(Ok(tr("Claude Code already has Scrim at this address, for every project.").into()));
                }
                Some(e) => app.ui.connect.stage = Stage::Asking(e),
                None => match add_args(app) {
                    Some(args) => run(app, args, Waiting::Add),
                    None => {
                        app.ui.connect.stage = Stage::Done(Err(tr("The MCP endpoint isn't running: restart it from Settings, under MCP.").into()))
                    }
                },
            }
        }
        Waiting::Remove => match failed(&out) {
            None => match add_args(app) {
                Some(args) => run(app, args, Waiting::Add),
                None => app.ui.connect.stage = Stage::Done(Err(tr("The MCP endpoint isn't running: restart it from Settings, under MCP.").into())),
            },
            Some(e) => app.ui.connect.stage = Stage::Done(Err(e)),
        },
        Waiting::Add => {
            crate::mcp_registration::forget(app);
            app.ui.connect.stage = Stage::Done(match failed(&out) {
                None => Ok(tr("Scrim was added to Claude Code for every project. Start a new Claude Code session to use it.").into()),
                Some(e) => Err(e),
            });
        }
        Waiting::RemoveOld => {
            app.ui.connect.stage = Stage::Done(match failed(&out) {
                None => {
                    app.ui.connect.old = None;
                    Ok(tr("The old figcraft entry was removed from Claude Code.").into())
                }
                Some(e) => Err(e),
            });
        }
    }
}

/// Take a finished program's output and go on.
fn poll(app: &mut FigApp, ctx: &Context) {
    let Stage::Running(rx, _) = &app.ui.connect.stage else { return };
    match rx.try_recv() {
        Ok(out) => {
            if let Stage::Running(_, waiting) = std::mem::take(&mut app.ui.connect.stage) {
                step(app, waiting, out);
            }
            ctx.request_repaint();
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            app.ui.connect.stage = Stage::Done(Err(tr("The command stopped without an answer.").into()))
        }
    }
}

/// The flow's panel: the command, Run and Copy, progress, and the outcome.
pub fn panel(app: &mut FigApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    poll(app, ui.ctx());
    let st = status(app);
    let Some(command) = st["command"].as_str().map(str::to_string) else {
        widgets::note(ui, tr("The MCP endpoint isn't running: restart it from Settings, under MCP."), t.text_secondary);
        return;
    };
    widgets::note(ui, tr("Scrim will run this command, which adds Scrim to Claude Code for every project:"), t.text_secondary);
    egui::Frame::NONE
        .fill(t.field)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::same(8))
        .outer_margin(egui::Margin::symmetric(16, 0))
        .show(ui, |ui| {
            ui.label(RichText::new(&command).monospace().color(t.text));
        });
    ui.add_space(6.0);
    let found = app.services.find_program("claude").is_some();
    let busy = matches!(app.ui.connect.stage, Stage::Running(..));
    crate::right::row(ui, 32.0, |ui| {
        if found && widgets::button(ui, tr("Run"), true, !busy).clicked() {
            app.run("mcp.run-connect", json!({}));
        }
        if widgets::button(ui, tr("Copy"), !found, true).clicked() {
            app.run("mcp.copy-command", json!({}));
        }
        if widgets::button(ui, tr("Close"), false, !busy).clicked() {
            app.ui.connect.open = false;
        }
    });
    if !found {
        widgets::note(
            ui,
            tr(
                "Claude Code's claude command wasn't found on this computer. Copy the command and run it in a terminal where Claude Code is installed.",
            ),
            t.text_secondary,
        );
    }
    match &app.ui.connect.stage {
        Stage::Ready => {}
        Stage::Running(..) => widgets::note(ui, tr("Asking Claude Code…"), t.text_secondary),
        Stage::Asking(e) => {
            let e = e.clone();
            widgets::note(
                ui,
                &trf("Claude Code already has a scrim entry pointing to {target}. Replace it with this window's address?", &[("target", &e.target)]),
                t.text,
            );
            crate::right::row(ui, 32.0, |ui| {
                if widgets::button(ui, tr("Replace"), true, true).clicked() {
                    let mut args = strings(&["mcp", "remove", "scrim"]);
                    if let Some(s) = &e.scope {
                        args.extend(strings(&["--scope", s]));
                    }
                    run(app, args, Waiting::Remove);
                }
                if widgets::button(ui, tr("Keep it"), false, true).clicked() {
                    app.ui.connect.stage = Stage::Done(Ok(tr("Kept Claude Code's scrim entry as it was.").into()));
                }
            });
        }
        // (In the text colour: the success green is 2.9:1 on light, too faint for text.)
        Stage::Done(Ok(m)) => widgets::note(ui, m, t.text),
        Stage::Done(Err(e)) => widgets::note(ui, &trf("It didn't work: {error}", &[("error", e)]), t.danger),
    }
    if matches!(app.ui.connect.stage, Stage::Done(_)) && app.ui.connect.old.is_some() {
        widgets::note(ui, tr("Claude Code still has the old figcraft entry from before the rename."), t.text_secondary);
        crate::right::row(ui, 32.0, |ui| {
            if widgets::button(ui, tr("Remove the old entry"), false, true).clicked() {
                app.run("mcp.remove-old-entry", json!({}));
            }
        });
    }
}

/// The confirmations: Run, and removing the old entry. Only the person can answer them.
pub fn dialogs(app: &mut FigApp, ctx: &Context) {
    poll(app, ctx);
    let t = Tokens::get(ctx);
    if app.ui.connect.confirm_run {
        let command = status(app)["command"].as_str().unwrap_or_default().to_string();
        let (mut go, mut cancel, mut open) = (false, false, true);
        crate::dialogs::window(tr("Run this command?"), ctx).open(&mut open).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.label(RichText::new(tr("Scrim will run Claude Code's claude command to add Scrim for every project. It checks first for an entry already there, and asks before replacing one.")).font(regular(11.0)).color(t.text_secondary));
            ui.add_space(6.0);
            ui.label(RichText::new(&command).monospace().color(t.text));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                go = widgets::button(ui, tr("Run"), true, true).clicked();
                cancel = widgets::button(ui, tr("Cancel"), false, true).clicked();
            });
        });
        if go {
            app.ui.connect.confirm_run = false;
            run(app, strings(&["mcp", "get", "scrim"]), Waiting::GetScrim);
        } else if cancel || !open {
            app.ui.connect.confirm_run = false;
        }
    }
    if let Some(e) = app.ui.connect.confirm_reregister.clone() {
        let command = status(app)["command"].as_str().unwrap_or_default().to_string();
        let (mut go, mut cancel, mut open) = (false, false, true);
        crate::dialogs::window(tr("Re-register Scrim?"), ctx).open(&mut open).show(ctx, |ui| {
            ui.set_width(420.0);
            let text = trf(
                "Scrim will run Claude Code's claude command to replace its scrim entry, which points to {old}, with this window's address:",
                &[("old", &e.target)],
            );
            ui.label(RichText::new(text).font(regular(11.0)).color(t.text_secondary));
            ui.add_space(6.0);
            ui.label(RichText::new(&command).monospace().color(t.text));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                go = widgets::button(ui, tr("Re-register"), true, true).clicked();
                cancel = widgets::button(ui, tr("Cancel"), false, true).clicked();
            });
        });
        if go {
            app.ui.connect.confirm_reregister = None;
            app.ui.connect.open = true;
            let mut args = strings(&["mcp", "remove", "scrim"]);
            if let Some(s) = &e.scope {
                args.extend(strings(&["--scope", s]));
            }
            run(app, args, Waiting::Remove);
        } else if cancel || !open {
            app.ui.connect.confirm_reregister = None;
        }
    }
    if app.ui.connect.confirm_remove_old {
        let (mut go, mut cancel, mut open) = (false, false, true);
        crate::dialogs::window(tr("Remove the old entry?"), ctx).open(&mut open).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.label(
                RichText::new(tr("Scrim will run Claude Code's claude command to remove its figcraft MCP entry."))
                    .font(regular(11.0))
                    .color(t.text_secondary),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                go = widgets::button(ui, tr("Remove"), true, true).clicked();
                cancel = widgets::button(ui, tr("Cancel"), false, true).clicked();
            });
        });
        if go {
            app.ui.connect.confirm_remove_old = false;
            let mut args = strings(&["mcp", "remove", "figcraft"]);
            if let Some(s) = app.ui.connect.old.as_ref().and_then(|e| e.scope.clone()) {
                args.extend(strings(&["--scope", &s]));
            }
            run(app, args, Waiting::RemoveOld);
        } else if cancel || !open {
            app.ui.connect.confirm_remove_old = false;
        }
    }
}

#[cfg(test)]
#[path = "connect_flow_tests.rs"]
mod tests;
