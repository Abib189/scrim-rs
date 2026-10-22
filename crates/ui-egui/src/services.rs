//! The platform services the app crate provides (file dialogs, clipboard, URLs, the MCP endpoint,
//! running a program), as one trait with no-op defaults (moved out of `lib.rs`).

use figcraft_engine::Session;

use crate::DialogFuture;

/// What a program the app ran printed, and how it ended.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProgramOutput {
    /// `None`: ended by a signal.
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// A program started by [`Services::run_program`]: its output, once it ends.
pub type ProgramRun = std::sync::mpsc::Receiver<Result<ProgramOutput, String>>;

impl ProgramOutput {
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }
}

/// An agent CLI run for "Describe a change" (R5): what to start. The program is started from this
/// argument list, never a shell, with this environment and nothing else.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentRunSpec {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    /// Stopped after this long.
    pub limit_secs: u64,
}

/// How an agent run ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentEnd {
    /// It exited by itself, with this code (`None`: a signal).
    Exited(Option<i32>),
    /// Stopped (Stop, Esc).
    Stopped,
    /// Its time limit.
    TimedOut,
}

/// What an agent run reports.
#[derive(Clone, Debug, PartialEq)]
pub enum AgentRunEvent {
    /// A line of its output.
    Line(String),
    /// It ended, how, the last lines of its error output, and how long it took.
    Ended { end: AgentEnd, stderr: Vec<String>, secs: f64 },
}

/// An agent run in progress. Dropping it stops it, and everything it started.
pub trait AgentRun {
    /// What happened since the last call.
    fn poll(&mut self) -> Vec<AgentRunEvent>;
    /// Stop it, with everything it started.
    fn stop(&mut self);
}

/// Platform services the app crate provides (file dialogs, clipboard, URLs).
pub trait Services {
    fn pick_open(&mut self, _title: &str, _exts: &[&str]) -> DialogFuture {
        Box::pin(std::future::ready(None))
    }
    fn pick_save(&mut self, _title: &str, _name: &str, _exts: &[&str]) -> DialogFuture {
        Box::pin(std::future::ready(None))
    }
    /// The platform saves files by handing them over as downloads (the browser), not through
    /// save dialogs.
    fn downloads(&self) -> bool {
        false
    }
    /// Hand `bytes` over as a download named `name`; false when nothing was delivered.
    fn download(&mut self, _name: &str, _bytes: &[u8]) -> bool {
        false
    }
    /// A folder to write several files into.
    fn pick_folder(&mut self, _title: &str) -> DialogFuture {
        Box::pin(std::future::ready(None))
    }
    fn clipboard_set_text(&mut self, _s: &str) {}
    fn clipboard_get_text(&mut self) -> Option<String> {
        None
    }
    fn clipboard_get_html(&mut self) -> Option<String> {
        None
    }
    fn clipboard_set_png(&mut self, _w: u32, _h: u32, _rgba: &[u8]) {}
    /// An image on the system clipboard as (width, height, straight RGBA).
    fn clipboard_get_image(&mut self) -> Option<(u32, u32, Vec<u8>)> {
        None
    }
    fn open_url(&mut self, _url: &str) {}
    /// Show the file at `path` in the platform's file manager (Finder, Explorer), selected where the
    /// platform can.
    fn reveal_path(&mut self, _path: &std::path::Path) {}
    /// The system asks for reduced motion (its accessibility setting).
    fn reduce_motion(&self) -> bool {
        false
    }
    /// After a panic in a frame: keep recovery copies of unsaved files.
    fn emergency_save(&mut self, _session: &Session) {}
    /// Keep the preferences now (the installed plugins or their settings changed), as the app
    /// does when it quits.
    fn save_prefs(&mut self, _prefs: &figcraft_engine::Prefs) {}
    /// The AI panel's conversations as last kept (Fix T4): JSON, or nothing. Only the desktop app
    /// keeps them, in its settings folder, never in a document.
    fn load_conversations(&mut self) -> Option<String> {
        None
    }
    /// Keep the AI panel's conversations (JSON).
    fn save_conversations(&mut self, _json: &str) {}
    /// The OS or browser language as a tag ("es-ES"), for the "system" language preference.
    fn system_language(&self) -> Option<String> {
        None
    }
    /// The controller buttons held now on any controller: bit i is standard gamepad layout index i
    /// (0 the bottom face button … 16 home; `figcraft_doc::proto::GAMEPAD_BUTTONS`). The prototype
    /// player asks every frame while its prototype has gamepad triggers.
    fn gamepad_held(&mut self) -> u32 {
        0
    }

    /// Stop the app's MCP endpoint (if it runs) and start it with `settings`: on their port, or the
    /// next free one. Returns the port it listens on. Only the desktop app has one.
    fn mcp_restart(&mut self, _settings: &figcraft_engine::settings::McpSettings) -> Result<u16, String> {
        Err("this version of Scrim has no MCP endpoint".into())
    }
    /// Apply changed settings (the token, the tools turned off) to the running endpoint.
    fn mcp_configure(&mut self, _settings: &figcraft_engine::settings::McpSettings) {}
    /// Start `program` with `args` (an argument list, never a shell command line) in the
    /// background; its output arrives on the returned channel. Only the desktop app runs programs,
    /// and only after the person confirmed (`connect_flow`).
    fn run_program(&mut self, _program: &str, _args: &[String]) -> ProgramRun {
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = tx.send(Err("this version of Scrim can't run programs".into()));
        rx
    }
    /// Where the program `name` is: on the `PATH`, or in a usual install folder (an app started
    /// from the Finder or a desktop menu gets a short `PATH`). Nothing is run to find it.
    fn find_program(&self, _name: &str) -> Option<String> {
        None
    }
    /// The binary that runs `mcp-bridge` (Fix P): the desktop app's own executable. Elsewhere (the
    /// browser, tests) none, so Claude Desktop's entry falls back to `mcp-remote`, to copy.
    fn bridge_command(&self) -> Option<String> {
        None
    }
    /// Claude Desktop's configuration file on this computer (`cmd::claude_desktop`). Only the desktop
    /// app knows it; elsewhere (the browser, tests) there is none, so nothing reads the real file.
    fn claude_desktop_config(&self) -> Option<std::path::PathBuf> {
        None
    }
    /// Where other agents' configurations are found (`cmd::agent_clients`): the home folder and the
    /// environment. Only the desktop app gives it; elsewhere nothing reads a real configuration.
    fn agent_client_env(&self) -> Option<figcraft_engine::cmd::agent_clients::PathEnv> {
        None
    }
    /// Agents started working (`on`) or stopped (R3a): the desktop app keeps the OS from napping it
    /// meanwhile (macOS App Nap). Elsewhere nothing.
    fn keep_awake(&mut self, _on: bool) {}
    /// Start an agent CLI for "Describe a change" (R5). Only the desktop app can; elsewhere (the
    /// browser, tests) it says so.
    fn start_agent(&mut self, _spec: AgentRunSpec) -> Result<Box<dyn AgentRun>, String> {
        Err("agent runs need the desktop app".into())
    }
}
