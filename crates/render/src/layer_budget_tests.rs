//! Nested isolated layers share one memory budget (upstream FigCraft 96eed159, audit #1061 RND-2):
//! a crafted nest of translucent groups holds a pixmap per level while the inner ones draw, so
//! past the budget the innermost are left out instead of the render asking for more memory than
//! there is.

use figcraft_doc::NodeKind;

use super::layer_budget;
use super::paint_behaviour_tests::{page, px, rect, render, solid_at};

/// `levels` translucent groups, each inside the last, with a red square at the bottom.
fn nest(levels: usize) -> figcraft_doc::Document {
    let (mut d, p) = page();
    let mut parent = p;
    for _ in 0..levels {
        parent = d.create(parent, NodeKind::Group, "G", 0.0, 0.0, 200.0, 200.0).unwrap();
        d.get_mut(parent).unwrap().opacity = 0.99;
    }
    rect(&mut d, parent, (50.0, 50.0, 100.0, 100.0), vec![solid_at("#FF0000", 1.0)]);
    d
}

#[test]
fn nested_layers_stay_within_their_budget() {
    // Each level's layer is about 200 × 200 px: room for four of them, then the rest are left out.
    let one = 202 * 202 * 4;
    let d = nest(12);
    let (pix, peak) = layer_budget::within(4 * one, || render(&d));
    assert!(peak <= 4 * one, "held {peak} bytes at once, over the {} budget", 4 * one);
    assert_eq!(px(&pix, 100, 100), [255, 255, 255], "the innermost layers are left out");
    // Within the budget the same nest draws.
    let shallow = nest(3);
    let (pix, peak) = layer_budget::within(4 * one, || render(&shallow));
    assert!(peak <= 4 * one);
    assert!(px(&pix, 100, 100)[0] > 200 && px(&pix, 100, 100)[1] < 40, "{:?}", px(&pix, 100, 100));
}
