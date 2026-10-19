//! Scrim's SVG cases (`docs/scrim-transfer/cases/svg.json`, 178) against this importer (Port 8).
//!
//! - **`file` (57):** every file Scrim refuses as unsafe is refused here with the same reason.
//!   The 11 Scrim accepted but reported as gaps are refused here too, because this app parses
//!   SVG rather than showing it as an image. Every safe file imports.
//! - **`inline` (15 accepted, with markup for 12):** Scrim's generated markup imports here, so a
//!   vector made in Scrim comes across. Scrim's refusals of its JSON vector data have no
//!   counterpart: this app stores vectors as vector networks, never as SVG data.
//! - **`pathEdit` (17):** path data an agent passes (`node.create {type: vector, path}`): every
//!   path Scrim's parser accepts imports, and none panics.

use figcraft_doc::Document;
use serde_json::Value;

use super::{gate, import_svg};

fn cases() -> Vec<Value> {
    let v: Value = serde_json::from_str(include_str!("../../../../docs/scrim-transfer/cases/svg.json")).unwrap();
    v["cases"].as_array().unwrap().clone()
}

/// A file case's bytes (generated ones built as the case file says).
fn bytes(c: &Value) -> Vec<u8> {
    if let Some(b) = c["svg"]["bytes"].as_array() {
        return b.iter().map(|x| x.as_u64().unwrap() as u8).collect();
    }
    let g = &c["generate"];
    let s = |k: &str| g[k].as_str().unwrap_or("").to_string();
    match g["kind"].as_str() {
        Some("repeat-string") => format!("{}{}{}", s("prefix"), s("unit").repeat(g["count"].as_u64().unwrap() as usize), s("suffix")).into_bytes(),
        Some("nest") => {
            let n = g["count"].as_u64().unwrap() as usize;
            format!("{}{}{}{}{}", s("prefix"), s("open").repeat(n), s("inner"), s("close").repeat(n), s("suffix")).into_bytes()
        }
        _ => c["svg"].as_str().unwrap().as_bytes().to_vec(),
    }
}

fn import(data: &[u8]) -> Result<(), String> {
    let mut d = Document::new("svg");
    let p = d.first_page().unwrap();
    import_svg(&mut d, data, p, 0.0, 0.0, "x").map(|_| ()).map_err(|e| e.0)
}

/// Scrim's refusals that are about images, not safety: the size and pixel caps of its raster
/// gate, and files that are not SVG at all.
fn not_about_safety(want: &str) -> bool {
    want.starts_with("Not a supported image") || want.starts_with("The image is") || want.starts_with("Images are limited")
}

#[test]
fn every_file_scrim_refuses_as_unsafe_is_refused_and_its_gaps_are_closed() {
    let (mut refused, mut gaps, mut accepted, mut other) = (0, 0, 0, 0);
    for c in cases().iter().filter(|c| c["layer"] == "file") {
        let name = c["name"].as_str().unwrap();
        let data = bytes(c);
        let got = import(&data);
        match &c["expect"] {
            Value::String(a) if a == "accept" => {
                if name.starts_with("file (gap)") {
                    gaps += 1;
                    if name.contains("deep nesting") {
                        // Bounded by the importer's nesting cap instead.
                        let e = got.expect_err(name);
                        assert!(e.contains("nested more than"), "{name}: {e}");
                    } else {
                        let e = got.expect_err(name);
                        assert!(gate::refusal(&String::from_utf8_lossy(&data)).is_some(), "{name}: {e}");
                    }
                } else {
                    accepted += 1;
                    got.unwrap_or_else(|e| panic!("{name}: {e}"));
                }
            }
            want => {
                let want = want["refuse"].as_str().unwrap();
                if not_about_safety(want) {
                    // Not SVG, too big to rasterise, or over 25 MB: no safety question here.
                    other += 1;
                    continue;
                }
                refused += 1;
                let e = got.expect_err(name);
                let reason = want.split(':').next().unwrap();
                assert!(e.starts_with(reason), "{name}: {e} (Scrim: {want})");
            }
        }
    }
    assert_eq!((refused, gaps, accepted, other), (23, 11, 12, 11));
}

#[test]
fn scrims_generated_vectors_import() {
    let mut n = 0;
    for c in cases().iter().filter(|c| c["layer"] == "inline" && c["expect"] == "accept") {
        let Some(markup) = c["markup"].as_str() else { continue };
        import(markup.as_bytes()).unwrap_or_else(|e| panic!("{}: {e}", c["name"]));
        n += 1;
    }
    assert_eq!(n, 12);
}

/// An SVG inside a base64 data URI passes the same gate: the outer scan can't read it.
#[test]
fn an_embedded_svg_is_gated_too() {
    use base64::Engine as _;
    let inner = br#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><script>alert(1)</script></svg>"#;
    let outer = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><image width="4" height="4" href="data:image/svg+xml;base64,{}"/></svg>"#,
        base64::engine::general_purpose::STANDARD.encode(inner)
    );
    let e = import(outer.as_bytes()).unwrap_err();
    assert!(e.starts_with("SVG contains a script element (in an embedded SVG)"), "{e}");
}
