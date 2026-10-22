//! Running an agent CLI for the AI tab's composer (R5, `docs/design/chat-composer.md` › How a run
//! goes). Process plumbing only; what the output says is read by the engine
//! (`cmd::agent_clients::headless::output`).
//! - **Started from an argument list,** never a shell, from a binary path resolved beforehand.
//! - **Its environment is only what it's given** (`env_clear`), and its working folder a new empty
//!   one, removed when the run ends. Its standard input is closed.
//! - **Its own process group** (Unix), so stopping it stops everything it started.
//! - **Stdout** arrives line by line ([`RunEvent::Line`]); the last lines of stderr are kept for the
//!   end ([`RunEvent::Ended`]).
//! - **Stopped** by [`Run::stop`], by its time limit, or by dropping the [`Run`] (Scrim closing): the
//!   group gets `SIGTERM`, then `SIGKILL` after [`GRACE`] (`kill`, as an argument list); on Windows
//!   `taskkill /T /F`.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long a stopped run has to end after `SIGTERM` before `SIGKILL`.
pub const GRACE: Duration = Duration::from_secs(2);
/// How many lines of stderr are kept.
const STDERR_LINES: usize = 20;
/// The longest stdout line read (a longer one is cut).
const MAX_LINE: usize = 4 << 20;

/// What to run.
#[derive(Clone, Debug)]
pub struct RunSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    /// The whole environment the child gets.
    pub env: Vec<(String, String)>,
    /// Stopped after this long.
    pub limit: Duration,
}

/// Why a run ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum End {
    /// It exited by itself: its exit code (`None`: a signal).
    Exited(Option<i32>),
    /// [`Run::stop`] (or the run was dropped).
    Stopped,
    /// Its time limit.
    TimedOut,
}

/// What the run reports.
#[derive(Clone, Debug, PartialEq)]
pub enum RunEvent {
    /// A line of its standard output.
    Line(String),
    /// It ended, how, and the last lines of its standard error.
    Ended { end: End, stderr: Vec<String>, secs: f64 },
}

/// A run in progress.
pub struct Run {
    pub events: Receiver<RunEvent>,
    stop: Arc<AtomicBool>,
    pid: u32,
    /// Done: the waiter has reported the end.
    done: Arc<AtomicBool>,
}

/// A run that couldn't start, in plain words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartError(pub String);

impl Run {
    /// Start `spec`. The working folder is a new empty one, removed when the run ends.
    pub fn start(spec: RunSpec) -> Result<Run, StartError> {
        let dir = work_dir().map_err(|e| StartError(format!("couldn't make a folder for the run: {e}")))?;
        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args).env_clear().envs(spec.env.iter().map(|(k, v)| (k, v))).current_dir(&dir);
        cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&dir);
                return Err(StartError(format!("{}: {e}", spec.program.display())));
            }
        };
        let (tx, rx) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicBool::new(false));
        let tail = Arc::new(Mutex::new(VecDeque::new()));
        let pid = child.id();
        let readers = [
            child.stdout.take().and_then(|out| spawn_named("scrim-agent-stdout", read_lines(out, tx.clone()))),
            child.stderr.take().and_then(|err| spawn_named("scrim-agent-stderr", keep_tail(err, tail.clone()))),
        ];
        let waiter = Waiter { child, pid, dir, limit: spec.limit, stop: stop.clone(), done: done.clone(), tail, tx, readers };
        if std::thread::Builder::new().name("scrim-agent-wait".into()).spawn(move || waiter.wait()).is_err() {
            kill_tree(pid);
            return Err(StartError("couldn't watch the run".into()));
        }
        Ok(Run { events: rx, stop, pid, done })
    }

    /// Stop it: the whole process group.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    /// Its process id (its group's, on Unix).
    pub fn pid(&self) -> u32 {
        self.pid
    }
}

impl Drop for Run {
    /// Scrim closing, or the run forgotten: nothing it started outlives it.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if !self.done.load(Ordering::SeqCst) {
            kill_tree(self.pid);
        }
    }
}

fn spawn_named(name: &str, f: impl FnOnce() + Send + 'static) -> Option<std::thread::JoinHandle<()>> {
    std::thread::Builder::new().name(name.into()).spawn(f).ok()
}

fn read_lines(out: impl std::io::Read + Send + 'static, tx: Sender<RunEvent>) -> impl FnOnce() + Send + 'static {
    move || {
        let mut r = BufReader::new(out);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match r.by_ref().take(MAX_LINE as u64).read_until(b'\n', &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let line = String::from_utf8_lossy(&buf).trim_end().to_string();
                    if !line.is_empty() && tx.send(RunEvent::Line(line)).is_err() {
                        break;
                    }
                }
            }
        }
    }
}

fn keep_tail(err: impl std::io::Read + Send + 'static, tail: Arc<Mutex<VecDeque<String>>>) -> impl FnOnce() + Send + 'static {
    move || {
        for line in BufReader::new(err).lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            let mut t = tail.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if t.len() == STDERR_LINES {
                t.pop_front();
            }
            t.push_back(line.chars().take(2000).collect());
        }
    }
}

struct Waiter {
    child: Child,
    pid: u32,
    dir: PathBuf,
    limit: Duration,
    stop: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
    tail: Arc<Mutex<VecDeque<String>>>,
    tx: Sender<RunEvent>,
    readers: [Option<std::thread::JoinHandle<()>>; 2],
}

impl Waiter {
    fn wait(mut self) {
        let start = Instant::now();
        let end = loop {
            if let Ok(Some(status)) = self.child.try_wait() {
                break End::Exited(status.code());
            }
            let why = if self.stop.load(Ordering::SeqCst) {
                Some(End::Stopped)
            } else if start.elapsed() >= self.limit {
                Some(End::TimedOut)
            } else {
                None
            };
            if let Some(why) = why {
                kill_tree(self.pid);
                let _ = self.child.wait();
                break why;
            }
            std::thread::sleep(Duration::from_millis(25));
        };
        // A child that exited by itself may have left processes of its group running.
        kill_tree(self.pid);
        for r in self.readers.iter_mut().filter_map(Option::take) {
            let _ = r.join();
        }
        let stderr = self.tail.lock().unwrap_or_else(std::sync::PoisonError::into_inner).iter().cloned().collect();
        let _ = std::fs::remove_dir_all(&self.dir);
        self.done.store(true, Ordering::SeqCst);
        let _ = self.tx.send(RunEvent::Ended { end, stderr, secs: start.elapsed().as_secs_f64() });
    }
}

/// A new empty folder for a run.
fn work_dir() -> std::io::Result<PathBuf> {
    use std::sync::atomic::AtomicU64;
    static N: AtomicU64 = AtomicU64::new(0);
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("scrim-agent-run-{}-{stamp}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir(&dir)?;
    Ok(dir)
}

/// Stop process group `pid` and everything in it: `SIGTERM`, then `SIGKILL` once [`GRACE`] has
/// passed with any of it still there. Windows: `taskkill /T /F` (the tree).
pub fn kill_tree(pid: u32) {
    #[cfg(unix)]
    {
        let group = format!("-{pid}");
        let signal = |sig: &str| {
            Command::new("kill").args([sig, "--", &group]).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
        };
        // `kill -0` succeeds while any process of the group is left.
        if !signal("-0") {
            return;
        }
        signal("-TERM");
        let deadline = Instant::now() + GRACE;
        while Instant::now() < deadline {
            if !signal("-0") {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        signal("-KILL");
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill").args(["/T", "/F", "/PID", &pid.to_string()]).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
}
