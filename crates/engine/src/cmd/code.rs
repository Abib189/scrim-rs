//! The Code tab's commands (`docs/design/panel-redesign.md`, B2), so an agent sees and copies what
//! the person does: `code.setFormat` picks the language the tab shows, `code.copy` copies a layer's
//! code (as `inspect.code` returns it), and `code.copyStandaloneHtml` copies a complete page for a
//! layer. They only read the document. Copying asks the UI to put the text on the clipboard;
//! headless (MCP, the CLI) the text is returned and nothing else happens.

use serde_json::{Value, json};

use super::{CommandSpec, find_command};
use crate::codegen::{CodeLang, code_for};
use crate::params::*;
use crate::{EngineError, Result, Session, bad, spec};

/// The UI request that switches the Code tab's language and unit (`{format, unit}`).
pub const FORMAT_REQUEST: &str = "code.format";
/// The UI request that puts text on the clipboard and says so (`{text, what}`).
pub const COPY_REQUEST: &str = "clipboard.text";

/// The Content Security Policy of a standalone page: inline styles and `data:` images and fonts
/// only, so opening the page fetches nothing (a missing image's `url(image.png)` included).
const CSP: &str = "default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:";

fn format_of(cmd: &str, p: &Value) -> Result<(CodeLang, &'static str)> {
    let lang = match string(p, "format").or_else(|| string(p, "language")) {
        Some(f) => CodeLang::from_id(&f).ok_or_else(|| bad(cmd, "format: css|html|react|leptos|swiftui|compose"))?,
        None => return Err(bad(cmd, "`format` is required: css|html|react|leptos|swiftui|compose")),
    };
    let unit = match string(p, "unit").map(|u| u.to_ascii_lowercase()).as_deref() {
        None | Some("px") => "px",
        Some("rem") if lang.is_web() => "rem",
        Some("rem") => return Err(bad(cmd, "rem is for the web formats (css, html, react, leptos)")),
        Some(_) => return Err(bad(cmd, "unit: px|rem")),
    };
    Ok((lang, unit))
}

/// `inspect.code`, run as itself (one implementation of a layer's code).
fn inspect_code(s: &mut Session, p: &Value) -> Result<Value> {
    let spec = find_command("inspect.code").ok_or_else(|| EngineError::UnknownCommand("inspect.code".into()))?;
    (spec.run)(s, p)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A complete page for layer `id`: the HTML export's markup in the body, its stylesheet in the
/// head, the layer's name as the title, and a policy that keeps the page from fetching anything.
/// Images are inlined by the export already.
pub fn standalone_html(doc: &figcraft_doc::Document, id: figcraft_doc::NodeId) -> String {
    let code = code_for(doc, id, CodeLang::Html);
    let (markup, style) = match code.find("<style>") {
        Some(at) => (code[..at].trim_end(), code[at..].trim()),
        None => (code.trim(), ""),
    };
    let title = doc.node(id).map(|n| escape(&n.name)).unwrap_or_default();
    format!(
        "<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta http-equiv=\"Content-Security-Policy\" content=\"{CSP}\">\n<title>{title}</title>\n{style}\n</head>\n<body style=\"margin:0\">\n{markup}\n</body>\n</html>\n"
    )
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        spec!(
            "code.setFormat",
            "Code format",
            "{format: css|html|react|leptos|swiftui|compose, unit?: px|rem (web formats; default px)} → {format, unit}  (what the Code tab shows; inspect.code returns any format without switching)",
            false,
            |s, p| {
                let (lang, unit) = format_of("code.setFormat", p)?;
                let r = json!({"format": lang.id(), "unit": unit});
                s.request_ui(FORMAT_REQUEST, r.clone());
                Ok(r)
            }
        ),
        spec!(
            "code.copy",
            "Copy code",
            "{id?, language?: css|html|react|leptos|swiftui|compose (default css), unit?: px|rem, rootFontSize?} → {code, language, connected}  (inspect.code, then onto the clipboard in the app)",
            false,
            |s, p| {
                let r = inspect_code(s, p)?;
                let what = CodeLang::from_id(r["language"].as_str().unwrap_or("css")).unwrap_or_default().label();
                s.request_ui(COPY_REQUEST, json!({"text": r["code"], "what": what}));
                Ok(r)
            }
        ),
        spec!(
            "code.copyStandaloneHtml",
            "Copy standalone HTML",
            "{id? (default: the first selected layer)} → {html, id, name}  (a complete page: doctype, the stylesheet in the head, images inline, and a Content Security Policy that fetches nothing; onto the clipboard in the app)",
            false,
            |s, p| {
                let cmd = "code.copyStandaloneHtml";
                let id = targets(s, p).first().copied().ok_or_else(|| bad(cmd, "select a layer"))?;
                let doc = s.doc()?;
                let name = doc.node(id).map(|n| n.name.clone()).ok_or_else(|| bad(cmd, "no such layer"))?;
                let html = standalone_html(doc, id);
                s.request_ui(COPY_REQUEST, json!({"text": html, "what": "standalone HTML"}));
                Ok(json!({"html": html, "id": id.0, "name": name}))
            }
        ),
    ]
}

#[cfg(test)]
#[path = "code_tests.rs"]
mod tests;
