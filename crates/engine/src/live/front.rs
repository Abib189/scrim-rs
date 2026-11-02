//! Which session's file comes to the front, and starting sessions (moved out of `live.rs`).

use super::{LiveLink, Lives, Role, SessionId, unavailable};
use crate::Result;

impl Lives {
    /// Start a session on the link for command `cmd`: `start` gets the link and the new
    /// session's id, which comes back with what it returned.
    pub(super) fn start<T>(&mut self, cmd: &str, start: impl FnOnce(&mut dyn LiveLink, SessionId) -> Result<T>) -> Result<(SessionId, T)> {
        let link = self.link.as_deref_mut().ok_or_else(|| unavailable(cmd))?;
        let id = self.last.saturating_add(1);
        let out = start(link, id)?;
        self.last = id;
        Ok((id, out))
    }

    /// Whether join `id` is the one asked for last (Fix AI C-7): its file comes to the front when
    /// it opens. Of two joins asked for one after the other, the network may finish either first,
    /// and the file you asked for last must stay in front. Session ids follow the order joins and
    /// hosting were started in; a session you host doesn't count.
    pub(super) fn asked_last(&self, id: SessionId) -> bool {
        !self.sessions.values().any(|l| l.id > id && l.info.as_ref().is_none_or(|i| i.role == Role::Guest))
    }
}
