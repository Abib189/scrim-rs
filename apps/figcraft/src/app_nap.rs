//! Keep macOS from napping Scrim while agents work (R3a, `docs/design/working-state.md` › R3a).
//! - **Why:** App Nap slows a hidden app's timers, so a working state could end long after its
//!   grace period, and an agent's requests could wait on a napping app.
//! - **What:** an `NSProcessInfo` activity, "user initiated, allowing idle system sleep", is held
//!   from the moment agents start working until they stop. It prevents App Nap and timer
//!   coalescing; it doesn't keep the computer awake.
//! - **Let go** by releasing it: the activity ends when its token is released (`endActivity:` is
//!   unsafe to call, which the workspace forbids). Both are done inside an autorelease pool, so
//!   nothing else holds the token.
//! - Other platforms nap no apps this way: nothing to hold.

#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};

/// The activity's reason, as the system lists it.
pub const REASON: &str = "Agents are working in Scrim";

/// The activity held while agents work, if any.
#[derive(Default)]
pub struct Awake {
    #[cfg(target_os = "macos")]
    token: Option<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
}

impl Awake {
    /// Agents work (`on`) or have stopped: take the activity, or let it go.
    pub fn set(&mut self, on: bool) {
        #[cfg(target_os = "macos")]
        self.set_with(on, NSActivityOptions::UserInitiatedAllowingIdleSystemSleep, REASON);
        #[cfg(not(target_os = "macos"))]
        let _ = on;
    }

    /// [`Awake::set`] with the activity's options and reason (tests use one the system lists).
    #[cfg(target_os = "macos")]
    pub(crate) fn set_with(&mut self, on: bool, options: NSActivityOptions, reason: &str) {
        match (on, self.token.take()) {
            (true, None) => {
                let reason = NSString::from_str(reason);
                self.token = Some(objc2::rc::autoreleasepool(|_| NSProcessInfo::processInfo().beginActivityWithOptions_reason(options, &reason)));
            }
            (true, held) => self.token = held,
            (false, Some(t)) => objc2::rc::autoreleasepool(|_| drop(t)),
            (false, None) => {}
        }
    }

    /// Whether the activity is held.
    #[cfg(test)]
    pub fn held(&self) -> bool {
        #[cfg(target_os = "macos")]
        return self.token.is_some();
        #[cfg(not(target_os = "macos"))]
        false
    }
}

#[cfg(test)]
#[path = "app_nap_tests.rs"]
mod tests;
