//! Malformed and hostile files give warnings or an error, never a panic.

use super::*;

#[test]
fn malformed_values_never_panic() {
    let weird = vec![
        json!({"_class": "rectangle", "name": 5, "frame": {"x": "a", "y": null}}),
        json!({"_class": "oval", "frame": frame(f64::MAX, -1e300, 1e300, -5.0), "points": "nope", "style": 3}),
        json!({"_class": "text", "frame": frame(0.0, 0.0, 1.0, 1.0), "attributedString": {"string": "ab", "attributes": [
            {"location": -5, "length": 99_999_999, "attributes": {"MSAttributedStringFontAttribute": {"attributes": {"name": "", "size": -1, "variation": {"x": 1}}}}},
            {"location": "a"}]}}),
        json!({"_class": "shapePath", "frame": frame(0.0, 0.0, 1.0, 1.0), "points": [{"point": "{nan, 1}"}, {"point": "{1e999, 0}"}, {"point": "{{1,2}"}]}),
        json!({"_class": "group", "frame": frame(0.0, 0.0, 1.0, 1.0), "layers": [json!(null), json!(7), json!("x")]}),
        json!({"_class": "bitmap", "frame": frame(0.0, 0.0, 1.0, 1.0), "image": {"_class": "MSJSONFileReference", "_ref": "../../etc/passwd"}}),
        json!({"_class": "bitmap", "frame": frame(0.0, 0.0, 1.0, 1.0), "image": {"_class": "MSJSONOriginalDataReference", "data": {"_data": "!!!"}}}),
        json!({"_class": "symbolInstance", "frame": frame(0.0, 0.0, 1.0, 1.0), "symbolID": 12, "overrideValues": [{"overrideName": "x"}]}),
        json!({"_class": "rectangle", "frame": frame(0.0, 0.0, 1.0, 1.0), "style": {"fills": [{"fillType": 9}, {"fillType": 1, "gradient": {"stops": []}}],
            "borders": [{"thickness": -3}], "shadows": [{"blurRadius": "big"}], "blur": {"isEnabled": true, "type": 7}}}),
    ];
    let (doc, warnings) = one(weird);
    assert!(!warnings.is_empty());
    assert!(doc.validate().is_empty(), "{:?}", doc.validate());
}

#[test]
fn deep_nesting_is_cut_off() {
    let mut v = rectangle(1, "Leaf", frame(0.0, 0.0, 1.0, 1.0), [0.0; 4]);
    for i in 0..300 {
        v = container("group", 1000 + i, "G", frame(0.0, 0.0, 1.0, 1.0), vec![v]);
    }
    let (doc, warnings) = one(vec![v]);
    // JSON this deep is refused by the parser before the layer depth cap is reached: the page
    // is reported, the file still opens.
    assert!(
        warnings.iter().any(|w| w.severity == Severity::Dropped && (w.message.contains("nested deeper") || w.message.contains("couldn't be read")))
    );
    assert!(doc.nodes().all(|n| n.name != "Leaf"));
}

#[test]
fn bad_page_references_and_pages_are_reported() {
    let doc = json!({"pages": [{"_class": "MSJSONFileReference", "_ref": "../evil"}, {"_class": "MSJSONFileReference", "_ref": "pages/missing"}]});
    let bytes = sketch(&[], doc, &[]);
    let (d, w) = import(&bytes, "Bad").unwrap();
    assert!(has(&w, Severity::Dropped, "", "valid file name"));
    assert!(has(&w, Severity::Dropped, "", "pages/missing"));
    assert_eq!(d.pages.len(), 1);
    // A page member that isn't JSON is a warning, not an error.
    let members = vec![
        (
            "document.json".to_string(),
            serde_json::to_vec(&json!({"_class": "document", "pages": [{"_class": "MSJSONFileReference", "_ref": "pages/p"}]})).unwrap(),
        ),
        ("meta.json".to_string(), b"{\"version\": 146}".to_vec()),
        ("pages/p.json".to_string(), b"{not json".to_vec()),
    ];
    let (_, w) = import(&zip(&members), "Broken page").unwrap();
    assert!(has(&w, Severity::Dropped, "", "couldn't be read"));
}

#[test]
fn a_cyclic_pair_of_symbols_terminates() {
    let a = master(10, 1, "A", frame(0.0, 0.0, 10.0, 10.0), vec![instance(11, 2, "B in A", frame(0.0, 0.0, 10.0, 10.0), vec![])]);
    let b = master(20, 2, "B", frame(0.0, 0.0, 10.0, 10.0), vec![instance(21, 1, "A in B", frame(0.0, 0.0, 10.0, 10.0), vec![])]);
    let (doc, warnings) = one(vec![a, b]);
    assert!(warnings.iter().any(|w| w.message.contains("instance of itself") || w.message.contains("inside its own symbol")));
    assert!(doc.validate().is_empty());
}
