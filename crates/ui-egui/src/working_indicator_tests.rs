//! I3, the working indicator: the wave is deterministic for a seed and a time, smooth and periodic;
//! it fades in over 160 ms and out over 400 ms when the working state ends (Stage I.1); the row is placed after E2's
//! labels and never overlaps one (Ridgeline at 10, 22, 50 and 100%), drops columns on a narrow
//! artboard, then hides, and is never wider than its artboard; the outline has no handles and no
//! hit area and gives way to the person's selection; one polite AccessKit status names it; the
//! static row with Animations Off; 3:1 at its brightest in both themes; the switches are saved.

use egui::{Pos2, Rect, Shape, pos2};
use figcraft_doc::NodeId;
use figcraft_engine::Session;
use serde_json::json;

use super::{COLUMNS, FADE_IN, FADE_OUT, LOOP, MIN_COLUMNS, column_x, place, still, wave, width};
use crate::follow_agent::tests::{agent, click, frames, id, poster, scene};
use crate::layers_drag_tests::Ui;

fn calm() -> (Ui, NodeId) {
    let (mut u, home) = scene();
    u.app.ui.agent_view.budget.frozen = true;
    (u, home)
}

#[test]
fn the_wave_is_deterministic_smooth_and_loops() {
    for (seed, row, col, t) in [(7, 0, 0, 0.0), (7, 1, 13, 0.73), (99, 0, 19, 5.2)] {
        let v = wave(seed, row, col, t);
        assert_eq!(v, wave(seed, row, col, t), "the same seed and time, the same value");
        assert!((0.0..=1.0).contains(&v));
        assert!((v - wave(seed, row, col, t + LOOP)).abs() < 1e-9, "a {LOOP} s loop");
        // Smooth: one frame moves it at most a little (no popping).
        assert!((v - wave(seed, row, col, t + 1.0 / 60.0)).abs() < 0.07);
    }
    // Columns differ (it reads organic), and another artboard's seed differs too.
    let cols: Vec<f64> = (0..COLUMNS).map(|c| wave(7, 0, c, 0.4)).collect();
    assert!(cols.windows(2).any(|w| (w[0] - w[1]).abs() > 0.01));
    assert_ne!(wave(7, 0, 3, 0.4), wave(8, 0, 3, 0.4));
    // The static row is one fixed moment of the wave.
    assert_eq!(still(7, 1, 4), still(7, 1, 4));
}

#[test]
fn it_fades_in_over_160_ms_stays_across_gaps_and_fades_out_over_400_ms_when_work_ends() {
    let mut w = crate::follow_agent::working_state::State::default();
    w.touch("session:a".into(), Some("Codex".into()), 10.0);
    w.tick(10.0, 15.0, true);
    assert_eq!(w.alpha(10.0, true), 0.0);
    assert!((w.alpha(10.0 + FADE_IN / 2.0, true) - 0.5).abs() < 1e-9);
    assert_eq!(w.alpha(10.0 + FADE_IN, true), 1.0);
    // A 2.5 s gap between steps: still fully on.
    w.tick(12.5, 15.0, true);
    assert_eq!(w.alpha(12.5, true), 1.0);
    // It finishes: out over 400 ms from then.
    w.finish("session:a", 13.0);
    w.tick(13.0, 15.0, true);
    assert!((w.alpha(13.0 + FADE_OUT / 2.0, true) - 0.5).abs() < 1e-9);
    assert_eq!(w.alpha(13.0 + FADE_OUT, true), 0.0);
    // Without motion: shown for the whole run, then gone, no fades.
    let mut w = crate::follow_agent::working_state::State::default();
    w.touch("session:a".into(), None, 10.0);
    w.tick(10.0, 15.0, false);
    assert_eq!(w.alpha(10.0, false), 1.0);
    w.tick(24.9, 15.0, false);
    assert_eq!(w.alpha(24.9, false), 1.0);
    w.tick(25.0, 15.0, false);
    assert_eq!(w.alpha(25.0, false), 0.0);
}

#[test]
fn the_row_drops_columns_on_a_narrow_artboard_then_hides_and_never_touches_a_label() {
    let corner = pos2(500.0, 300.0);
    // Room for everything.
    let (cols, r) = place(corner, 400.0, &[]).unwrap();
    assert_eq!(cols, COLUMNS);
    assert_eq!(r.max.x, corner.x, "right-aligned to the artboard's edge");
    assert!((corner.y - r.max.y - super::ABOVE).abs() < 1e-3, "12 points above it");
    assert!((r.width() - width(COLUMNS)).abs() < 1e-3);
    // Narrow: fewer columns, never wider than the artboard.
    let (cols, r) = place(corner, 60.0, &[]).unwrap();
    assert!((MIN_COLUMNS..COLUMNS).contains(&cols) && r.width() <= 60.0, "{cols}");
    // Too narrow even for one group: hidden.
    assert!(place(corner, width(MIN_COLUMNS) - 1.0, &[]).is_none());
    // A label in the way: columns dropped from the left to clear it by the gap, or hidden.
    let label = Rect::from_min_max(pos2(300.0, 282.0), pos2(420.0, 298.0));
    let (cols, r) = place(corner, 400.0, &[label]).unwrap();
    assert!(cols < COLUMNS && !label.expand(crate::canvas_labels::GAP).intersects(r), "{cols} {r:?}");
    // A label across the whole width: one row higher, above the labels, still right-aligned.
    let wide = Rect::from_min_max(pos2(100.0, 282.0), pos2(499.0, 298.0));
    let (cols, r) = place(corner, 400.0, &[wide]).unwrap();
    assert_eq!(cols, COLUMNS);
    assert!(r.max.y <= wide.min.y - crate::canvas_labels::GAP + 1e-3 && r.max.x == corner.x, "{r:?}");
    // Both places taken: hidden.
    let above = Rect::from_min_max(pos2(100.0, 250.0), pos2(499.0, 281.0));
    assert!(place(corner, 400.0, &[wide, above]).is_none(), "no room left: hidden");
    // Columns keep their places as others drop (counted from the right).
    assert!(column_x(4) < column_x(5) - super::DOT - super::SPACE);
}

/// Ridgeline: an agent works inside "Home — Light" at 10%, 22%, 50% and 100%.
#[test]
fn on_ridgeline_at_four_zooms_the_row_never_overlaps_a_label_and_stays_on_its_artboard() {
    let mut s = Session::new();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ridgeline-app.figcraft");
    s.execute("file.open", &json!({"path": path})).unwrap();
    s.execute("select.none", &json!({})).unwrap();
    let home = id(&s.execute("node.query", &json!({"query": "//FRAME[@name='Home — Light']", "limit": 1})).unwrap()["results"][0]);
    let mut u = Ui::new(s);
    u.app.ui.agent_view.budget.frozen = true;
    u.app.run("camera.follow-agents", json!({"on": false}));
    for zoom in [0.10, 0.22, 0.50, 1.00] {
        let ds = u.app.session.ds_mut().unwrap();
        (ds.view.cx, ds.view.cy, ds.view.zoom) = (196.0, 300.0, zoom);
        frames(&mut u, 2);
        agent(&mut u, "shape.rectangle", json!({"x": 20, "y": 20, "width": 10, "height": 10, "parent": home}));
        frames(&mut u, 3);
        let b = u.app.session.doc().unwrap().world_bounds(home).unwrap();
        let art = Rect::from_two_pos(u.app.to_screen(b.x0, b.y0), u.app.to_screen(b.x1, b.y1));
        let labels: Vec<Rect> = u.app.session.tool.labels.iter().map(|(_, q)| Rect::from_points(&q.map(|p| u.app.to_screen(p.x, p.y)))).collect();
        assert!(!labels.is_empty(), "{zoom}: the labels are there");
        match u.app.ui.agent_view.indicator.drawn.get() {
            Some((r, cols)) => {
                for l in &labels {
                    assert!(!l.intersects(r), "{zoom}: the row ({cols} columns, {r:?}) overlaps a label {l:?}");
                }
                assert!(r.min.x >= art.min.x - 0.5 && r.max.x <= art.max.x + 0.5, "{zoom}: within its artboard's width");
                assert!(r.max.y < art.min.y, "{zoom}: above it");
            }
            None => assert!(width(MIN_COLUMNS) > art.width(), "{zoom}: hidden only when the artboard is too narrow ({art:?})"),
        }
        assert!(u.app.ui.agent_view.indicator.outlined.get().is_some(), "{zoom}: the outline");
    }
}

#[test]
fn the_outline_has_no_handles_and_no_hit_area_and_the_persons_selection_wins() {
    let (mut u, _) = calm();
    let p = poster(&mut u, 300.0);
    // (An agent's new frame becomes the selection, whose own outline wins: clear it.)
    u.app.run("select.none", json!({}));
    frames(&mut u, 20);
    let r = u.app.ui.agent_view.indicator.outlined.get().expect("outlined");
    assert!(u.app.session.selection().is_empty(), "the outline doesn't select");
    // A press just outside the frame, on the outline: empty canvas, nothing selected.
    click(&mut u, pos2(r.max.x + 1.0, r.center().y));
    assert!(u.app.session.selection().is_empty(), "no hit area of its own");
    // The person selects the artboard while the agent works on it: their outline and handles,
    // not this one.
    agent(&mut u, "shape.rectangle", json!({"x": 10, "y": 10, "width": 10, "height": 10, "parent": p}));
    u.app.run("select.set", json!({"ids": [p]}));
    frames(&mut u, 2);
    assert!(u.app.ui.agent_view.indicator.outlined.get().is_none(), "the selection wins");
    assert!(u.app.ui.agent_view.indicator.drawn.get().is_some(), "the dots stay");
}

#[test]
fn one_polite_status_names_the_agent_and_the_artboard() {
    let (mut u, _) = calm();
    u.ctx.enable_accesskit();
    poster(&mut u, 300.0);
    frames(&mut u, 3);
    let out = u.frame_output(vec![]);
    let nodes: Vec<egui::accesskit::Node> = out.platform_output.accesskit_update.expect("a tree").nodes.into_iter().map(|(_, n)| n).collect();
    let status: Vec<&egui::accesskit::Node> = nodes.iter().filter(|n| n.label() == Some("Claude Code is working on Poster")).collect();
    assert_eq!(status.len(), 1, "one node, not one per dot");
    assert_eq!(status[0].role(), egui::accesskit::Role::Status);
    assert_eq!(status[0].live(), Some(egui::accesskit::Live::Polite));
}

/// The indicator's dots this frame: circles inside its row (where, how big, what colour).
fn dots(u: &mut Ui) -> Vec<(Pos2, f32, egui::Color32)> {
    let out = u.frame_output(vec![]);
    let Some((row, _)) = u.app.ui.agent_view.indicator.drawn.get() else { return Vec::new() };
    out.shapes
        .iter()
        .filter_map(|c| match &c.shape {
            Shape::Circle(c) if row.expand(1.0).contains(c.center) => Some((c.center, c.radius, c.fill)),
            _ => None,
        })
        .collect()
}

#[test]
fn with_animations_off_the_row_is_still_and_with_motion_it_moves() {
    // (The camera stays put: only the wave may change the dots.)
    let (mut u, _) = calm();
    u.app.run("camera.follow-agents", json!({"on": false}));
    poster(&mut u, 300.0);
    frames(&mut u, 20);
    let looks = |d: Vec<(Pos2, f32, egui::Color32)>| d.into_iter().map(|(_, r, c)| (r, c)).collect::<Vec<_>>();
    let (a, b) = (looks(dots(&mut u)), {
        frames(&mut u, 6);
        looks(dots(&mut u))
    });
    assert_eq!(a.len(), 2 * COLUMNS);
    assert_ne!(a, b, "the wave moves");
    let (mut u, _) = calm();
    u.app.run("pref.motion", json!({"motion": "off"}));
    poster(&mut u, 300.0);
    frames(&mut u, 2);
    let a = dots(&mut u);
    frames(&mut u, 6);
    assert_eq!(a, dots(&mut u), "the static row");
    assert_eq!(a.len(), 2 * COLUMNS);
}

#[test]
fn it_fades_out_once_the_grace_period_passes_and_the_dots_switch_is_its_own() {
    let (mut u, _) = calm();
    u.app.session.agents.grace_for_tests = Some(5);
    poster(&mut u, 300.0);
    frames(&mut u, 10);
    assert!(u.app.ui.agent_view.indicator.artboard().is_some());
    frames(&mut u, ((5.0 + FADE_OUT) * 60.0) as usize + 4);
    assert!(u.app.ui.agent_view.indicator.artboard().is_none(), "gone after the grace period");
    assert!(u.app.ui.agent_view.indicator.drawn.get().is_none());
    // "Show working indicator" off: no dots, the outline stays.
    let r = agent(&mut u, "pref.workingIndicator", json!({"on": false}));
    assert_eq!(r["on"], false);
    poster(&mut u, 900.0);
    u.app.run("select.none", json!({}));
    frames(&mut u, 30);
    assert!(u.app.ui.agent_view.indicator.drawn.get().is_none() && u.app.ui.agent_view.indicator.outlined.get().is_some());
}

#[test]
fn at_its_brightest_the_row_reads_at_3_to_1_in_both_themes() {
    use crate::accent_ink::for_page;
    use crate::theme::Tokens;
    use crate::theme::accent::contrast;
    for (theme, t) in [("light", Tokens::light()), ("dark", Tokens::dark())] {
        for page in [t.canvas, crate::theme::c32(figcraft_color::Color::DEFAULT_PAGE), crate::theme::c32(figcraft_color::Color::DARK_PAGE)] {
            let dot = for_page(t.accent_base, page);
            assert!(contrast(dot, page) >= 3.0, "{theme}: {dot:?} on {page:?}");
        }
    }
}

#[test]
fn the_switches_are_saved_and_answer_over_the_route() {
    let mut s = Session::new();
    assert!(s.prefs.settings.reveal_agent_work && s.prefs.settings.working_indicator, "on by default");
    assert_eq!(s.execute("pref.revealAgentWork", &json!({})).unwrap(), json!({"on": false}));
    assert_eq!(s.execute("pref.workingIndicator", &json!({"on": false})).unwrap(), json!({"on": false}));
    let v = serde_json::to_value(&s.prefs.settings).unwrap();
    assert_eq!(
        (v["reveal_agent_work"].clone(), v["working_indicator"].clone(), v["settings_version"].clone()),
        (json!(false), json!(false), json!(figcraft_engine::settings::SETTINGS_VERSION))
    );
    let old: figcraft_engine::settings::Settings = serde_json::from_value(json!({"settings_version": 3})).unwrap();
    let old = old.sanitized();
    assert!(old.reveal_agent_work && old.working_indicator, "a file from version 3 reads them on");
}
