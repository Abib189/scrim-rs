//! E1: the handoff status icon. Its colours read (3:1, non-text) on both themes' canvases and on
//! both default page colours, the states map to distinct icons, and a title shares its frame's
//! width with the icon: cut before it, the icon dropped first, then the whole label.

use egui::Color32;
use figcraft_doc::{DevStatus, Node};

use super::{Ink, State};
use crate::frame_labels::{MIN_TITLE, fit};
use crate::theme::accent::contrast;
use crate::theme::{Tokens, c32};

/// The backgrounds a canvas label sits on: each theme's canvas, and the two colours new pages get.
pub(crate) fn pages() -> [(&'static str, Color32); 4] {
    [
        ("light canvas", Tokens::light().canvas),
        ("dark canvas", Tokens::dark().canvas),
        ("default page", c32(figcraft_color::Color::DEFAULT_PAGE)),
        ("dark page", c32(figcraft_color::Color::DARK_PAGE)),
    ]
}

#[test]
fn every_status_icon_reads_on_both_canvases_and_both_default_pages() {
    let mut low = Vec::new();
    for (page, bg) in pages() {
        let ink = Ink::for_page(bg);
        for s in [State::Ready, State::Changed, State::Completed] {
            let r = contrast(ink.of(s), bg);
            if r < 3.0 {
                low.push(format!("{s:?} on the {page}: {r:.2}:1"));
            }
        }
    }
    assert!(low.is_empty(), "{low:#?}");
}

#[test]
fn the_states_are_ready_changed_and_completed() {
    let mut n = Node::new(figcraft_doc::NodeId(1), figcraft_doc::NodeKind::Frame, "F", 100.0, 100.0);
    assert_eq!(State::of(&n), None);
    n.dev_status = DevStatus::ReadyForDev;
    assert_eq!(State::of(&n), Some(State::Ready));
    n.dev_changed = true;
    assert_eq!(State::of(&n), Some(State::Changed));
    n.dev_status = DevStatus::Completed;
    assert_eq!(State::of(&n), Some(State::Completed));
    let labels = [State::Ready, State::Changed, State::Completed].map(State::label);
    assert_eq!(labels, ["Ready for dev", "Changed", "Completed"]);
}

#[test]
fn a_title_is_cut_before_the_icon_and_the_icon_goes_first_then_the_label() {
    // Room for both: the whole title, then the icon (4 px gap, 14 px).
    assert_eq!(fit(200.0, 80.0, true), Some((80.0, true)));
    // A long title ends 18 px before the room does.
    assert_eq!(fit(100.0, 300.0, true), Some((82.0, true)));
    // Too narrow for both: the icon goes, the title keeps the room.
    assert_eq!(fit(30.0, 300.0, true), Some((30.0, false)));
    // Too narrow for the shortest title: nothing (no icon on its own).
    assert_eq!(fit(MIN_TITLE - 1.0, 300.0, true), None);
    // A title shorter than the minimum shows whole, with the icon when they fit.
    assert_eq!(fit(30.0, 8.0, true), Some((8.0, true)));
    // Junk sizes hide the label.
    assert_eq!(fit(f32::NAN, 10.0, true), None);
    assert_eq!(fit(-5.0, 10.0, false), None);
}
