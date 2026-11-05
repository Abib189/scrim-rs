//! Fix R-8: the README's "What Scrim opens and saves" table is made from the import registry
//! (`figcraft_engine::importers::REGISTRY`), one row per importer in its order. A new importer, or
//! a changed extension, fails this test until the README is brought up to date; the failure
//! prints the table to paste.

use figcraft_engine::importers::REGISTRY;

const START: &str = "<!-- formats: made from the import registry; xtask's readme_formats_tests checks it -->";
const END: &str = "<!-- formats end -->";

/// What Scrim does with an importer's files, by its id.
fn what(id: &str) -> &'static str {
    match id {
        "scrim" => {
            "Scrim's own documents: opened and saved. A `.figcraft` file (FigCraft's) opens too and is never written over: Save asks for a `.scrim` beside it"
        }
        "scrim-earlier" => "The earlier Scrim's JSON documents: imported as a new document",
        "fig" => "Figma files: imported as a new document (best effort)",
        "sketch" => "Sketch files: imported as a new document",
        "penpot" => "Penpot files: imported as a new document",
        "free" => "Lunacy files: imported as a new document",
        "rest-json" => "Figma REST API file JSON, with its local variables: imported as a new document",
        "svg" => "SVG: imported as editable vectors",
        "image" => "Images: opened as an image layer, or placed in a document",
        other => panic!("describe the new importer `{other}` here and in the README"),
    }
}

/// The table, as the README carries it between the markers.
fn table() -> String {
    let mut out = format!("{START}\n| Files | What Scrim does with them |\n|---|---|\n");
    for imp in REGISTRY {
        let exts: Vec<String> = imp.extensions().iter().map(|e| format!("`.{e}`")).collect();
        out.push_str(&format!("| {} | {} |\n", exts.join(", "), what(imp.id())));
    }
    out.push_str(END);
    out
}

#[test]
fn the_readme_lists_what_the_import_registry_opens() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the workspace");
    let readme = std::fs::read_to_string(root.join("README.md")).expect("README.md");
    let want = table();
    let have = readme.find(START).and_then(|a| readme[a..].find(END).map(|b| &readme[a..a + b + END.len()]));
    assert_eq!(have, Some(want.as_str()), "README.md's formats table is out of date; it should read:\n\n{want}\n");
}
