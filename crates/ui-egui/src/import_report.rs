//! The import report: what a successful `.fig`, REST JSON or SVG import left out or changed, shown
//! in a dialog window after the file opens, so the notes don't vanish with a toast.

use egui::RichText;
use serde_json::{Value, json};

use crate::FigApp;
use crate::i18n::{tr, trn, try_tr};
use crate::theme::{Tokens, regular};

/// A report stays small even when a damaged input produces many or very long notes.
pub const MAX_NOTES: usize = 500;
pub const MAX_NOTE_CHARS: usize = 2_000;

/// One thing an import left out or changed, as an importer reported it.
#[derive(Clone, Debug)]
pub struct Note {
    /// `dropped`, `approximated` or `info` (`figcraft_format::import_report::Severity::id`).
    pub severity: String,
    /// The layer it was on (names from the page down); empty for the file as a whole.
    pub path: String,
    pub message: String,
    /// `message` in the UI language (worked out once, not every frame).
    pub shown: String,
}

#[derive(Clone, Debug)]
pub struct Report {
    pub name: String,
    pub pages: Option<u64>,
    pub layers: Option<u64>,
    pub images: Option<u64>,
    /// The pages of an imported file were added to the open document.
    pub appended: bool,
    /// An SVG was placed on the current page of the open document.
    pub placed: bool,
    /// The document the report belongs to: it goes when that document closes.
    pub uid: u64,
    pub notes: Vec<Note>,
    pub note_count: usize,
    /// How many of each severity, `[dropped, approximated, info]`, all notes counted.
    pub counts: [usize; 3],
}

/// The severities in the order the report counts them.
const SEVERITIES: [&str; 3] = ["dropped", "approximated", "info"];

impl Report {
    /// Only successful `file.open` and `file.import` results with import `warnings` create a report
    /// (Fix AG: an import without any shows nothing); a native file's `repairs` are shown by the
    /// engine's own message. `params` are the command's, for the file's name.
    pub fn from_result(command: &str, params: &Value, result: &Value) -> Option<Self> {
        if !matches!(command, "file.open" | "file.openRecent" | "file.import") {
            return None;
        }
        let warnings = result.get("warnings").and_then(Value::as_array).filter(|w| !w.is_empty())?;
        let file_name = params.get("path").and_then(Value::as_str).and_then(|p| std::path::Path::new(p).file_name()).and_then(|n| n.to_str());
        // `file.import` of an SVG places a frame on the current page (no `pages`): its notes
        // are reported, but no pages were appended.
        let appended = command == "file.import" && result.get("pages").is_some();
        let summary = result.get("summary");
        let count = |key: &str| summary.and_then(|s| s.get(key)).and_then(Value::as_u64);
        let name = summary
            .and_then(|s| s.get("name"))
            .and_then(Value::as_str)
            .or(file_name)
            .or_else(|| params.get("name").and_then(Value::as_str))
            .unwrap_or(tr("Imported document"));
        let mut report = Self {
            name: name.chars().take(512).collect(),
            pages: count("pages"),
            layers: count("layers"),
            images: count("images"),
            appended,
            placed: command == "file.import" && !appended,
            uid: 0,
            notes: Vec::new(),
            note_count: 0,
            counts: [0; 3],
        };
        for w in warnings {
            // `{severity, path, message}`; a plain sentence (older results) is a note.
            let field = |k: &str| w.get(k).and_then(Value::as_str).unwrap_or_default();
            match w.as_str() {
                Some(text) => report.add_note("info", "", text),
                None => report.add_note(field("severity"), field("path"), field("message")),
            }
        }
        Some(report)
    }

    fn add_note(&mut self, severity: &str, path: &str, message: &str) {
        self.note_count = self.note_count.saturating_add(1);
        let severity = if SEVERITIES.contains(&severity) { severity } else { "info" };
        if let Some(c) = SEVERITIES.iter().position(|s| *s == severity).and_then(|i| self.counts.get_mut(i)) {
            *c += 1;
        }
        if self.notes.len() < MAX_NOTES {
            let clip = |t: &str| {
                let mut text: String = t.chars().take(MAX_NOTE_CHARS).collect();
                if t.chars().nth(MAX_NOTE_CHARS).is_some() {
                    text.push('…');
                }
                text
            };
            let message = clip(message);
            self.notes.push(Note { severity: severity.into(), path: clip(path), shown: note_text(&message), message });
        }
    }

    /// The report as text, for Copy: its name, the counts and one line per note.
    pub fn text(&self) -> String {
        let mut out = format!("{}\n{}\n", self.name, self.count_line());
        for n in &self.notes {
            let at = if n.path.is_empty() { String::new() } else { format!("{}: ", n.path) };
            out.push_str(&format!("- [{}] {at}{}\n", severity_label(&n.severity), n.shown));
        }
        out
    }

    /// "2 left out · 1 approximated · 3 notes" (only the severities there are).
    pub fn count_line(&self) -> String {
        let [dropped, approximated, info] = self.counts;
        let parts = [
            (dropped > 0).then(|| trn("{n} left out", dropped, &[])),
            (approximated > 0).then(|| trn("{n} approximated", approximated, &[])),
            (info > 0).then(|| trn("{n} for information", info, &[])),
        ];
        parts.into_iter().flatten().collect::<Vec<_>>().join(" · ")
    }

    pub fn inspect(&self) -> Value {
        let warnings: Vec<Value> = self.notes.iter().map(|n| json!({"severity": n.severity, "path": n.path, "message": n.message})).collect();
        let [dropped, approximated, info] = self.counts;
        json!({"name": self.name, "pages": self.pages, "layers": self.layers, "images": self.images,
            "appended": self.appended, "placed": self.placed, "warnings": warnings, "warningCount": self.note_count,
            "counts": {"dropped": dropped, "approximated": approximated, "info": info}})
    }
}

/// A severity as the report names it.
fn severity_label(severity: &str) -> &'static str {
    match severity {
        "dropped" => tr("Left out"),
        "approximated" => tr("Approximated"),
        _ => tr("For information"),
    }
}

/// `note` in the UI language when the catalog has it (the importers' fixed notes: `.fig` imports
/// name "the file" where REST imports name the JSON they read); a repeat count (` (×N)`) is kept.
/// The stored notes are lower-case fragments; a known note is shown as a sentence, so its first
/// letter is capitalised here. Other notes (SVG's, say) are shown as they are.
pub(crate) fn note_text(note: &str) -> String {
    let translated = |base: &str| {
        let Some(text) = try_tr(base) else { return base.to_owned() };
        let mut chars = text.chars();
        chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
    };
    match note.rsplit_once(" (×") {
        Some((base, count)) if count.strip_suffix(')').is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())) => {
            format!("{} (×{count}", translated(base))
        }
        _ => translated(note),
    }
}

/// Show the report for a file the user opened or imported (menus, drops, pastes, recents, the
/// system's open event); commands from the control channel, MCP or scripts never call this.
pub fn capture(app: &mut FigApp, command: &str, params: &Value, result: &Value) {
    if let Some(mut report) = Report::from_result(command, params, result) {
        report.uid = app.session.ds().map_or(0, |d| d.uid);
        app.ui.import_report = Some(report);
    }
}

pub fn show(app: &mut FigApp, ctx: &egui::Context) {
    // The report goes with its document.
    if app.ui.import_report.as_ref().is_some_and(|r| !app.session.docs.iter().any(|d| d.uid == r.uid)) {
        app.ui.import_report = None;
    }
    let Some(report) = &app.ui.import_report else { return };
    let t = Tokens::get(ctx);
    let mut open = true;
    let mut close_clicked = false;
    let mut copy_clicked = false;
    crate::dialogs::window(tr("Import report"), ctx).open(&mut open).show(ctx, |ui| {
        ui.set_width(420.0);
        ui.label(RichText::new(&report.name).strong());
        ui.label(if report.appended {
            tr("Pages added to the document")
        } else if report.placed {
            tr("Placed on the current page")
        } else {
            tr("Opened as a new document")
        });
        let counts: Vec<String> = [(tr("Pages"), report.pages), (tr("Layers"), report.layers), (tr("Images"), report.images)]
            .into_iter()
            .filter_map(|(label, count)| count.map(|n| format!("{label}: {n}")))
            .collect();
        if !counts.is_empty() {
            ui.label(counts.join(" · "));
        }
        ui.add_space(8.0);
        ui.label(RichText::new(tr("Compatibility notes")).strong());
        ui.label(
            RichText::new(tr(
                "Scrim doesn't support every feature of these files yet. The notes list what was left out or changed in the imported copy; the original file is unchanged.",
            ))
            .color(t.text_secondary)
            .font(regular(11.0)),
        );
        ui.add_space(4.0);
        ui.label(RichText::new(report.count_line()).strong());
        ui.add_space(4.0);
        // Room for five or so notes before it scrolls (Fix AH5: the fifth was cut off at 260).
        egui::ScrollArea::vertical().id_salt("import_report_notes").max_height(360.0).show(ui, |ui| {
            for note in &report.notes {
                ui.label(RichText::new(format!("{} · {}", severity_label(&note.severity), if note.path.is_empty() { tr("The file") } else { &note.path })).color(t.text_secondary).font(regular(11.0)));
                ui.label(&note.shown);
                ui.add_space(4.0);
            }
        });
        let omitted = report.note_count.saturating_sub(report.notes.len());
        if omitted > 0 {
            ui.label(trn("{n} more notes omitted from this report", omitted, &[]));
        }
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            copy_clicked = crate::widgets::button(ui, tr("Copy report"), false, true).clicked();
            close_clicked = crate::widgets::button(ui, tr("Close"), true, true).clicked();
        });
    });
    if copy_clicked {
        let text = report.text();
        app.services.clipboard_set_text(&text);
        app.toast(tr("Copied as text"));
    }
    if !open || close_clicked {
        app.ui.import_report = None;
    }
}
