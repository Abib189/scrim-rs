//! `query` and `find` (Port 5): Scrim's XPath-style layer query on a file, read-only. The
//! argument rules and the output are the engine's (`figcraft_engine::cmd::query::run_cli`), so
//! they are tested against Scrim's own CLI cases there; this file only loads the file.

use std::path::Path;
use std::process::ExitCode;

use figcraft_engine::cmd::query::{CliFail, FIGCRAFT, QueryOut, query_doc, run_cli};

/// The file size the CLI reads, as Scrim's: refused from its metadata before it is read.
const MAX_FILE: u64 = 200 * 1024 * 1024;

fn load_and_query(file: &str, query: &str, limit: Option<f64>) -> Result<QueryOut, CliFail> {
    let path = Path::new(file);
    let meta = std::fs::metadata(path).map_err(|_| CliFail::new("not_found", format!("no such file: {file}")))?;
    if !meta.is_file() {
        return Err(CliFail::new("not_a_file", format!("{file} is not a file")));
    }
    let base = path.file_name().map_or_else(|| file.to_string(), |n| n.to_string_lossy().to_string());
    if meta.len() > MAX_FILE {
        return Err(CliFail::new(
            "too_large",
            format!("\u{201c}{base}\u{201d} is {} MB: figcraft-cli queries files up to 200 MB", meta.len() / (1024 * 1024)),
        ));
    }
    let doc = figcraft_engine::cmd::file::load(file).map_err(|e| CliFail::new("invalid_document", format!("{base}: {e}")))?.0;
    query_doc(&doc, query, limit, None).map_err(|e| CliFail::new("invalid_query", format!("invalid query: {e}")))
}

/// Run `query` or `find` with the arguments after the command name.
pub fn run(cmd: &str, args: &[String]) -> ExitCode {
    let out = run_cli("figcraft-cli", &FIGCRAFT, cmd, args, load_and_query);
    print!("{}", out.stdout);
    eprint!("{}", out.stderr);
    ExitCode::from(out.exit as u8)
}
