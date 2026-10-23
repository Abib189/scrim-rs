//! Fix V2: the Follow agents setting survives every settings file Scrim has written, and only the
//! person changes it (`docs/design/follow-setting.md`).
//! - A file written by the build before Fix T (version 7, Follow on, an unmarked 30 s grace) stays
//!   on through the migration, the save and a restart; so does every older version, on or off.
//! - A fresh settings folder follows agents.
//! - The person's change is kept across a restart, killed or quit.
//! - A migration changes nothing but its own keys.
//!
//! The settings files are real: `tests/fixtures/settings/`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use figcraft_engine::Session;
use figcraft_engine::cmd::camera::follow_state;
use figcraft_ui_egui::FigApp;
use serde_json::{Value, json};

use crate::services::Desktop;

const BEFORE_FIX_T: &str = include_str!("../tests/fixtures/settings/state-v7-before-fix-t.json");
const FIX_T: &str = include_str!("../tests/fixtures/settings/state-v8-fix-t.json");

/// The keys each settings version added (`Settings::settings_version`'s list).
const ADDED: [(u32, &[&str]); 5] = [
    (3, &["follow_agents"]),
    (4, &["reveal_agent_work", "working_indicator"]),
    (5, &["agent_grace_secs"]),
    (6, &["reveal_speed"]),
    (7, &["agent_grace_chosen"]),
];

fn root(name: &str) -> PathBuf {
    let r = std::env::temp_dir().join(format!("scrim-follow-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&r);
    std::fs::create_dir_all(&r).unwrap();
    r
}

/// The app as it starts with its settings in `dir`.
fn start(dir: &Path) -> FigApp {
    let svc = Desktop::load_from(Some(dir.to_path_buf()), Arc::new(Mutex::new(None)));
    let mut s = Session::new();
    if let Some(prefs) = svc.prefs.clone() {
        s.prefs = prefs;
    }
    FigApp::new(s, Box::new(svc))
}

fn frames(fig: &mut FigApp, ctx: &egui::Context) {
    for _ in 0..2 {
        ctx.run_ui(egui::RawInput::default(), |ui| fig.ui(ui)).textures_delta.clear();
    }
}

/// What the app saved in `dir`.
fn saved(dir: &Path) -> Value {
    serde_json::from_str::<Value>(&std::fs::read_to_string(dir.join("state.json")).unwrap()).unwrap()["prefs"].clone()
}

/// The pre-Fix-T file as version `v` wrote it, with Follow `on` (from version 3, which added it).
fn at_version(v: u32, on: bool) -> Value {
    let mut d: Value = serde_json::from_str(BEFORE_FIX_T).unwrap();
    let p = d["prefs"].as_object_mut().unwrap();
    for (since, keys) in ADDED {
        if v < since {
            for k in keys {
                p.remove(*k);
            }
        }
    }
    if v == 0 {
        p.remove("settings_version");
    } else {
        p.insert("settings_version".into(), json!(v));
    }
    if p.contains_key("follow_agents") {
        p.insert("follow_agents".into(), json!(on));
    }
    d
}

/// Start on `file` (read and migrated as the app starts), save as the app quits, start again:
/// following as each start saw it, and as saved. Without the window, whose startup asks the system
/// for its language and motion setting in child processes: a sweep of starts would fork often
/// enough to hold the recovery tests' locks (`recovery_tests.rs`, #334).
fn round_trip(name: &str, file: &str) -> (&'static str, &'static str, Value) {
    let dir = root(name);
    std::fs::write(dir.join("state.json"), file).unwrap();
    let state = |p: Option<figcraft_engine::Prefs>| {
        let mut s = Session::new();
        s.prefs = p.unwrap_or_default();
        follow_state(&s)
    };
    let prefs = crate::services::read_prefs(&dir);
    let first = state(prefs.clone());
    crate::services::write_prefs(&dir, &prefs.unwrap_or_default());
    let again = state(crate::services::read_prefs(&dir));
    let kept = saved(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    (first, again, kept)
}

#[test]
fn a_file_from_the_build_before_fix_t_with_follow_on_stays_on() {
    let (first, again, kept) = round_trip("before-fix-t", BEFORE_FIX_T);
    assert_eq!((first, again), ("following", "following"));
    assert_eq!((kept["follow_agents"].clone(), kept["settings_version"].clone()), (json!(true), json!(9)), "migrated, still on");
    let (first, again, kept) = round_trip("fix-t", FIX_T);
    assert_eq!((first, again, kept["follow_agents"].clone()), ("following", "following", json!(true)), "a Fix T file too");
}

#[test]
fn every_supported_settings_version_keeps_follow_unless_off_was_never_chosen() {
    for v in 0..=8 {
        for on in [true, false] {
            if v < 3 && !on {
                continue; // no key to say off before version 3
            }
            let (first, again, kept) = round_trip(&format!("v{v}-{on}"), &at_version(v, on).to_string());
            // Fix W1: off that nobody marked as chosen (no file before version 9 has the marker)
            // comes back on, once.
            assert_eq!((first, again, kept["follow_agents"].clone()), ("following", "following", json!(true)), "version {v}, follow {on}");
            assert_eq!(kept["follow_chosen"], false, "version {v}: a migration never writes the marker");
            let repaired = repairs(&kept);
            assert_eq!(repaired, usize::from(!on), "version {v}, follow {on}: noted once when it was off");
        }
    }
}

/// How many times Settings ▸ Diagnostics says version 9 turned following back on.
fn repairs(prefs: &Value) -> usize {
    prefs["diagnostics"].as_array().map_or(0, |d| d.iter().filter(|e| e["detail"] == REPAIRED).count())
}

const REPAIRED: &str = "follow setting: OFF → ON (made by a settings migration: saved off was never chosen)";

/// `file` with following `on` and the person's marker set.
fn chosen(mut file: Value, on: bool) -> Value {
    file["prefs"]["follow_agents"] = json!(on);
    file["prefs"]["follow_chosen"] = json!(true);
    file
}

/// Fix W1, on the real files from before Fix V (a Fix T build's, version 8) and before Fix T
/// (version 7): saved off with no marker is turned on once and noted; with the marker it stays
/// off; on stays on; and the repair never runs again.
#[test]
fn a_saved_off_nobody_chose_is_turned_on_once_and_a_chosen_one_is_kept() {
    for (name, file) in [("v8", FIX_T), ("v7", BEFORE_FIX_T)] {
        let mut off: Value = serde_json::from_str(file).unwrap();
        off["prefs"]["follow_agents"] = json!(false);
        let (first, again, kept) = round_trip(&format!("{name}-off"), &off.to_string());
        assert_eq!((first, again), ("following", "following"), "{name}: off, never chosen, comes back on");
        assert_eq!((repairs(&kept), kept["follow_chosen"].clone(), kept["settings_version"].clone()), (1, json!(false), json!(9)), "{name}");
        // Started again from what was saved: nothing more happens.
        let (_, _, twice) = round_trip(&format!("{name}-again"), &json!({"prefs": kept}).to_string());
        assert_eq!(repairs(&twice), 1, "{name}: the repair never runs twice");
        for on in [false, true] {
            let (first, again, kept) = round_trip(&format!("{name}-chosen-{on}"), &chosen(serde_json::from_str(file).unwrap(), on).to_string());
            let want = if on { "following" } else { "off" };
            assert_eq!((first, again, repairs(&kept)), (want, want, 0), "{name}: chosen {on} stays");
        }
        let (first, _, kept) = round_trip(&format!("{name}-on"), file);
        assert_eq!((first, repairs(&kept)), ("following", 0), "{name}: on stays on, nothing noted");
    }
    // A version 9 file with following off and no marker is never repaired: the migration ran once.
    let mut later: Value = serde_json::from_str(FIX_T).unwrap();
    later["prefs"]["settings_version"] = json!(9);
    later["prefs"]["follow_agents"] = json!(false);
    let (first, _, kept) = round_trip("v9-off", &later.to_string());
    assert_eq!((first, repairs(&kept)), ("off", 0));
}

/// Fix W1: after the repair, the person turning following off with the header's button writes the
/// marker, and a restart keeps it off: the repair doesn't undo the person's choice.
#[test]
fn the_persons_off_after_the_repair_is_kept_across_restarts() {
    let dir = root("repair-then-off");
    let mut off: Value = serde_json::from_str(FIX_T).unwrap();
    off["prefs"]["follow_agents"] = json!(false);
    std::fs::write(dir.join("state.json"), off.to_string()).unwrap();
    let mut fig = start(&dir);
    assert_eq!(follow_state(&fig.session), "following", "repaired");
    assert!(!fig.session.prefs.settings.follow_chosen);
    fig.run("camera.follow-agents", json!({"on": false, "origin": "button"})).unwrap();
    assert!(fig.session.prefs.settings.follow_chosen, "the person's choice is marked");
    crate::services::shutdown(&mut fig, Some(&dir), None);
    for _ in 0..2 {
        let mut fig = start(&dir);
        assert_eq!(follow_state(&fig.session), "off", "kept off");
        crate::services::shutdown(&mut fig, Some(&dir), None);
    }
    assert_eq!(repairs(&saved(&dir)), 1, "repaired once, before the person's choice");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_fresh_settings_folder_follows_agents() {
    let dir = root("fresh");
    let ctx = egui::Context::default();
    let mut fig = start(&dir);
    frames(&mut fig, &ctx);
    assert_eq!(follow_state(&fig.session), "following");
    crate::services::shutdown(&mut fig, Some(&dir), None);
    assert_eq!(saved(&dir)["follow_agents"], true);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_persons_change_is_kept_across_a_restart() {
    let dir = root("persons-change");
    std::fs::write(dir.join("state.json"), BEFORE_FIX_T).unwrap();
    let ctx = egui::Context::default();
    let mut fig = start(&dir);
    assert_eq!(fig.run("camera.follow-agents", json!({"on": false})).unwrap()["state"], "off");
    frames(&mut fig, &ctx);
    // Killed: nothing saves on the way out, so the change was kept when it was made.
    drop(fig);
    let mut fig = start(&dir);
    assert_eq!(follow_state(&fig.session), "off", "off after a kill");
    assert_eq!(fig.run("camera.follow-agents", json!({"on": true})).unwrap()["state"], "following");
    crate::services::shutdown(&mut fig, Some(&dir), None);
    assert_eq!(follow_state(&start(&dir).session), "following", "on after a quit");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Migrations change only what they are for: the pre-Fix-T file comes back with only its version and
/// its unmarked 30 s grace (raised to 60, Fix T1/U1) changed. Following, and every other value, is
/// as the person left it.
#[test]
fn a_migration_changes_nothing_but_its_own_keys() {
    let old: Value = serde_json::from_str(BEFORE_FIX_T).unwrap();
    let prefs: figcraft_engine::Prefs = serde_json::from_value(old["prefs"].clone()).unwrap();
    let mut migrated = prefs.clone();
    migrated.settings = migrated.settings.sanitized();
    let (a, b) = (serde_json::to_value(&prefs).unwrap(), serde_json::to_value(&migrated).unwrap());
    let changed: Vec<&String> = a.as_object().unwrap().iter().filter(|(k, v)| b.get(k.as_str()) != Some(v)).map(|(k, _)| k).collect();
    assert_eq!(changed, ["agent_grace_secs", "settings_version"], "only the documented migrations");
    assert_eq!(b["follow_agents"], true);
}

/// Fix V3: an app that starts with following off says so at once in Settings ▸ Diagnostics,
/// "following is OFF"; one that starts with it on says nothing about it.
#[test]
fn starting_with_following_off_says_so_in_diagnostics() {
    let lines = |file: &Value| {
        let dir = root(&format!("start-{}-{}", file["prefs"]["follow_agents"], file["prefs"]["follow_chosen"]));
        std::fs::write(dir.join("state.json"), file.to_string()).unwrap();
        let fig = start(&dir);
        let follow: Vec<String> = fig.session.prefs.settings.diagnostics.iter().filter(|e| e.kind == "follow").map(|e| e.detail.clone()).collect();
        let _ = std::fs::remove_dir_all(&dir);
        follow
    };
    // Off as the person chose it (an off nobody chose is turned on: Fix W1).
    assert_eq!(lines(&chosen(at_version(7, false), false)), ["following is OFF"]);
    assert!(lines(&at_version(7, true)).is_empty());
}
