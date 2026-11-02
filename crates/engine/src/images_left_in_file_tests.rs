//! Fix AI LW-3: a save never drops an image the document holds. A file whose images together pass
//! what Scrim reads from one file (4 GiB or 20,000 images) opens without the ones past that budget;
//! they stay in the file, and saving (over it, or as a copy) copies them across instead of
//! deleting them, and says so.

use base64::Engine as _;
use serde_json::json;

use crate::Session;

fn png(k: u8) -> String {
    let mut pix = figcraft_render::Pixmap::new(2, 2).unwrap();
    for px in pix.data_mut().chunks_mut(4) {
        px.copy_from_slice(&[k, 40, 90, 255]);
    }
    base64::engine::general_purpose::STANDARD.encode(figcraft_render::encode_png(&pix).unwrap())
}

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("figcraft-engine-images-left-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The images a saved file holds, read without any budget.
fn images_in(path: &std::path::Path) -> usize {
    figcraft_format::load_bytes_with_versions(&std::fs::read(path).unwrap()).unwrap().doc.images.len()
}

#[test]
fn images_past_the_budget_are_kept_when_the_file_is_saved_over_or_as_a_copy() {
    let d = dir("kept");
    let file = d.join("Big.scrim");
    let mut s = Session::new();
    for k in 0..3 {
        s.execute("image.place", &json!({"data": png(k * 60), "x": f64::from(k) * 20.0, "y": 0, "width": 10, "height": 10})).unwrap();
    }
    s.execute("file.save", &json!({"path": file.to_string_lossy()})).unwrap();
    assert_eq!(images_in(&file), 3);
    // Read with room for one image: two stay in the file.
    let copy = d.join("Copy.scrim");
    figcraft_format::with_image_budget(1, u64::MAX, || {
        let mut s = Session::empty();
        s.execute("file.open", &json!({"path": file.to_string_lossy()})).unwrap();
        assert_eq!(s.doc().unwrap().images.len(), 1, "one image loaded");
        s.execute("shape.rectangle", &json!({"x": 0, "y": 40, "width": 5, "height": 5})).unwrap();
        s.execute("file.save", &json!({})).expect("saves over the file");
        assert!(
            s.ui_requests.iter().any(|r| r.kind == "notice.imagesKept" && r.data["count"] == 2),
            "the save says the images were kept: {:?}",
            s.ui_requests.iter().map(|r| &r.kind).collect::<Vec<_>>()
        );
        s.execute("file.saveAs", &json!({"path": copy.to_string_lossy()})).expect("saves a copy");
        // Saved again from the copy: they come along still.
        s.execute("shape.rectangle", &json!({"x": 10, "y": 40, "width": 5, "height": 5})).unwrap();
        s.execute("file.save", &json!({})).expect("saves the copy again");
    });
    assert_eq!(images_in(&file), 3, "saving over the file kept every image");
    assert_eq!(images_in(&copy), 3, "the copy holds every image");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_save_that_cant_copy_the_images_left_in_the_file_is_refused_and_writes_nothing() {
    let d = dir("gone");
    let file = d.join("Big.scrim");
    let mut s = Session::new();
    for k in 0..2 {
        s.execute("image.place", &json!({"data": png(k * 90 + 1), "x": 0, "y": 0, "width": 10, "height": 10})).unwrap();
    }
    s.execute("file.save", &json!({"path": file.to_string_lossy()})).unwrap();
    figcraft_format::with_image_budget(1, u64::MAX, || {
        let mut s = Session::empty();
        s.execute("file.open", &json!({"path": file.to_string_lossy()})).unwrap();
        // The file the images are in is gone: a save would lose them.
        let moved = d.join("Moved.scrim");
        std::fs::rename(&file, &moved).unwrap();
        let copy = d.join("Copy.scrim");
        let r = s.execute("file.saveAs", &json!({"path": copy.to_string_lossy()}));
        assert!(r.is_err(), "refused");
        assert!(!copy.exists(), "nothing written");
        std::fs::rename(&moved, &file).unwrap();
    });
    let _ = std::fs::remove_dir_all(&d);
}
