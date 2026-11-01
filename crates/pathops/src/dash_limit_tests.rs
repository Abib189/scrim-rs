//! A dash pattern too fine for its path strokes solid (upstream FigCraft 494387a6 and 2baeff34,
//! audit #1060 RND-1): kurbo makes one outline per dash, so a crafted pattern made millions of path
//! elements or ran out of memory.

use figcraft_doc::{StrokeAlign, StrokeProps};
use figcraft_geom::kurbo::{BezPath, Shape};

use crate::stroke_center;

#[test]
fn a_dash_pattern_too_fine_for_its_path_strokes_solid() {
    // About 500,000 dashes along this line: drawn solid, as a variable width stroke is past its
    // dash limit, rather than one outline per dash.
    let mut line = BezPath::new();
    line.move_to((0.0, 0.0));
    line.line_to((10_000.0, 0.0));
    let props = StrokeProps { weight: 2.0, dashes: vec![0.01, 0.01], align: StrokeAlign::Center, ..Default::default() };
    let s = stroke_center(&line, 2.0, &props);
    assert!(s.elements().len() < 100, "{} elements", s.elements().len());
    assert!((s.area().abs() - 20_000.0).abs() < 10.0, "{}", s.area());
}

#[test]
fn each_subpath_restarting_the_dash_pattern_counts_toward_the_limit() {
    // The pattern starts again at every subpath: on 10,000 short lines its 199 tiny dashes come
    // round each time (about a million dashes), though the length alone suggests a few hundred.
    let mut path = BezPath::new();
    for i in 0..10_000 {
        path.move_to((0.0, f64::from(i) * 4.0));
        path.line_to((2.0, f64::from(i) * 4.0));
    }
    let mut dashes = vec![0.01; 199];
    dashes.push(10_000.0);
    let props = StrokeProps { weight: 1.0, dashes, align: StrokeAlign::Center, ..Default::default() };
    let s = stroke_center(&path, 1.0, &props);
    assert!(s.elements().len() < 100_000, "{} elements", s.elements().len());
}

#[test]
fn an_ordinary_dash_pattern_still_dashes() {
    let mut line = BezPath::new();
    line.move_to((0.0, 0.0));
    line.line_to((100.0, 0.0));
    let props = StrokeProps { weight: 2.0, dashes: vec![10.0, 10.0], align: StrokeAlign::Center, ..Default::default() };
    let s = stroke_center(&line, 2.0, &props);
    assert!((s.area().abs() - 100.0).abs() < 1.0, "half the line is dashed: {}", s.area());
}
