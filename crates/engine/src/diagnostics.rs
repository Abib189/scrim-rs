//! Settings ▸ Diagnostics (`docs/design/settings-and-connect.md`, C3d): a short list of what
//! happened, kept on this computer only (in the preferences) for as long as the person chooses,
//! so a problem can be described. Nothing is ever sent anywhere.
//!
//! **Only allow-listed facts go in.** An event is made from an [`Entry`], never from free text:
//! - a file reverted to its saved copy (no name or path);
//! - the fonts a file falls back from (family names only);
//! - the MCP server starting (its port) or failing to;
//! - an MCP client connecting (its name and version as it gave them), the same one again within a
//!   minute counted on its line (Fix V4);
//! - a command failing (its id, when it is a registered command, and the error's category; never
//!   the message, which can quote the document);
//! - an agent refused when it asks to change Follow agents: the route and its session (Fix V2);
//! - every change of Follow agents, from and to, and who made it ([`FollowOrigin`]); the app
//!   starting with it off (Fix V3);
//! - following an agent pausing, and why ([`Pause`]), and the pause clearing ([`Cleared`], Fix N), and the reveal drawing an agent's new layers
//!   final at once: how many entered and how many didn't, by reason ([`Skip`]) (Fix L2);
//! - the working state's transitions: a client beginning, ending or being held, with the cause
//!   ([`Change`], [`Cause`]) and the client as it named itself, with its session id cut to 8
//!   characters (Fix M0).
//!
//! So no prompts, document content, paths or tokens can reach it; `diagnostics_tests.rs` scans for
//! them.

use serde::{Deserialize, Serialize};

use crate::EngineError;
use crate::settings::Settings;

/// The most events kept, whatever the retention.
pub const MAX_EVENTS: usize = 1000;
/// The retention choices, in days (0: keep nothing).
pub const RETENTION_DAYS: [u32; 4] = [0, 1, 7, 30];
/// The retention unless the person picks another.
pub const DEFAULT_RETENTION_DAYS: u32 = 7;
/// The most font families one event names.
const MAX_FAMILIES: usize = 30;

/// One thing that happened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Seconds since 1970 (UTC).
    pub at: u64,
    /// `reload`, `fontFallback`, `mcpServer`, `mcpClient`, `error`, `follow`, `reveal`, `working`,
    /// `chat` or `setting`.
    pub kind: String,
    pub detail: String,
}

/// What can be recorded: each variant carries only what its event may say.
pub enum Entry<'a> {
    /// A file was reverted to its saved copy (File ▸ Revert).
    Reload,
    /// A file opened with fonts it doesn't have: their family names.
    FontFallback(&'a [String]),
    /// The MCP server started, on this port.
    McpStarted(u16),
    /// The MCP server couldn't start.
    McpFailed,
    /// An MCP client initialized.
    McpClient { name: &'a str, version: &'a str },
    /// A command failed: its id and the error.
    Error { command: &'a str, error: &'a EngineError },
    /// The Follow agents setting changed, from `from` to `to`, made by `origin` (Fix V3).
    FollowSetting { from: bool, to: bool, origin: FollowOrigin },
    /// The app started with Follow agents off (Fix V3).
    FollowOffAtStart,
    /// A saved setting changed (Fix W4): its name, the change as said ("system → dark", or
    /// "changed"), and who made it. Never an agent's (refused).
    SettingChanged { name: &'a str, change: &'a str, origin: super::changes::Origin },
    /// An agent asked for a command that writes a saved setting and was refused (Fix W3): the
    /// command (a registered id), the route and the session. Nothing the agent sent.
    SettingRefused { command: &'a str, route: &'a str, session: Option<&'a str> },
    /// An agent asked to turn Follow agents on or off and was refused (Fix V2): the setting (`on`,
    /// unchanged), the route it came by and its session. Nothing the agent sent.
    FollowRefused { on: bool, route: &'a str, session: Option<&'a str> },
    /// Following the agent paused, for this reason.
    FollowPaused(Pause),
    /// The pause was cleared, for this reason (Fix N).
    FollowCleared(Cleared),
    /// Agents' new layers since the last such event: how many entered, and how many were drawn
    /// final at once, by reason.
    Reveal { entered: usize, skipped: &'a [(Skip, usize)] },
    /// Agents' new layers waited for following's flight to arrive, and entered at the cap
    /// (`ms`) while it was still on its way (Fix T3).
    RevealWaited { layers: usize, ms: u64 },
    /// The working state changed for a client (`client`: its name as announced; `session`: its
    /// session id, of which 8 characters are kept). `run`: whether the dots are shown after it.
    Working { change: Change, cause: Cause, client: Option<&'a str>, session: Option<&'a str>, run: bool },
    /// A client's working state ended at the grace period's end, and it came back `after` seconds
    /// after its request before: the setting (`setting` seconds) is shorter than its pauses (Fix Q2).
    GraceTooShort { client: Option<&'a str>, after: u64, setting: u32 },
    /// An agent run from "Describe a change" (R5): the agent's name, and how it ended once it has
    /// (`finished`, `stopped`, `timed out`, `failed`), its exit code and how long it took. Never the
    /// prompt or what the agent said.
    ChatRun { agent: &'a str, ended: Option<(&'a str, Option<i32>, u64)> },
}

/// A working-state transition (Fix M0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// A client began working.
    Begin,
    /// A client stopped, and with it the run of work: the dots fade out.
    End,
    /// A client stopped, and the run carries on (another client works, or it is held for the same
    /// client to come back).
    Hold,
}

/// Why it changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    FirstRequest,
    FinishWorking,
    SessionClosed,
    GraceExpired,
    Disconnected,
    AnotherClient,
}

impl Cause {
    pub fn text(self) -> &'static str {
        match self {
            Cause::FirstRequest => "first request",
            Cause::FinishWorking => "finish_working",
            Cause::SessionClosed => "session closed",
            Cause::GraceExpired => "grace period expired",
            Cause::Disconnected => "disconnected",
            Cause::AnotherClient => "another client is working",
        }
    }
}

/// Why following the agent paused (Stage I1's rules).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pause {
    PersonEdit,
    /// And where the move behind it was made (Fix S4).
    CameraMoved(Mover),
    CanvasInput,
}

impl Pause {
    pub fn text(self) -> String {
        match self {
            Pause::PersonEdit => "the person's edit".into(),
            Pause::CameraMoved(by) => format!("a camera move that wasn't follow's or an agent's (made by {})", by.text()),
            Pause::CanvasInput => "the person's press, scroll or pinch on the canvas".into(),
        }
    }
}

/// Where a change of the camera was made (Fix S4): the window notes each one as it happens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mover {
    /// An agent's request (MCP or the control channel).
    Agent,
    /// A camera flight an agent's request started.
    AgentFlight,
    /// Following's own move, or its flight.
    Follow,
    /// A camera flight the person started (a Layers click, a zoom command).
    PersonFlight,
    /// The person's input on the canvas: a pan, a scroll, a pinch, a tool.
    Canvas,
    /// The canvas's size changed, and with it the first fit of a file.
    Sizing,
    /// Anywhere else in the window, outside the canvas: a shortcut, a menu, a panel, a tab, a
    /// collaborator's view.
    Window,
}

impl Mover {
    pub fn text(self) -> &'static str {
        match self {
            Mover::Agent => "an agent's request",
            Mover::AgentFlight => "an agent's camera flight",
            Mover::Follow => "following's own move",
            Mover::PersonFlight => "a camera flight the person started",
            Mover::Canvas => "the person's input on the canvas",
            Mover::Sizing => "the canvas's sizing",
            Mover::Window => "the window outside the canvas: a shortcut, menu, panel, tab or collaborator",
        }
    }
}

/// Why a pause of following was cleared (Fix N): the work it paused ended, or the person resumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cleared {
    AgentFinished,
    GraceExpired,
    Disconnected,
    Resumed,
}

impl Cleared {
    pub fn text(self) -> &'static str {
        match self {
            Cleared::AgentFinished => "the agent finished",
            Cleared::GraceExpired => "the grace period expired",
            Cleared::Disconnected => "the agent disconnected",
            Cleared::Resumed => "resumed",
        }
    }
}

/// Why an agent's new layer was drawn final at once instead of entering (Stage I2's rules).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Skip {
    RevealOff,
    NoMotion,
    Hidden,
    Undo,
    OffScreenPaused,
    OffScreenOff,
    CutShort,
}

impl Skip {
    pub fn text(self) -> &'static str {
        match self {
            Skip::RevealOff => "Reveal agent work off",
            Skip::NoMotion => "Animations Off or reduced motion",
            Skip::Hidden => "landed while the window was hidden",
            Skip::Undo => "undo or redo",
            Skip::OffScreenPaused => "off screen, follow paused",
            Skip::OffScreenOff => "off screen, follow off",
            Skip::CutShort => "entrance cut short (dropped frames, a hidden window or a window capture)",
        }
    }
}

/// Who changed the Follow agents setting (Fix V3): the person, by the control they used, or a
/// settings migration. An agent never does (it is refused, [`Entry::FollowRefused`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FollowOrigin {
    /// The AI panel header's Follow agents button.
    Button,
    /// Settings ▸ Agents' toggle.
    SettingsToggle,
    /// The resume on the canvas while following is paused (Fix N).
    Resume,
    /// The command another way: the Actions palette, a menu or a shortcut.
    Command,
    /// A settings migration as the app starts.
    Migration,
    /// Version 9's repair (Fix W1): following saved off that the person never chose.
    NeverChosen,
}

impl FollowOrigin {
    /// `camera.follow-agents`' `origin` parameter (the window's own controls name themselves); any
    /// other value, or none, is the command.
    pub fn from_param(origin: Option<&str>) -> Self {
        match origin {
            Some("button") => FollowOrigin::Button,
            Some("settings") => FollowOrigin::SettingsToggle,
            Some("resume") => FollowOrigin::Resume,
            _ => FollowOrigin::Command,
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            FollowOrigin::Button => "the person's button",
            FollowOrigin::SettingsToggle => "the person's Settings toggle",
            FollowOrigin::Resume => "the person's resume",
            FollowOrigin::Command => "the person's command",
            FollowOrigin::Migration => "a settings migration",
            FollowOrigin::NeverChosen => "a settings migration: saved off was never chosen",
        }
    }
}

/// "ON" or "OFF", as the follow setting's lines say it.
pub fn on_off(on: bool) -> &'static str {
    if on { "ON" } else { "OFF" }
}

/// How an agent's request came, in fixed words (never a name a plugin or client gave), with its
/// session's first 8 letters and digits when the route knows it.
fn by(route: &str, session: Option<&str>) -> String {
    let route = match route {
        r if r.starts_with("mcp") => "over MCP",
        "cli" => "from the CLI",
        "control channel" => "over the control channel",
        r if r.starts_with("plugin") => "from a plugin",
        _ => "by another route",
    };
    let session = session.map(|s| s.chars().filter(char::is_ascii_alphanumeric).take(8).collect::<String>()).filter(|s| !s.is_empty());
    session.map_or_else(|| route.to_string(), |s| format!("{route}, session {s}…"))
}

/// Seconds since 1970 (UTC), now.
pub fn now() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// `s` without control characters, at most `max` characters.
fn clean(s: &str, max: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(max).collect()
}

/// The kind of error, never its message.
pub fn category(e: &EngineError) -> &'static str {
    match e {
        EngineError::UnknownCommand(_) => "unknown command",
        EngineError::Disabled(..) => "not available now",
        EngineError::BadParams { .. } => "invalid parameters",
        EngineError::NoDocument => "no open document",
        EngineError::NoNode(_) => "no such layer",
        EngineError::Other(_) => "failed",
        EngineError::Waiting => "waiting",
        EngineError::Internal { .. } => "internal error",
    }
}

impl Entry<'_> {
    fn kind(&self) -> &'static str {
        match self {
            Entry::Reload => "reload",
            Entry::FontFallback(_) => "fontFallback",
            Entry::McpStarted(_) | Entry::McpFailed => "mcpServer",
            Entry::McpClient { .. } => "mcpClient",
            Entry::Error { .. } => "error",
            Entry::FollowPaused(_)
            | Entry::FollowCleared(_)
            | Entry::FollowRefused { .. }
            | Entry::FollowSetting { .. }
            | Entry::FollowOffAtStart => "follow",
            Entry::SettingRefused { .. } | Entry::SettingChanged { .. } => "setting",
            Entry::Reveal { .. } | Entry::RevealWaited { .. } => "reveal",
            Entry::Working { .. } | Entry::GraceTooShort { .. } => "working",
            Entry::ChatRun { .. } => "chat",
        }
    }

    fn detail(&self) -> String {
        match self {
            Entry::Reload => "a file was reverted to its saved copy".into(),
            Entry::FontFallback(families) => {
                let mut names: Vec<String> = families.iter().take(MAX_FAMILIES).map(|f| clean(f, 64)).collect();
                if families.len() > MAX_FAMILIES {
                    names.push(format!("and {} more", families.len() - MAX_FAMILIES));
                }
                format!("fonts not on this computer, a fallback used: {}", names.join(", "))
            }
            Entry::McpStarted(port) => format!("started on 127.0.0.1:{port}"),
            Entry::McpFailed => "couldn't start".into(),
            Entry::McpClient { name, version } => {
                // As announced (cleaned); a client that announced no name is "an agent", never a guess.
                let name = Some(clean(name, 80)).filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "an agent".into());
                format!("{name} {} connected", clean(version, 40)).replace("  ", " ")
            }
            Entry::Error { command, error } => {
                // Only a registered command's id: anything else could be text someone typed.
                let id = if crate::find_command(command).is_some() { *command } else { "a command" };
                format!("{id}: {}", category(error))
            }
            Entry::FollowPaused(why) => format!("following paused: {}", why.text()),
            Entry::FollowSetting { from, to, origin } => {
                format!("follow setting: {} → {} (made by {})", on_off(*from), on_off(*to), origin.text())
            }
            Entry::FollowOffAtStart => "following is OFF".into(),
            Entry::SettingChanged { name, change, origin } => {
                format!("setting {}: {} (made by {})", clean(name, 60), clean(change, 80), origin.text())
            }
            Entry::SettingRefused { command, route, session } => {
                // Only a registered command's id: anything else could be text an agent sent.
                let id = if crate::find_command(command).is_some() { *command } else { "a command" };
                format!("setting {id}: unchanged (refused: an agent asked, {})", by(route, *session))
            }
            Entry::FollowRefused { on, route, session } => {
                format!("follow setting: stays {} (refused: an agent asked, {})", on_off(*on), by(route, *session))
            }
            Entry::FollowCleared(why) => format!("follow pause cleared: {}", why.text()),
            Entry::Reveal { entered, skipped } => {
                let n: usize = skipped.iter().map(|s| s.1).sum();
                let why: Vec<String> = skipped.iter().map(|(s, k)| format!("{}: {k}", s.text())).collect();
                format!("agent layers: {entered} entered, {n} drawn final at once ({})", why.join("; "))
            }
            Entry::RevealWaited { layers, ms } => {
                format!("agent layers: {layers} waited {ms} ms for the camera, then entered while it was still on its way")
            }
            Entry::ChatRun { agent, ended } => {
                // A headless profile's name; anything else reads "an agent".
                let agent = crate::cmd::agent_clients::headless::profiles().iter().find(|p| p.name == *agent).map_or("an agent", |p| p.name.as_str());
                match ended {
                    None => format!("{agent} run started"),
                    Some((how, code, secs)) => {
                        let how = if matches!(*how, "finished" | "stopped" | "timed out" | "failed") { *how } else { "ended" };
                        let code = code.map_or_else(|| "no exit code".to_string(), |c| format!("exit {c}"));
                        format!("{agent} run ended: {how}, {code}, {secs} s")
                    }
                }
            }
            Entry::GraceTooShort { client, after, setting } => {
                let who = client.map(|c| clean(c, 80)).filter(|c| !c.trim().is_empty()).unwrap_or_else(|| "an agent".into());
                format!("grace too short: {who} came back after {after} s (the setting is {setting} s)")
            }
            Entry::Working { change, cause, client, session, run } => {
                let change = match change {
                    Change::Begin => "begin",
                    Change::End => "end",
                    Change::Hold => "hold",
                };
                let who = client.map(|c| clean(c, 80)).filter(|c| !c.trim().is_empty()).unwrap_or_else(|| "an agent".into());
                // Only the id's first 8 letters and digits: enough to tell sessions apart.
                let session = session.map(|s| s.chars().filter(char::is_ascii_alphanumeric).take(8).collect::<String>());
                let session = session.filter(|s| !s.is_empty()).map_or_else(|| "no session".to_string(), |s| format!("session {s}…"));
                format!("{change}: {who} ({session}), {}; the dots {}", cause.text(), if *run { "are on" } else { "fade out" })
            }
        }
    }
}

/// The same client connecting again within this many seconds of the line's first is counted on
/// that line ("… connected ×3"), not given another (Fix V4).
pub const CONNECT_WINDOW_SECS: u64 = 60;
/// The most "connected" lines kept, the newest: they never push out what matters (Fix V4).
pub const MAX_CONNECT_EVENTS: usize = 200;

/// Note `entry` at `now` (seconds since 1970), then drop what the retention no longer keeps.
pub fn record(settings: &mut Settings, now: u64, entry: Entry) {
    if matches!(entry, Entry::Error { error: EngineError::Waiting, .. }) {
        return;
    }
    let (kind, detail) = (entry.kind(), entry.detail());
    // A client connecting again and again (Fix V4): one line, counted, while nothing else happened.
    match settings.diagnostics.last_mut() {
        Some(last)
            if kind == "mcpClient"
                && last.kind == kind
                && now.saturating_sub(last.at) <= CONNECT_WINDOW_SECS
                && counted(&last.detail).0 == detail =>
        {
            last.detail = format!("{detail} ×{}", counted(&last.detail).1 + 1);
        }
        _ => settings.diagnostics.push(Event { at: now, kind: kind.into(), detail }),
    }
    prune(settings, now);
}

/// A "connected" line without its count, and the count (1 without one).
fn counted(detail: &str) -> (&str, u32) {
    detail
        .rsplit_once(" ×")
        .and_then(|(line, n)| Some((line, n.parse().ok()?)))
        .filter(|(line, _)| line.ends_with(" connected"))
        .unwrap_or((detail, 1))
}

/// Which events go first when there are too many: clients connecting and the server starting, then
/// the rest, and an agent's work (working, following, the reveal) last (Fix V4).
fn rank(kind: &str) -> u8 {
    match kind {
        "mcpClient" | "mcpServer" => 0,
        "working" | "follow" | "reveal" => 2,
        _ => 1,
    }
}

/// Drop the oldest `n` events `pick` chooses.
fn drop_oldest(events: &mut Vec<Event>, mut n: usize, pick: impl Fn(&Event) -> bool) {
    events.retain(|e| {
        let gone = n > 0 && pick(e);
        n -= usize::from(gone);
        !gone
    });
}

/// Drop events older than the retention allows; then the oldest "connected" lines past
/// [`MAX_CONNECT_EVENTS`]; then, past [`MAX_EVENTS`], the oldest of the least important kind first
/// ([`rank`]), so connection lines never push out an agent's work.
pub fn prune(settings: &mut Settings, now: u64) {
    let keep = u64::from(settings.diagnostics_days) * 86_400;
    settings.diagnostics.retain(|e| settings.diagnostics_days > 0 && now.saturating_sub(e.at) <= keep);
    let connects = settings.diagnostics.iter().filter(|e| e.kind == "mcpClient").count();
    drop_oldest(&mut settings.diagnostics, connects.saturating_sub(MAX_CONNECT_EVENTS), |e| e.kind == "mcpClient");
    for r in 0..=2 {
        let excess = settings.diagnostics.len().saturating_sub(MAX_EVENTS);
        drop_oldest(&mut settings.diagnostics, excess, |e| rank(&e.kind) == r);
    }
}

/// `secs` since 1970 as `2026-10-20 14:03:09 UTC`.
pub fn timestamp(secs: u64) -> String {
    // Days to a civil date (Howard Hinnant's civil_from_days).
    let z = i64::try_from(secs / 86_400).unwrap_or(0) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let s = secs % 86_400;
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC", s / 3600, s / 60 % 60, s % 60)
}

/// The events as text, one a line, oldest first (Copy and Save).
pub fn text(events: &[Event]) -> String {
    events.iter().map(|e| format!("{}  {}  {}\n", timestamp(e.at), e.kind, e.detail)).collect()
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod tests;
