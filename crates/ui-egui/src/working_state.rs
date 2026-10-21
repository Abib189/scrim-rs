//! One working state (Stage I.1, `docs/design/working-state.md`): whether agents are working on the
//! canvas, per client, and the single source the dots, the outline, follow's status, the AI tab,
//! the AccessKit status and the load budget all read.
//! - **A client** is keyed by its MCP session (an HTTP `Mcp-Session-Id`, or an `--attach`
//!   connection), else its name, else its route ([`key`]).
//! - **It is working** from any request of its, reads included, until the grace period
//!   (Settings ▸ General) passes without another, it calls `finish_working`, or its session ends.
//! - **Several clients:** the state is on while any of them works, named after the most recently
//!   active one.
//! - **One run:** the state begins when the first client starts and ends when the last one stops,
//!   so gaps between an agent's steps never end it. It ends at the grace period's end exactly (or
//!   at the finish), which the fade out counts from.
//! - Window state only: never in the document, undo, the file, MCP results or exports.

use std::collections::BTreeMap;

/// The fade in when a run of work starts, and the fade out when it ends.
pub const FADE_IN: f64 = 0.16;
pub const FADE_OUT: f64 = 0.4;
/// How long "{name} finished" stays for screen readers after a run ends.
pub const FINISHED_FOR: f64 = 2.0;
/// Clients that stopped longer ago than this are forgotten.
const FORGET: f64 = 120.0;

/// A client's key: its session, else its MCP client name, else the route.
pub fn key(session: Option<&str>, client: Option<&str>, route: &str) -> String {
    match (session, client) {
        (Some(s), _) => format!("session:{s}"),
        (None, Some(c)) => format!("client:{c}"),
        (None, None) => format!("route:{route}"),
    }
}

#[derive(Clone, Debug)]
struct Client {
    /// The name a person sees (J1's rules); `None`: "an agent".
    name: Option<String>,
    /// Its last request.
    last: f64,
    /// When it said it finished (or its session ended).
    finished: Option<f64>,
}

impl Client {
    /// When its work ends (or ended): its finish, else the grace period after its last request.
    fn end(&self, grace: f64) -> f64 {
        self.finished.unwrap_or(self.last + grace)
    }
    fn working(&self, now: f64, grace: f64) -> bool {
        self.finished.is_none() && now < self.last + grace
    }
}

/// The working state.
#[derive(Default, Debug)]
pub struct State {
    clients: BTreeMap<String, Client>,
    /// The run of work showing now: when it began (counted back so a run starting during the last
    /// one's fade out carries on from where that fade was).
    began: Option<f64>,
    /// When the last run ended, and the name of the client that stopped last.
    ended: Option<(f64, Option<String>)>,
}

impl State {
    /// Client `key` (named `name`, if it says) made a request at `now`.
    pub fn touch(&mut self, key: String, name: Option<String>, now: f64) {
        let c = self.clients.entry(key).or_insert(Client { name: None, last: now, finished: None });
        if name.is_some() {
            c.name = name;
        }
        c.last = c.last.max(now);
        c.finished = None;
    }

    /// Client `key` finished (`finish_working`, or its session ended) at `now`.
    pub fn finish(&mut self, key: &str, now: f64) {
        if let Some(c) = self.clients.get_mut(key) {
            // Never before its own last request (a finish can be read a frame's time early).
            let at = now.max(c.last);
            c.finished = Some(c.finished.map_or(at, |f| f.min(at)));
        }
    }

    /// Bring the run up to `now` (`grace`: the setting, in seconds). Returns whether agents work.
    pub fn tick(&mut self, now: f64, grace: f64, motion: bool) -> bool {
        let working = self.clients.values().any(|c| c.working(now, grace));
        match (working, self.began) {
            (true, None) => {
                // Starting during the last run's fade out: carry on from its current opacity.
                let shown = self.ended.as_ref().map_or(0.0, |(e, _)| fade_out(now - e, motion));
                self.began = Some(now - FADE_IN * shown);
                self.ended = None;
            }
            (false, Some(_)) => {
                // It ends when the last client stopped: its finish, or its grace period's end.
                let last = self.clients.values().max_by(|a, b| a.end(grace).total_cmp(&b.end(grace)));
                let at = last.map_or(now, |c| c.end(grace).min(now));
                self.ended = Some((at, last.and_then(|c| c.name.clone())));
                self.began = None;
            }
            _ => {}
        }
        self.clients.retain(|_, c| now - c.end(grace) < FORGET);
        working
    }

    /// Agents are working (as of the last [`State::tick`]).
    pub fn working(&self) -> bool {
        self.began.is_some()
    }

    /// The most recently active working client's name (when working), else the one that stopped
    /// last. `None`: no name was announced ("an agent").
    pub fn name(&self, now: f64, grace: f64) -> Option<String> {
        let working = self.clients.values().filter(|c| c.working(now, grace)).max_by(|a, b| a.last.total_cmp(&b.last));
        match working {
            Some(c) => c.name.clone(),
            None => self.ended.as_ref().and_then(|(_, n)| n.clone()),
        }
    }

    /// How visible the indicator is at `now` (0..1): fading in when a run starts, out when it ends.
    /// Without motion: shown for the whole run, then gone.
    pub fn alpha(&self, now: f64, motion: bool) -> f64 {
        match (self.began, &self.ended) {
            (Some(b), _) => {
                if motion {
                    ((now - b) / FADE_IN).clamp(0.0, 1.0)
                } else {
                    1.0
                }
            }
            (None, Some((e, _))) => fade_out(now - e, motion),
            (None, None) => 0.0,
        }
    }

    /// The name to say "{name} finished" for, while that is recent.
    pub fn finished(&self, now: f64) -> Option<Option<String>> {
        match (&self.began, &self.ended) {
            (None, Some((e, name))) if now - e < FINISHED_FOR => Some(name.clone()),
            _ => None,
        }
    }

    /// Seconds until the state can next change on its own (a grace period running out), if it can.
    pub fn next_change(&self, now: f64, grace: f64) -> Option<f64> {
        self.clients.values().filter(|c| c.working(now, grace)).map(|c| c.last + grace - now).min_by(f64::total_cmp)
    }

    /// The clients working now (for `ui.inspect`).
    pub fn count(&self, now: f64, grace: f64) -> usize {
        self.clients.values().filter(|c| c.working(now, grace)).count()
    }
}

fn fade_out(since: f64, motion: bool) -> f64 {
    if !motion {
        return 0.0;
    }
    1.0 - (since / FADE_OUT).clamp(0.0, 1.0)
}

#[cfg(test)]
#[path = "working_state_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "working_state_window_tests.rs"]
mod window_tests;
