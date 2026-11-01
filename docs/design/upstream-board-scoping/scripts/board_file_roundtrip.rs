//! Run as the main.rs of a scratch binary crate outside the repo, with path dependencies on
//! `/path/to/scrim-rs/crates/format` and `/path/to/scrim-rs/crates/doc`, plus `serde_json = "1"` and
//! `zip = { version = "2", default-features = false, features = ["deflate"] }`; copy Scrim's
//! Cargo.lock beside it and `cargo run --offline`. The output is ../board-file-roundtrip-output.txt.
//!
//! What Scrim (b790dea1) does with a file upstream FigCraft saved as a board: the board marks
//! upstream 8167f13a adds are `kind: "board"` on the document and `board: {...}` on a node.
use std::io::{Cursor, Read, Write};

fn members(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..z.len()).map(|i| { let mut f = z.by_index(i).unwrap(); let mut v = Vec::new(); f.read_to_end(&mut v).unwrap(); (f.name().to_string(), v) }).collect()
}
fn zipped(m: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    let mut w = zip::ZipWriter::new(&mut buf);
    for (n, d) in m { w.start_file(n.as_str(), zip::write::SimpleFileOptions::default()).unwrap(); w.write_all(d).unwrap(); }
    w.finish().unwrap();
    buf.into_inner()
}
fn json_of(bytes: &[u8]) -> serde_json::Value {
    let m = members(bytes);
    serde_json::from_slice(&m.iter().find(|(n, _)| n == "document.json").unwrap().1).unwrap()
}

fn main() {
    let mut d = figcraft_doc::Document::new("Board");
    let p = d.first_page().unwrap();
    let sticky = d.create(p, figcraft_doc::NodeKind::Frame, "Sticky", 0.0, 0.0, 200.0, 200.0).unwrap();
    let saved = figcraft_format::save_bytes(&d, None).unwrap();
    // Make it the file upstream would write for a board with one sticky note.
    let mut m = members(&saved);
    let doc = m.iter_mut().find(|(n, _)| n == "document.json").unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&doc.1).unwrap();
    let root = if v.get("document").is_some() { v.get_mut("document").unwrap() } else { &mut v };
    root["kind"] = "board".into();
    let id = serde_json::to_value(sticky).unwrap();
    let mut marked = false;
    fn mark(x: &mut serde_json::Value, id: &serde_json::Value, marked: &mut bool) {
        match x {
            serde_json::Value::Object(o) => {
                if o.get("id") == Some(id) && o.contains_key("type") { o.insert("board".into(), serde_json::json!({"type": "sticky", "author": "Ana"})); *marked = true; return; }
                for (_, c) in o.iter_mut() { mark(c, id, marked); }
            }
            serde_json::Value::Array(a) => for c in a { mark(c, id, marked); },
            _ => {}
        }
    }
    mark(root, &id, &mut marked);
    println!("nodes sample: {}", serde_json::to_string(&v["nodes"]).unwrap().chars().take(400).collect::<String>());
    println!("top-level keys: {:?}", v.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()));
    println!("sticky marked in JSON: {marked}");
    doc.1 = serde_json::to_vec(&v).unwrap();
    let board = zipped(&m);
    let s = String::from_utf8_lossy(&m.iter().find(|(n, _)| n == "document.json").unwrap().1).to_string();
    println!("board file has kind=board: {}, board item: {}", s.contains("\"kind\":\"board\""), s.contains("\"board\":{"));
    match figcraft_format::load_bytes(&board) {
        Ok((back, issues)) => {
            println!("Scrim opens it: yes; layers {}; issues {:?}", back.node_count(), issues);
            let again = figcraft_format::save_bytes(&back, None).unwrap();
            let s2 = serde_json::to_string(&json_of(&again)).unwrap();
            println!("after Scrim saves it: kind=board kept: {}; board item kept: {}", s2.contains("\"kind\":\"board\""), s2.contains("\"board\":{"));
        }
        Err(e) => println!("Scrim refuses it: {e}"),
    }
}
