//! E2: the canvas label layout on rectangles. A label stays inside its frame's width (so Home —
//! Light's icon no longer lands on Trail — Light's title), a higher-priority label pushes a lower
//! one along or hides it, a turned label's box is respected, junk sizes hide a label, and over
//! hundreds of scattered labels no two placed labels overlap.

use egui::{Pos2, Rect, pos2};

use super::{Class, GAP, Item, Placed, ROW_BOTTOM, ROW_TOP, layout};
use crate::dev_icon::{ICON, ICON_GAP};

fn item(x: f32, y: f32, room: f32, title: f32, icon: bool, class: Class) -> Item {
    Item { corner: pos2(x, y), room, turned: None, class, depth: 0, lead: 0.0, title, icon }
}

fn placed(p: &[Option<Placed>], i: usize) -> Placed {
    p[i].unwrap_or_else(|| panic!("label {i} has no room: {p:?}"))
}

fn overlap(a: Rect, b: Rect) -> bool {
    a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
}

#[test]
fn ridgeline_at_22_percent_keeps_each_label_on_its_own_frame() {
    // Home — Light (86 px wide on screen, its title 70 px, Ready for dev), then Trail — Light 104 px
    // further right: before Stage E, Home's pill ran 70 px past its frame onto Trail's title.
    let p = layout(&[item(100.0, 300.0, 86.0, 70.0, true, Class::Other), item(204.0, 300.0, 86.0, 68.0, false, Class::Other)]);
    let (home, trail) = (placed(&p, 0), placed(&p, 1));
    assert!(home.rect.max.x <= 186.0 + 1e-3, "inside its frame: {home:?}");
    assert!(home.icon && home.title == 86.0 - ICON_GAP - ICON, "the title cut before the icon: {home:?}");
    assert_eq!(trail.offset, 0.0);
    assert_eq!(trail.title, 68.0, "Trail's title whole");
    assert!(!overlap(home.rect, trail.rect));
}

#[test]
fn a_higher_priority_label_pushes_a_lower_one_along_or_hides_it() {
    // A section's title and a child's label in the same row at the same corner (Landscape in
    // Components, zoomed out): the section keeps its place, the child moves right past it.
    let p = layout(&[item(0.0, 100.0, 300.0, 40.0, false, Class::Other), item(0.0, 104.0, 600.0, 80.0, true, Class::Section)]);
    let (child, section) = (placed(&p, 0), placed(&p, 1));
    assert_eq!(section.offset, 0.0);
    assert!(child.rect.min.x >= section.rect.max.x + GAP - 1e-3, "{child:?} after {section:?}");
    // With no room left in its frame, the child's label hides instead.
    let p = layout(&[item(0.0, 100.0, 60.0, 40.0, false, Class::Other), item(0.0, 104.0, 600.0, 80.0, true, Class::Section)]);
    assert!(p[0].is_none() && p[1].is_some(), "{p:?}");
    // Selected comes before hovered, hovered before the rest; outer sections before inner ones.
    let mut inner = item(0.0, 0.0, 200.0, 50.0, false, Class::Section);
    inner.depth = 1;
    let p = layout(&[item(0.0, 0.0, 200.0, 50.0, false, Class::Other), inner, item(0.0, 0.0, 200.0, 50.0, false, Class::Section)]);
    assert_eq!(placed(&p, 2).offset, 0.0, "the outer section first");
    assert!(placed(&p, 1).offset > 0.0 && placed(&p, 0).offset > placed(&p, 1).offset, "{p:?}");
}

#[test]
fn a_label_further_right_ends_the_room_before_it() {
    // B (selected, placed first) starts 40 px into A's room: A's title ends a gap before it.
    let p = layout(&[item(0.0, 0.0, 300.0, 200.0, true, Class::Other), item(40.0, 0.0, 300.0, 50.0, false, Class::Selected)]);
    let (a, b) = (placed(&p, 0), placed(&p, 1));
    assert!(a.rect.max.x <= b.rect.min.x - GAP + 1e-3, "{a:?} {b:?}");
    assert!(!a.icon && a.title == 40.0 - GAP, "32 px holds a cut title, not the icon too: {a:?}");
}

#[test]
fn a_turned_label_keeps_its_place_and_its_box_is_taken() {
    let mut turned = item(0.0, 0.0, 200.0, 50.0, true, Class::Turned);
    turned.turned = Some(Rect::from_min_max(pos2(-10.0, -150.0), pos2(120.0, 20.0)));
    let p = layout(&[item(50.0, 0.0, 300.0, 60.0, false, Class::Other), turned]);
    assert_eq!(placed(&p, 1).rect, turned.turned.unwrap());
    assert!(!overlap(placed(&p, 0).rect, placed(&p, 1).rect), "{p:?}");
}

#[test]
fn junk_sizes_hide_a_label() {
    for (x, room, title) in [(f32::NAN, 100.0, 10.0), (0.0, f32::INFINITY, 10.0), (0.0, -50.0, 10.0), (0.0, 100.0, f32::NAN), (0.0, 0.0, 10.0)] {
        let p = layout(&[item(x, 0.0, room, title, true, Class::Other)]);
        assert!(p[0].is_none(), "{x} {room} {title}: {p:?}");
    }
}

#[test]
fn hundreds_of_scattered_labels_never_overlap() {
    // A deterministic scatter: frames in a few rows, some overlapping, some narrow, every class.
    let mut seed = 0x2545_F491_u64;
    let mut next = |m: u64| {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (seed >> 33) % m
    };
    let classes = [Class::Section, Class::Selected, Class::Hovered, Class::Other, Class::Other, Class::Other];
    let items: Vec<Item> = (0..400)
        .map(|_| {
            let mut it = item(
                next(2000) as f32,
                (next(12) * 9) as f32,
                (5 + next(300)) as f32,
                (next(240)) as f32,
                next(2) == 0,
                classes[next(classes.len() as u64) as usize],
            );
            it.lead = if next(4) == 0 { 14.0 } else { 0.0 };
            it.depth = next(3) as u32;
            it
        })
        .collect();
    let p = layout(&items);
    let shown: Vec<(usize, Placed)> = p.iter().enumerate().filter_map(|(i, p)| Some((i, (*p)?))).collect();
    assert!(shown.len() > 100, "most labels still show: {}", shown.len());
    for (k, (i, a)) in shown.iter().enumerate() {
        let it = &items[*i];
        // Inside its own row and room.
        assert!(a.rect.min.x >= it.corner.x - 1e-3 && a.rect.max.x <= it.corner.x + it.room + 1e-3, "{i}: {a:?} {it:?}");
        assert!((a.rect.min.y - (it.corner.y - ROW_TOP)).abs() < 1e-3 && (a.rect.max.y - (it.corner.y - ROW_BOTTOM)).abs() < 1e-3);
        for (j, b) in shown.iter().skip(k + 1) {
            assert!(!overlap(a.rect, b.rect), "{i} {a:?} overlaps {j} {b:?}");
        }
    }
    let _ = Pos2::ZERO;
}
