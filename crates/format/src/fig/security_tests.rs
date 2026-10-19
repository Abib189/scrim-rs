//! Scrim's hostile `.fig` inputs (`docs/scrim-transfer/cases/fig-import.json › security`, Port 4)
//! against this importer: none may panic or run away, and every one Scrim refuses at its default
//! limits is refused here too. Plus the stress cases Scrim's file only describes: deep nesting and
//! very many layers.

use std::time::{Duration, Instant};

use base64::Engine as _;
use serde_json::Value;

use super::tests::{arr, canvas, node, o, schema, xy};
use super::*;

fn cases() -> Vec<Value> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/scrim-transfer/cases/fig-import.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    v["security"].as_array().unwrap().clone()
}

/// Cases this importer may accept, with why.
/// - Small inputs Scrim refuses only under shrunken test limits.
/// - The one Scrim accepts.
/// - The stress cases Scrim only observed. Here the layer budget bounds them.
const MAY_ACCEPT: &[&str] = &[
    "file over the size cap (test limit)",
    "data chunk over the decompression cap (test-style limit)",
    "unsupported method on an ignored entry",
    "component containing an instance of itself",
    "nested instances expand exponentially",
    "a Kiwi field named __proto__",
];

#[test]
fn scrims_hostile_inputs_are_refused_or_bounded_never_a_panic() {
    let mut checked = 0;
    for c in cases() {
        let Some(b64) = c["inputBase64"].as_str() else { continue };
        let name = c["name"].as_str().unwrap();
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
        let start = Instant::now();
        let r = std::panic::catch_unwind(|| import(&bytes, "hostile"));
        assert!(start.elapsed() < Duration::from_secs(30), "{name}: took {:?}", start.elapsed());
        let r = r.unwrap_or_else(|_| panic!("{name}: the importer panicked"));
        match r {
            Err(e) => assert!(!e.to_string().is_empty(), "{name}"),
            Ok((doc, _)) => {
                assert!(MAY_ACCEPT.contains(&name), "{name}: accepted ({} layers), but Scrim refuses it", doc.nodes().count());
                assert!(doc.nodes().count() <= layer_budget(bytes.len()) + 16, "{name}: past the layer budget");
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 56, "every case Scrim's file carries the bytes of");
}

#[test]
fn bytes_after_the_message_and_a_message_without_layers_are_refused() {
    let named = |n: &str| cases().into_iter().find(|c| c["name"] == n).unwrap();
    for (n, want) in [("bytes after the message", "bytes after the message"), ("message without nodeChanges", "no layers")] {
        let bytes = base64::engine::general_purpose::STANDARD.decode(named(n)["inputBase64"].as_str().unwrap()).unwrap();
        let e = import(&bytes, "x").unwrap_err().to_string();
        assert!(e.contains(want), "{n}: {e}");
    }
}

/// A frame chain `depth` deep under the first page.
fn nested(depth: u64) -> Vec<u8> {
    let mut nodes = vec![node(0, None, "DOCUMENT", "Doc", vec![]), node(1, Some((0, "!")), "CANVAS", "Page", vec![])];
    for i in 0..depth {
        nodes.push(node(10 + i, Some((if i == 0 { 1 } else { 9 + i }, "!")), "FRAME", &format!("F{i}"), vec![("size", xy(10.0, 10.0))]));
    }
    canvas(&schema(), &o(vec![("nodeChanges", arr(nodes))]))
}

#[test]
fn deep_nesting_never_overflows_the_stack() {
    for depth in [3_000, 20_000] {
        // The import runs on a thread with an ordinary stack: a recursion that grows with the
        // nesting would overflow it and abort the process rather than fail this test.
        let r = std::thread::Builder::new()
            .stack_size(8 << 20)
            .spawn(move || import(&nested(depth), "deep").map(|(d, w)| (d.nodes().count(), w)))
            .unwrap()
            .join();
        let r = r.unwrap_or_else(|_| panic!("{depth}: panicked"));
        match r {
            Ok((layers, warnings)) => assert!(layers > 0, "{depth}: {warnings:?}"),
            Err(e) => assert!(!e.to_string().is_empty()),
        }
    }
}

#[test]
fn a_hundred_thousand_layers_import_within_the_budget() {
    let mut nodes = vec![
        node(0, None, "DOCUMENT", "Doc", vec![]),
        node(1, Some((0, "!")), "CANVAS", "Page", vec![]),
        node(2, Some((1, "!")), "FRAME", "Board", vec![]),
    ];
    for i in 0..100_000u64 {
        nodes.push(node(10 + i, Some((2, "!")), "RECTANGLE", "R", vec![("size", xy(1.0, 1.0))]));
    }
    let bytes = canvas(&schema(), &o(vec![("nodeChanges", arr(nodes))]));
    let start = Instant::now();
    let r = import(&bytes, "many");
    assert!(start.elapsed() < Duration::from_secs(60), "took {:?}", start.elapsed());
    if let Ok((doc, _)) = r {
        assert!(doc.nodes().count() <= layer_budget(bytes.len()) + 16);
    }
}

/// With correct checksums (Scrim's hostile zips have CRC 0, which this reader refuses first), an
/// archive with an unsafe entry name is refused whatever the entry is, and an honest one opens.
#[test]
fn unsafe_entry_names_are_refused_before_anything_is_read() {
    use std::io::Write;
    let archive = |extra: &str| {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, bytes) in [("canvas.fig", nested(1)), (extra, b"x".to_vec())] {
            z.start_file(name, stored).unwrap();
            z.write_all(&bytes).unwrap();
        }
        z.finish().unwrap().into_inner()
    };
    let long = "a".repeat(513);
    for bad in ["../evil.fig", "/etc/passwd", "images/../../x", "images\\..\\x", "C:/x", "a/./b", "nul\0byte", long.as_str()] {
        let e = import(&archive(bad), "x").unwrap_err().to_string();
        assert!(e.contains("unsafe path"), "{bad:?}: {e}");
    }
    assert!(import(&archive("images/0123abcd"), "x").is_ok());
    assert!(import(&archive("thumbnail.png"), "x").is_ok());
}
