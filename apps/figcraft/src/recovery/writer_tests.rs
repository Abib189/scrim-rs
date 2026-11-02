//! The recovery writer's schedule on a fake clock (Fix AI C-5): what is due when depends only on
//! the time it is given, never on frames.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::{Job, Plan};

const SECS: Duration = Duration::from_secs(10);

/// A job for `stamp` that records its stamp in `log` when it runs.
fn job(stamp: u64, log: &Arc<Mutex<Vec<u64>>>) -> Job {
    let log = log.clone();
    Job {
        stamp: vec![stamp],
        run: Box::new(move || {
            log.lock().unwrap().push(stamp);
            true
        }),
    }
}

/// Run what is due at `now` and record it as written then; when the next is due.
fn step(plan: &mut Plan, now: Instant) -> Option<Instant> {
    let (due, next) = plan.due(now);
    for (uid, j) in due {
        let ok = (j.run)();
        plan.wrote(uid, j.stamp, ok, now);
    }
    next.or_else(|| plan.due(now).1)
}

#[test]
fn a_first_copy_is_due_at_once_and_the_next_one_interval_later() {
    let (t0, log) = (Instant::now(), Arc::new(Mutex::new(Vec::new())));
    let mut plan = Plan::new(SECS);
    plan.offer(1, job(1, &log));
    assert_eq!(step(&mut plan, t0), None, "written at once, nothing waits");
    assert_eq!(*log.lock().unwrap(), [1]);
    // An edit a second later waits for the interval, in real time, with no frame in between.
    plan.offer(1, job(2, &log));
    assert_eq!(step(&mut plan, t0 + Duration::from_secs(1)), Some(t0 + SECS));
    assert_eq!(step(&mut plan, t0 + SECS - Duration::from_millis(1)), Some(t0 + SECS), "not yet");
    assert_eq!(step(&mut plan, t0 + SECS), None);
    assert_eq!(*log.lock().unwrap(), [1, 2]);
}

#[test]
fn the_newest_offer_replaces_the_waiting_one() {
    let (t0, log) = (Instant::now(), Arc::new(Mutex::new(Vec::new())));
    let mut plan = Plan::new(SECS);
    plan.offer(1, job(1, &log));
    step(&mut plan, t0);
    for stamp in 2..=5 {
        plan.offer(1, job(stamp, &log));
        step(&mut plan, t0 + Duration::from_secs(stamp));
    }
    step(&mut plan, t0 + SECS);
    assert_eq!(*log.lock().unwrap(), [1, 5], "one copy, of the newest revision");
}

#[test]
fn documents_are_paced_on_their_own() {
    let (t0, log) = (Instant::now(), Arc::new(Mutex::new(Vec::new())));
    let mut plan = Plan::new(SECS);
    plan.offer(1, job(11, &log));
    step(&mut plan, t0);
    // Another document's first copy doesn't wait for the first one's interval.
    plan.offer(2, job(21, &log));
    step(&mut plan, t0 + Duration::from_secs(3));
    assert_eq!(*log.lock().unwrap(), [11, 21]);
    plan.offer(1, job(12, &log));
    plan.offer(2, job(22, &log));
    assert_eq!(step(&mut plan, t0 + Duration::from_secs(4)), Some(t0 + SECS), "the earlier of the two");
    step(&mut plan, t0 + SECS);
    assert_eq!(*log.lock().unwrap(), [11, 21, 12]);
    step(&mut plan, t0 + Duration::from_secs(13));
    assert_eq!(*log.lock().unwrap(), [11, 21, 12, 22]);
}
