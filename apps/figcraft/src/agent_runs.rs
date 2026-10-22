//! The desktop's agent runs for "Describe a change" (R5, `docs/design/chat-composer.md`): the
//! window's `Services::start_agent`, over `figcraft_mcp::agent_run` (an argument list, an empty
//! folder, only the given environment, its own process group). Dropping a run (its window closing,
//! Scrim quitting) stops it with everything it started.

use std::path::PathBuf;
use std::time::Duration;

use figcraft_mcp::agent_run::{End, Run, RunEvent, RunSpec};
use figcraft_ui_egui::{AgentEnd, AgentRun, AgentRunEvent, AgentRunSpec};

/// A run in progress.
struct DesktopRun(Run);

impl AgentRun for DesktopRun {
    fn poll(&mut self) -> Vec<AgentRunEvent> {
        self.0
            .events
            .try_iter()
            .map(|e| match e {
                RunEvent::Line(l) => AgentRunEvent::Line(l),
                RunEvent::Ended { end, stderr, secs } => AgentRunEvent::Ended {
                    end: match end {
                        End::Exited(code) => AgentEnd::Exited(code),
                        End::Stopped => AgentEnd::Stopped,
                        End::TimedOut => AgentEnd::TimedOut,
                    },
                    stderr,
                    secs,
                },
            })
            .collect()
    }

    fn stop(&mut self) {
        self.0.stop();
    }
}

/// Start `spec`.
pub fn start(spec: AgentRunSpec) -> Result<Box<dyn AgentRun>, String> {
    let run = Run::start(RunSpec {
        program: PathBuf::from(spec.program),
        args: spec.args,
        env: spec.env,
        limit: Duration::from_secs(spec.limit_secs.max(1)),
    })
    .map_err(|e| e.0)?;
    Ok(Box::new(DesktopRun(run)))
}
