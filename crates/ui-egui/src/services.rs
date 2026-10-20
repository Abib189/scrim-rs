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

impl ProgramOutput {
    pub fn success(&self) -> bool {
        self.code == Some(0)
    }
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
    /// Run `program` with `args` (an argument list, never a shell command line) and wait for it.
    /// Only the desktop app runs programs, and only after the person confirmed.
    fn run_program(&mut self, _program: &str, _args: &[String]) -> Result<ProgramOutput, String> {
        Err("this version of Scrim can't run programs".into())
    }
}
