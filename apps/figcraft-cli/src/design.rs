//! `lint` and `analyze` (Port 6): the design lint and `analyze_design` on a file, read-only. Each
//! prints the MCP tool's JSON result: the same engine commands (`lint.run`, `design.analyze`).

use std::process::ExitCode;

use figcraft_engine::Session;
use serde_json::{Value, json};

use crate::{CliError, CliResult, files};

fn run(file: &str, command: &str, params: Value) -> CliResult<ExitCode> {
    let doc = files::load(file)?;
    let mut s = Session::with_document(doc);
    let out = s.execute(command, &params).map_err(|e| CliError::Failed(e.to_string()))?;
    println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    Ok(ExitCode::SUCCESS)
}

/// `lint <file> [--id ID] [--profile ui|expressive]`.
pub fn lint(file: &str, id: Option<String>, profile: Option<String>) -> CliResult<ExitCode> {
    let mut p = json!({});
    if let Some(id) = id {
        p["id"] = id.parse::<u64>().map_or(json!(id), |n| json!(n));
    }
    if let Some(pr) = profile {
        p["profile"] = json!(pr);
    }
    run(file, "lint.run", p)
}

/// `analyze <file> [--id ID]`.
pub fn analyze(file: &str, id: Option<String>) -> CliResult<ExitCode> {
    let p = match id {
        Some(id) => json!({"id": id.parse::<u64>().map_or(json!(id), |n| json!(n))}),
        None => json!({}),
    };
    run(file, "design.analyze", p)
}
