//! Settings ▸ Diagnostics as commands (C3d): the events, Copy, Save, Clear and the retention. The
//! events themselves are recorded by the app (`crate::settings::diagnostics`); nothing here sends
//! anything anywhere.

use serde_json::{Value, json};

use super::CommandSpec;
use super::mcp::person_only;
use crate::settings::diagnostics::{self, RETENTION_DAYS};
use crate::{Session, bad, spec};

/// The UI request that asks where to save the events (an agent's call opens nothing).
pub const SAVE_DIALOG: &str = "dialog.diagnosticsSave";

fn state(s: &Session) -> Value {
    let d = &s.prefs.settings;
    json!({
        "events": d.diagnostics.iter().rev().map(|e| json!({"at": e.at, "time": diagnostics::timestamp(e.at), "kind": e.kind, "detail": e.detail})).collect::<Vec<_>>(),
        "retentionDays": d.diagnostics_days,
    })
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "diagnostics.events",
            "Diagnostics",
            "{} → {events: [{at, time, kind, detail}] (newest first), retentionDays}  (kept on this computer only; no document content, prompts or tokens)",
            false,
            |s, _| Ok(state(s))
        ),
        spec!("diagnostics.copy", "Copy diagnostics", "{} → {lines}  (the events as text, onto the clipboard in the app)", false, |s, _| {
            let text = diagnostics::text(&s.prefs.settings.diagnostics);
            let lines = text.lines().count();
            s.request_ui(super::code::COPY_REQUEST, json!({"text": text, "what": "the diagnostics"}));
            Ok(json!({"lines": lines}))
        }),
        spec!(
            "diagnostics.save",
            "Save diagnostics…",
            "{path?} → {path, lines} | {asked: true}  (the events as a text file; without `path` the app asks where)",
            false,
            |s, p| {
                let cmd = "diagnostics.save";
                let Some(path) = p.get("path").and_then(Value::as_str).filter(|p| !p.trim().is_empty()) else {
                    s.request_ui(SAVE_DIALOG, json!({}));
                    return Ok(json!({"asked": true}));
                };
                let text = diagnostics::text(&s.prefs.settings.diagnostics);
                crate::file_access::write(path, text.as_bytes()).map_err(|e| bad(cmd, e.to_string()))?;
                Ok(json!({"path": path, "lines": text.lines().count()}))
            }
        ),
        spec!("diagnostics.clear", "Clear diagnostics", "{} → {cleared}  (the person's alone)", false, |s, _| {
            person_only(s, "diagnostics.clear", "clear the diagnostics")?;
            let n = s.prefs.settings.diagnostics.len();
            s.prefs.settings.diagnostics.clear();
            Ok(json!({"cleared": n}))
        }),
        spec!(
            "diagnostics.retention",
            "Keep diagnostics for",
            "{days: 0|1|7|30, now? (unix seconds: drop what is older at once)} → {retentionDays}  (0 keeps none)",
            false,
            |s, p| {
                let cmd = "diagnostics.retention";
                let days = p.get("days").and_then(Value::as_u64).and_then(|d| u32::try_from(d).ok()).filter(|d| RETENTION_DAYS.contains(d));
                let days = days.ok_or_else(|| bad(cmd, "`days` is 0, 1, 7 or 30"))?;
                s.prefs.settings.diagnostics_days = days;
                if let Some(now) = p.get("now").and_then(Value::as_u64) {
                    diagnostics::prune(&mut s.prefs.settings, now);
                } else if days == 0 {
                    s.prefs.settings.diagnostics.clear();
                }
                Ok(json!({"retentionDays": days}))
            }
        ),
    ]
}
