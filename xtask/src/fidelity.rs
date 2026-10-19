//! `cargo xtask fidelity`: the export guarantee (AGENTS.md › Scrim's rules 6). Every top-level
//! frame of the transfer fixtures is rendered twice:
//! - by the canvas renderer (`figcraft-cli export --node`);
//! - by browsers, from the HTML export (`inspect.code {language: html}`).
//!
//! Scrim's harness (`docs/scrim-transfer/html-fidelity/fidelity.mjs`, Node + playwright-core) then
//! diffs each pair. The result is compared with `xtask/fidelity-baseline.json`: an artboard that
//! gets worse, or fewer identical artboards, fails.
//!
//! Needs Node ≥ 20 and playwright-core with Chromium (and WebKit): set `PWCORE` (playwright-core's
//! `index.mjs`) and `PLAYWRIGHT_BROWSERS_PATH`, as `docs/scrim-transfer/html-fidelity/README.md`
//! says. It isn't part of `cargo xtask ci` because CI machines may lack browsers.
//!
//! Flags:
//! - `--engines chromium,webkit` (default);
//! - `--update` writes the measured numbers as the new baseline;
//! - `--slack PCT` (default 0.05) is how much one artboard may worsen before it fails.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

const FIXTURES: &str = "docs/scrim-transfer/fixtures/fig";
const HARNESS: &str = "docs/scrim-transfer/html-fidelity/fidelity.mjs";
const BASELINE: &str = "xtask/fidelity-baseline.json";
/// The fixtures, in Scrim's order. stage-a is read bare: its zip has CRC 0, which the reader
/// rightly refuses (`fixtures/fig/stage-a.bare.fig` is its `canvas.fig`).
const FILES: &[(&str, &str)] =
    &[("stage-a", "stage-a.bare.fig"), ("stage-b", "stage-b.fig"), ("stage-c", "stage-c.fig"), ("stage-d", "stage-d.fig"), ("lines", "lines.fig")];

pub fn run(root: &Path, args: &[String]) -> bool {
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let engines = flag("--engines").unwrap_or_else(|| "chromium,webkit".into());
    let slack: f64 = flag("--slack").and_then(|s| s.parse().ok()).unwrap_or(0.05);
    let update = args.iter().any(|a| a == "--update");

    let ok = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["build", "--release", "-q", "-p", "figcraft-cli"])
        .current_dir(root)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        eprintln!("fidelity: building figcraft-cli failed");
        return false;
    }
    let target = std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| root.join("target"));
    let cli = target.join("release").join("figcraft-cli");
    let work = target.join("fidelity");
    let _ = std::fs::remove_dir_all(&work);
    if let Err(e) = std::fs::create_dir_all(work.join("pairs")) {
        eprintln!("fidelity: {e}");
        return false;
    }

    let mut manifest = Vec::new();
    for (doc, file) in FILES {
        let path = root.join(FIXTURES).join(file);
        let info = match cli_json(&cli, &["info", &path.to_string_lossy(), "--json"]) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("fidelity: {doc}: {e}");
                return false;
            }
        };
        for frame in info["pages"].as_array().into_iter().flatten().flat_map(|p| p["frames"].as_array().cloned().unwrap_or_default()) {
            let (Some(id), Some(name)) = (frame["id"].as_u64(), frame["name"].as_str()) else { continue };
            let tag = format!("{doc}-{}", slug(name));
            let png = work.join("pairs").join(format!("{tag}.png"));
            let html = work.join("pairs").join(format!("{tag}.html"));
            match pair(&cli, &path, id, &png, &html, &work) {
                Ok(()) => manifest.push(json!({"name": tag, "reference": png, "html": html})),
                Err(e) => eprintln!("fidelity: {tag}: {e} (left out)"),
            }
        }
    }
    let manifest_path = work.join("manifest.json");
    if std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap_or_default()).is_err() {
        return false;
    }
    let out = work.join("out");
    let status =
        Command::new("node").arg(root.join(HARNESS)).arg(&manifest_path).args(["--engines", &engines, "--out"]).arg(&out).current_dir(root).status();
    if !status.map(|s| s.success()).unwrap_or(false) {
        eprintln!("fidelity: the harness failed: Node ≥ 20 and playwright-core are needed (PWCORE, PLAYWRIGHT_BROWSERS_PATH)");
        return false;
    }
    let Ok(summary) = std::fs::read_to_string(out.join("summary.json"))
        .map_err(|e| e.to_string())
        .and_then(|s| serde_json::from_str::<Value>(&s).map_err(|e| e.to_string()))
    else {
        eprintln!("fidelity: no summary.json");
        return false;
    };
    let measured = numbers(&summary);
    if update {
        let text = serde_json::to_string_pretty(&measured).unwrap_or_default() + "\n";
        return std::fs::write(root.join(BASELINE), text).is_ok();
    }
    let baseline: Value = std::fs::read_to_string(root.join(BASELINE)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    let problems = regressions(&baseline, &measured, slack);
    if problems.is_empty() {
        eprintln!("fidelity: ok (no artboard worse than {BASELINE})");
        true
    } else {
        eprintln!("fidelity: the export got worse than {BASELINE}:\n  {}", problems.join("\n  "));
        false
    }
}

/// Export `id` of `file` as the canvas PNG and the wrapped HTML page.
fn pair(cli: &Path, file: &Path, id: u64, png: &Path, html: &Path, work: &Path) -> Result<(), String> {
    let out = Command::new(cli)
        .args(["export", &file.to_string_lossy(), "--node", &id.to_string(), "-o", &png.to_string_lossy()])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("png: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let script = work.join("script.json");
    std::fs::write(&script, json!([{"command": "inspect.code", "params": {"id": id.to_string(), "language": "html"}}]).to_string())
        .map_err(|e| e.to_string())?;
    let out = Command::new(cli).args(["run", &script.to_string_lossy(), "--in", &file.to_string_lossy()]).output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let last: Value = text.lines().last().and_then(|l| serde_json::from_str(l).ok()).ok_or("html: no result")?;
    let code = last["result"]["code"].as_str().ok_or_else(|| format!("html: {last}"))?;
    let page = format!("<!doctype html><html><head><meta charset=\"utf-8\"></head><body style=\"margin:0\">{code}</body></html>");
    std::fs::write(html, page).map_err(|e| e.to_string())
}

fn cli_json(cli: &Path, args: &[&str]) -> Result<Value, String> {
    let out = Command::new(cli).args(args).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())
}

fn slug(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

/// `{engine: {identical, artboards: {name: realPct}}}` from the harness's summary.
pub fn numbers(summary: &Value) -> Value {
    let mut engines = serde_json::Map::new();
    for (engine, total) in summary["totals"].as_object().into_iter().flatten() {
        let mut boards = serde_json::Map::new();
        for row in summary["artboards"].as_array().into_iter().flatten() {
            if let (Some(name), Some(pct)) = (row["name"].as_str(), row["results"][engine]["realPct"].as_f64()) {
                boards.insert(name.to_string(), json!(pct));
            }
        }
        engines.insert(engine.clone(), json!({"identical": total["identical"], "artboards": boards}));
    }
    Value::Object(engines)
}

/// What got worse from `baseline` to `measured`: fewer identical artboards, an artboard's real
/// difference up by more than `slack` percentage points, or an artboard that no longer measures.
pub fn regressions(baseline: &Value, measured: &Value, slack: f64) -> Vec<String> {
    let mut out = Vec::new();
    for (engine, base) in baseline.as_object().into_iter().flatten() {
        let now = &measured[engine];
        if let (Some(b), Some(n)) = (base["identical"].as_u64(), now["identical"].as_u64())
            && n < b
        {
            out.push(format!("{engine}: {n} identical, the baseline has {b}"));
        }
        for (name, pct) in base["artboards"].as_object().into_iter().flatten() {
            match (pct.as_f64(), now["artboards"][name].as_f64()) {
                (Some(b), Some(n)) if n > b + slack => out.push(format!("{engine} {name}: real {n}% (baseline {b}%)")),
                (Some(_), None) => out.push(format!("{engine} {name}: not measured any more")),
                _ => {}
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worse_artboards_and_fewer_identical_ones_are_regressions() {
        let base = json!({"chromium": {"identical": 8, "artboards": {"a": 0.0, "b": 12.2, "c": 3.0}}});
        let same = json!({"chromium": {"identical": 8, "artboards": {"a": 0.0, "b": 12.24, "c": 1.0}}});
        assert!(regressions(&base, &same, 0.05).is_empty());
        let worse = json!({"chromium": {"identical": 7, "artboards": {"a": 0.5, "b": 12.2}}});
        let p = regressions(&base, &worse, 0.05);
        assert_eq!(p.len(), 3, "{p:?}");
    }

    #[test]
    fn the_summary_reduces_to_identical_counts_and_real_percentages() {
        let summary = json!({"totals": {"chromium": {"identical": 1, "artboards": 2}}, "artboards": [
            {"name": "x", "results": {"chromium": {"realPct": 0.0}}}, {"name": "y", "results": {"chromium": {"realPct": 2.5}}}]});
        assert_eq!(numbers(&summary), json!({"chromium": {"identical": 1, "artboards": {"x": 0.0, "y": 2.5}}}));
    }
}
