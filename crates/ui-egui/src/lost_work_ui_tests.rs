//! Fix AI stage 2b through the window: what the lost-work fixes say to a person, in English and
//! Spanish.

use figcraft_engine::Session;
use serde_json::{Value, json};

use crate::layers_drag_tests::Ui;

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-ui-lost-work-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// LW-2: a file an earlier version saved with `null` numbers opens, and the window says how many
/// were reset; an edit that would make a number that isn't finite is refused with a reason.
#[test]
fn null_numbers_and_numbers_that_arent_finite_are_explained_in_english_and_spanish() {
    let d = dir("numbers");
    let mut s = Session::new();
    let r = figcraft_engine::params::id_of(&s.execute("shape.rectangle", &json!({"width": 10, "height": 10})).unwrap()["id"]).unwrap();
    let mut v = figcraft_format::to_json(s.doc().unwrap()).unwrap();
    v["nodes"][r.0.to_string()]["opacity"] = Value::Null;
    v["nodes"][r.0.to_string()]["width"] = Value::Null;
    let file = d.join("Old.scrim");
    std::fs::write(&file, v.to_string()).unwrap();
    let mut u = Ui::new(Session::empty());
    u.app.run("file.open", json!({"path": file.to_string_lossy()}));
    u.frame(vec![]);
    assert_eq!(
        u.app.ui.toast.as_ref().map(|t| t.0.as_str()),
        Some("Old: 2 numbers saved as null by an earlier version were reset to their defaults. The file itself wasn't changed.")
    );
    let refused = crate::windows::error_text(&figcraft_engine::EngineError::NotFinite { cmd: "node.set".into(), layer: "Title".into() });
    assert!(refused.starts_with("“Title” would get a number that isn't finite"), "{refused}");
    for key in [
        "{name}: {n} numbers saved as null by an earlier version were reset to their defaults. The file itself wasn't changed.",
        "“{layer}” would get a number that isn't finite (too large, or not a number), which a file can't hold, so the change wasn't made.",
    ] {
        assert!(crate::i18n::own_entry("en", key).is_some(), "{key} in English");
        assert!(crate::i18n::own_entry("es", key).is_some_and(|t| t.contains("null") || t.contains("finito")), "{key} in Spanish");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// LW-3: saving a file whose images past the budget weren't loaded says they were kept.
#[test]
fn a_save_that_kept_images_left_in_the_file_says_so_in_english_and_spanish() {
    let mut u = Ui::new(Session::new());
    u.app.session.request_ui("notice.imagesKept", json!({"name": "Big", "count": 2}));
    u.app.handle_ui_requests();
    assert_eq!(
        u.app.ui.toast.as_ref().map(|t| t.0.as_str()),
        Some(
            "Big was saved with every image: 2 that weren't loaded (together they pass what Scrim reads from one file) were copied over from the file."
        )
    );
    let key = "{name} was saved with every image: {n} that weren't loaded (together they pass what Scrim reads from one file) were copied over from the file.";
    assert!(crate::i18n::own_entry("es", key).is_some_and(|t| t.contains("imágenes")));
}
