//! Port 8: Scrim's path-data cases (`docs/scrim-transfer/cases/svg.json` › `pathEdit`) against the
//! parser behind `node.create {type: vector, path}` (`cmd::create::parse_path_data`, kurbo's SVG
//! path parser). Every path Scrim reads, this reads to the same shape; the malformed ones are
//! refused here as in Scrim, and nothing panics.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use figcraft_engine::cmd::create::parse_path_data;
use figcraft_geom::Shape;
use serde_json::Value;

fn cases() -> Vec<Value> {
    let v: Value = serde_json::from_str(include_str!("../../../docs/scrim-transfer/cases/svg.json")).unwrap();
    v["cases"].as_array().unwrap().iter().filter(|c| c["layer"] == "pathEdit").cloned().collect()
}

#[test]
fn scrims_path_cases_parse_alike() {
    let (mut read, mut refused) = (0, 0);
    let mut lenient = Vec::new();
    for c in cases() {
        let d = c["svg"].as_str().unwrap();
        let got = std::panic::catch_unwind(|| parse_path_data(d)).unwrap_or_else(|_| panic!("{d:?} panicked"));
        match c["expect"].get("normalised").and_then(Value::as_str) {
            Some(norm) => {
                read += 1;
                let ours = got.unwrap_or_else(|| panic!("{d:?}: Scrim reads it, this doesn't"));
                // The same shape: Scrim's normalised path (arcs as curves, rounded to 0.01) has the
                // same bounds as this one, within that rounding.
                let theirs = parse_path_data(norm).unwrap();
                if theirs.segments().count() == 0 {
                    // A bare move (an arc to its own start): nothing to draw either way.
                    let a = ours.bounding_box();
                    assert!(a.width() == 0.0 && a.height() == 0.0, "{d:?}: {a:?}");
                    continue;
                }
                let (a, b) = (ours.bounding_box(), theirs.bounding_box());
                let close = |x: f64, y: f64| (x - y).abs() < 0.02;
                assert!(close(a.x0, b.x0) && close(a.y0, b.y0) && close(a.x1, b.x1) && close(a.y1, b.y1), "{d:?}: {a:?} vs Scrim {b:?}");
            }
            None => {
                refused += 1;
                if got.is_some() {
                    lenient.push(d.to_string());
                }
            }
        }
    }
    assert_eq!((read, refused), (11, 6));
    assert!(lenient.is_empty(), "Scrim refuses, this reads: {lenient:?}");
}
