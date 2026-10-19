//! The gate an SVG passes before it is parsed (Port 8; `docs/scrim-transfer/specs/svg-allowlist.md`
//! §3.2 and §6): an SVG that could run script or reach outside itself is **refused, never
//! sanitised**. The parser would drop a script or ignore a link, and the import would look fine,
//! so the file is refused with the reason instead.
//!
//! - **Scrim's six tests,** with its reasons, in its order: entities, script elements, `@import`,
//!   external `url()`, event handlers, external `href` / `src`.
//! - **The gaps Scrim's report lists, closed here** (Scrim showed files only as images; this app
//!   parses them):
//!   - a stylesheet processing instruction;
//!   - an external (`SYSTEM`) DTD;
//!   - schemes spelled with character references (`&#106;avascript:`);
//!   - SMIL writing a link (`to=`, `from=`, `by=`, `values=`);
//!   - relative references to other files (`other.svg#a`, `url(other.svg#g)`);
//!   - `foreignObject` (HTML);
//!   - a handler after a slash (`<svg/onload=…>`).
//! - **Structure:** the importer's own caps (nesting 512, 100,000 layers) bound the rest.
//!
//! It is a text match over the whole file, comments included, as Scrim's is: a script inside a
//! comment is refused too.

use std::sync::OnceLock;

use regex::Regex;

struct Tests {
    entity: Regex,
    script: Regex,
    import: Regex,
    url_external: Regex,
    tag: Regex,
    handler: Regex,
    href_external: Regex,
    stylesheet_pi: Regex,
    system_dtd: Regex,
    foreign: Regex,
    reference: Regex,
    url_any: Regex,
}

fn tests() -> Option<&'static Tests> {
    static T: OnceLock<Option<Tests>> = OnceLock::new();
    T.get_or_init(|| {
        let r = |p: &str| Regex::new(p).ok();
        Some(Tests {
            entity: r(r"(?i)<!ENTITY")?,
            script: r(r"(?i)<(?:[a-z0-9_-]+:)?script[\s>/]")?,
            import: r(r"(?i)@import")?,
            url_external: r(r#"(?i)url\(\s*['"]?\s*(?:https?:|file:|javascript:|//)"#)?,
            tag: r(r"(?i)<[a-z][^>]*>")?,
            handler: r(r"(?i)[\s/]on[a-z]+\s*=")?,
            href_external: r(r#"(?i)\s(?:[a-z0-9_-]+:)?(?:href|src|to|from|by|values)\s*=\s*["']?\s*([a-z][a-z0-9+.-]*:|//)"#)?,
            stylesheet_pi: r(r"(?i)<\?xml-stylesheet")?,
            system_dtd: r(r"(?i)<!DOCTYPE[^>\[]*\sSYSTEM\s")?,
            foreign: r(r"(?i)<(?:[a-z0-9_-]+:)?foreignObject[\s>/]")?,
            reference: r(r#"(?i)\s(?:[a-z0-9_-]+:)?(?:href|src)\s*=\s*["']?\s*([^"'\s>]*)"#)?,
            url_any: r(r#"(?i)url\(\s*['"]?\s*([^'")\s]*)"#)?,
        })
    })
    .as_ref()
}

/// `&#106;` and `&#x6A;` as the characters they spell (what an XML parser reads).
fn decode_references(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find("&#") {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 2..];
        let (hex, digits) = match tail.strip_prefix(['x', 'X']) {
            Some(h) => (true, h),
            None => (false, tail),
        };
        let n = digits.bytes().take_while(|b| if hex { b.is_ascii_hexdigit() } else { b.is_ascii_digit() }).count();
        let code = u32::from_str_radix(&digits[..n], if hex { 16 } else { 10 }).ok().and_then(char::from_u32);
        match (code, digits[n..].starts_with(';')) {
            (Some(c), true) if n > 0 => {
                out.push(c);
                rest = &digits[n + 1..];
            }
            _ => {
                out.push_str("&#");
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A reference to something outside this file: not `#id`, not a `data:` URI, not empty.
fn outside(r: &str) -> bool {
    let r = r.trim();
    !(r.is_empty() || r.starts_with('#') || r.to_ascii_lowercase().starts_with("data:"))
}

/// Why `text` is refused, or `None`.
pub fn refusal(text: &str) -> Option<&'static str> {
    let Some(t) = tests() else { return Some("SVG could not be checked") };
    // Scrim's six, in order.
    if t.entity.is_match(text) {
        return Some("SVG declares entities");
    }
    if t.script.is_match(text) {
        return Some("SVG contains a script element");
    }
    if t.import.is_match(text) {
        return Some("SVG imports a stylesheet");
    }
    let decoded = decode_references(text);
    if t.url_external.is_match(text) || t.url_external.is_match(&decoded) {
        return Some("SVG references an external file");
    }
    for tag in t.tag.find_iter(&decoded) {
        // Scrim needs whitespace before `on`; a slash (`<svg/onload=…>`) counts here too.
        if t.handler.is_match(tag.as_str()) {
            return Some("SVG has an event handler attribute");
        }
        // Any scheme but `data:` (Scrim lists http, https, file and javascript; SMIL's `to`,
        // `from`, `by` and `values` write links too).
        if t.href_external.captures_iter(tag.as_str()).any(|c| c.get(1).is_some_and(|m| !m.as_str().eq_ignore_ascii_case("data:"))) {
            return Some("SVG references an external file");
        }
    }
    // The gaps, closed.
    if t.stylesheet_pi.is_match(text) {
        return Some("SVG links a stylesheet");
    }
    if t.system_dtd.is_match(text) {
        return Some("SVG references an external file");
    }
    if t.foreign.is_match(text) {
        return Some("SVG holds HTML (foreignObject)");
    }
    for tag in t.tag.find_iter(&decoded) {
        if t.reference.captures_iter(tag.as_str()).any(|c| c.get(1).is_some_and(|m| outside(m.as_str()))) {
            return Some("SVG references an external file");
        }
    }
    if t.url_any.captures_iter(&decoded).any(|c| c.get(1).is_some_and(|m| outside(m.as_str()))) {
        return Some("SVG references an external file");
    }
    None
}
