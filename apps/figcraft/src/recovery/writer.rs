//! Recovery copies written on a real-time clock (Fix AI C-5). Each frame offers the newest copy of
//! every document that changed; one writer thread writes it once the interval has passed since
//! that document's last copy, in real time. It asks for no frame: a window that draws none (hidden,
//! covered, minimised) still gets its copies, and an idle window can sleep. Edits only happen in a
//! frame, so the newest offer is always the document as it is.
//!
//! What is due when is [`Plan::due`], a function of the time it's given, so tests run it on a
//! fake clock.

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// A copy to write: the revisions it is of ([`super::stamp`]), and the write (true: it succeeded).
pub(super) struct Job {
    pub stamp: Vec<u64>,
    pub run: Box<dyn FnOnce() -> bool + Send>,
}

/// What the writer has to do, shared with the frame loop.
pub(super) struct Plan {
    interval: Duration,
    /// The newest copy of each document waiting to be written.
    jobs: HashMap<u64, Job>,
    /// When each document was last copied.
    last: HashMap<u64, Instant>,
    /// Copies written since the frame loop last looked: document, stamp, success.
    results: Vec<(u64, Vec<u64>, bool)>,
    /// The document being written now.
    writing: Option<u64>,
    stop: bool,
}

impl Plan {
    pub(super) fn new(interval: Duration) -> Self {
        Self { interval, jobs: HashMap::new(), last: HashMap::new(), results: Vec::new(), writing: None, stop: false }
    }

    /// Offer `job` for document `uid`, in place of a copy of it still waiting.
    pub(super) fn offer(&mut self, uid: u64, job: Job) {
        self.jobs.insert(uid, job);
    }

    /// The copies due at `now`, each document at most once every interval, and when the next one
    /// is due (`None`: nothing waits).
    pub(super) fn due(&mut self, now: Instant) -> (Vec<(u64, Job)>, Option<Instant>) {
        let at = |last: &HashMap<u64, Instant>, uid: &u64| last.get(uid).map_or(now, |t| *t + self.interval);
        let ready: Vec<u64> = self.jobs.keys().filter(|uid| at(&self.last, uid) <= now).copied().collect();
        let due = ready.into_iter().filter_map(|uid| self.jobs.remove(&uid).map(|j| (uid, j))).collect();
        let next = self.jobs.keys().map(|uid| at(&self.last, uid)).min();
        (due, next)
    }

    /// Document `uid`'s copy of `stamp` was written at `at` (or failed).
    pub(super) fn wrote(&mut self, uid: u64, stamp: Vec<u64>, ok: bool, at: Instant) {
        self.last.insert(uid, at);
        self.results.push((uid, stamp, ok));
    }
}

/// The thread that writes the copies, and the plan it shares with the frame loop.
pub(super) struct Writer {
    shared: Arc<(Mutex<Plan>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl Writer {
    pub(super) fn start(interval: Duration) -> Self {
        let shared = Arc::new((Mutex::new(Plan::new(interval)), Condvar::new()));
        let s = shared.clone();
        let thread = std::thread::Builder::new().name("recovery".into()).spawn(move || run(&s)).ok();
        Self { shared, thread }
    }

    fn plan(&self) -> MutexGuard<'_, Plan> {
        self.shared.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Copy document `uid` as `job` once it is due.
    pub(super) fn offer(&self, uid: u64, job: Job) {
        self.plan().offer(uid, job);
        self.shared.1.notify_all();
    }

    /// The copies written since the last look: document, stamp, success.
    pub(super) fn results(&self) -> Vec<(u64, Vec<u64>, bool)> {
        std::mem::take(&mut self.plan().results)
    }

    /// Drop document `uid`'s waiting copy, and wait for a write of it in progress to end.
    pub(super) fn cancel(&self, uid: u64) {
        let mut p = self.plan();
        p.jobs.remove(&uid);
        while p.writing == Some(uid) {
            p = self.shared.1.wait(p).unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Whether document `uid` is being written now.
    #[cfg(test)]
    pub(super) fn writing(&self, uid: u64) -> bool {
        self.plan().writing == Some(uid)
    }

    /// Copies every `interval` from now on.
    #[cfg(test)]
    pub(super) fn set_interval(&self, interval: Duration) {
        self.plan().interval = interval;
        self.shared.1.notify_all();
    }

    /// Stop once the write in progress ends; copies still waiting aren't written.
    pub(super) fn stop(&mut self) {
        self.plan().stop = true;
        self.shared.1.notify_all();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The writer thread: write what is due, then sleep until the next copy is due or one is offered.
fn run(shared: &(Mutex<Plan>, Condvar)) {
    let (m, cv) = shared;
    let mut p = m.lock().unwrap_or_else(PoisonError::into_inner);
    loop {
        if p.stop {
            return;
        }
        let (due, next) = p.due(Instant::now());
        if due.is_empty() {
            p = match next {
                Some(at) => cv.wait_timeout(p, at.saturating_duration_since(Instant::now())).map_or_else(|e| e.into_inner().0, |(g, _)| g),
                None => cv.wait(p).unwrap_or_else(PoisonError::into_inner),
            };
            continue;
        }
        for (uid, job) in due {
            p.writing = Some(uid);
            drop(p);
            let ok = (job.run)();
            p = m.lock().unwrap_or_else(PoisonError::into_inner);
            p.writing = None;
            p.wrote(uid, job.stamp, ok, Instant::now());
            cv.notify_all();
        }
    }
}

#[cfg(test)]
#[path = "writer_tests.rs"]
mod tests;
