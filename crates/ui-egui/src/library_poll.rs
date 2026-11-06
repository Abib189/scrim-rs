//! When the Assets panel asks team libraries' servers for a newer version (Fix R2-2). A team library
//! is a library linked from a server; asking costs the server a request and tells it which of the
//! library's components the document uses, so Scrim asks rarely and only while the person can see
//! the answer:
//! - once when the panel comes into view (at most every [`FOCUS_CHECK`]: a tab flicked back and
//!   forth, or frames of another window between the panel's, ask nothing more), or a library is
//!   linked;
//! - then at most every [`LIBRARY_POLL`] while the panel shows and the window is in front;
//! - at once when the window comes back to the front, at most every [`FOCUS_CHECK`];
//! - never while the panel is hidden or the window isn't in front.
//!
//! Libraries ▸ Check team libraries for updates (`libraries.checkUpdates`) asks at any time.
//!
//! The schedule runs on the real-time clock and asks for no frame of its own: it is consulted on
//! the frames the panel draws anyway (it looks for changed library files every 2 seconds while it
//! shows). A window that draws nothing asks nothing.

use std::time::{Duration, Instant};

/// At most this often while the panel shows and the window is in front.
pub const LIBRARY_POLL: Duration = Duration::from_secs(5 * 60);
/// A window coming back to the front asks at once, at most this often.
pub const FOCUS_CHECK: Duration = Duration::from_secs(30);

/// The panel's schedule: what it saw on the last frame it drew.
#[derive(Clone, Debug, Default)]
pub struct Poll {
    /// When the servers were last asked.
    last_check: Option<Instant>,
    /// When coming back to the front last asked.
    last_focus_check: Option<Instant>,
    /// The window was in front on the last frame.
    focused: bool,
    /// The last frame the panel drew (a gap: it was hidden in between).
    drawn: Option<u64>,
    /// The team libraries linked on the last frame.
    links: Vec<String>,
}

impl Poll {
    /// One frame of the panel, `frame`, at `now`, with the window in front or not and the
    /// document's team libraries `links`: whether to ask their servers now.
    pub fn frame(&mut self, now: Instant, frame: u64, focused: bool, links: &[String]) -> bool {
        let came_into_view = self.drawn.is_none_or(|d| d.saturating_add(1) < frame);
        let came_to_front = focused && !self.focused;
        let linked = links.iter().any(|l| !self.links.contains(l));
        self.drawn = Some(frame);
        self.focused = focused;
        self.links = links.to_vec();
        if links.is_empty() || !focused {
            return false;
        }
        let front = came_to_front && self.last_focus_check.is_none_or(|t| now >= t + FOCUS_CHECK);
        let due = self.last_check.is_none_or(|t| now >= t + LIBRARY_POLL);
        let shown = came_into_view && self.last_check.is_none_or(|t| now >= t + FOCUS_CHECK);
        let ask = shown || linked || front || due;
        if ask {
            self.last_check = Some(now);
            if came_to_front {
                self.last_focus_check = Some(now);
            }
        }
        ask
    }
}

#[cfg(test)]
#[path = "library_poll_tests.rs"]
mod tests;
