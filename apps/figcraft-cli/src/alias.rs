//! `figcraft-cli`: the old name of `scrim`, kept as a thin alias (docs/design/rebrand.md, A3). It
//! says so in one line on stderr (stdout stays the command's own, so `figcraft-cli mcp` keeps
//! working for an existing agent configuration), then runs the `scrim` next to it with the same
//! arguments, input and output, and exits with its status.

use std::process::{Command, ExitCode};

/// The note, one line.
pub const NOTE: &str = "note: figcraft-cli is now called `scrim`; this old name will be removed (see docs/cli.md)";

fn main() -> ExitCode {
    eprintln!("{NOTE}");
    let Ok(me) = std::env::current_exe() else {
        eprintln!("error: can't find where figcraft-cli is installed");
        return ExitCode::from(1);
    };
    let scrim = me.with_file_name(if cfg!(windows) { "scrim.exe" } else { "scrim" });
    match Command::new(&scrim).args(std::env::args_os().skip(1)).status() {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1).clamp(0, 255) as u8),
        Err(e) => {
            eprintln!("error: can't run {} ({e}): install `scrim` next to figcraft-cli", scrim.display());
            ExitCode::from(1)
        }
    }
}
