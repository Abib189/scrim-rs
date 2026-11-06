//! Fix R-4: `release-macos` without the machine: its arguments, which identity signs, the plan, and
//! that no step runs without what an earlier step makes. Nothing here touches the keychain, the
//! network or Apple: listings and answers are written out, and a stand-in runs the plan.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::*;

const DEV_ID: &str = "Developer ID Application: Ada Example (ABCDE12345)";

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

fn listing() -> String {
    format!(
        "Policy: Code Signing\n  Matching identities\n  1) 0123456789ABCDEF0123456789ABCDEF01234567 \"Maelig Local Development\"\n  2) 89ABCDEF0123456789ABCDEF0123456789ABCDEF \"{DEV_ID}\"\n     2 valid identities found\n"
    )
}

#[test]
fn arguments_are_read_and_the_flag_wins_over_the_environment() {
    let a = parse(&args(&["--dry-run", "--skip-notarize", "--identity", DEV_ID, "--notary-profile", "other"]), Some("ignored".into())).unwrap();
    assert_eq!(a, Args { dry_run: true, skip_notarize: true, identity: Some(DEV_ID.into()), profile: "other".into() });
    let a = parse(&[], Some(DEV_ID.into())).unwrap();
    assert_eq!((a.identity.as_deref(), a.profile.as_str(), a.dry_run, a.skip_notarize), (Some(DEV_ID), DEFAULT_PROFILE, false, false));
    assert_eq!(parse(&[], Some("  ".into())).unwrap().identity, None, "a blank SCRIM_SIGN_IDENTITY is none");
    assert!(parse(&args(&["--identity"]), None).unwrap_err().contains("needs"));
    assert!(parse(&args(&["--notary-profile"]), None).unwrap_err().contains("needs"));
    assert!(parse(&args(&["--notary-profile", " "]), None).unwrap_err().contains("empty"));
    assert!(parse(&args(&["--identity", ""]), None).unwrap_err().contains("empty"));
    assert!(parse(&args(&["--sign"]), None).unwrap_err().contains("unknown argument"));
}

#[test]
fn the_keychain_listing_gives_the_names() {
    assert_eq!(identity_names(&listing()), vec!["Maelig Local Development".to_string(), DEV_ID.to_string()]);
    assert!(identity_names("     0 valid identities found\n").is_empty());
}

#[test]
fn only_a_named_listed_developer_id_identity_signs() {
    let listed = identity_names(&listing());
    assert_eq!(choose_identity(Some(DEV_ID), &listed).unwrap(), DEV_ID);
    // Missing: never picked for the person.
    assert!(choose_identity(None, &listed).unwrap_err().contains("no signing identity"));
    // The development certificate, even though it is in the keychain.
    assert!(choose_identity(Some("Maelig Local Development"), &listed).unwrap_err().contains("isn't a Developer ID Application"));
    // Not exactly a listed name: another person's, a prefix, another case.
    for wrong in ["Developer ID Application: Someone Else (ZZZZZ99999)", "Developer ID Application: Ada Example", &DEV_ID.to_lowercase()] {
        assert!(choose_identity(Some(wrong), &listed).is_err(), "{wrong}");
    }
    // Two with the name: ambiguous.
    let twice = vec![DEV_ID.to_string(), DEV_ID.to_string()];
    assert!(choose_identity(Some(DEV_ID), &twice).unwrap_err().contains("2 code-signing identities"));
}

#[test]
fn the_minimum_macos_is_the_same_in_the_build_and_the_plist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let template = std::fs::read_to_string(root.join("packaging/macos/Info.plist.in")).unwrap();
    assert!(min_macos_matches(&template).is_ok());
    let other = template.replace("<string>11.0</string>", "<string>12.0</string>");
    assert!(min_macos_matches(&other).is_err());
}

fn the_plan(skip_notarize: bool) -> (Vec<Step>, Paths) {
    let a = Args { dry_run: true, skip_notarize, identity: Some(DEV_ID.into()), profile: DEFAULT_PROFILE.into() };
    let p = Paths::new(Path::new("/t"));
    (plan(&a, &p, "abc123", "0.7.0", DEV_ID, &[]), p)
}

fn lines(plan: &[Step]) -> Vec<String> {
    plan.iter().map(describe).collect()
}

fn position(lines: &[String], has: &str) -> usize {
    lines.iter().position(|l| l.contains(has)).unwrap_or_else(|| panic!("no step with {has:?} in\n{}", lines.join("\n")))
}

#[test]
fn the_plan_builds_both_slices_signs_inside_out_and_notarises_with_the_profile() {
    let (plan, p) = the_plan(false);
    let l = lines(&plan);
    let build = &l[position(&l, "cargo build")];
    for want in [
        "MACOSX_DEPLOYMENT_TARGET=11.0",
        "FIGCRAFT_BUILD_SHA=abc123",
        "--release",
        "--locked",
        "-p figcraft ",
        "-p figcraft-cli",
        "--target aarch64-apple-darwin",
        "--target x86_64-apple-darwin",
    ] {
        assert!(build.contains(want), "{want} in {build}");
    }
    for bin in BINARIES {
        assert!(l.iter().any(|x| x.contains("lipo -create") && x.contains(&format!("bin/{bin}"))), "{bin} made universal");
        assert!(plan.iter().any(|s| s.expect == Expect::BothArchs && describe(s).contains(&format!("bin/{bin}"))), "{bin} checked for both slices");
    }
    // Inside out: the nested CLI, the executable, then the bundle; hardened runtime and timestamp.
    let (cli, exe, app) = (
        position(&l, "--sign \"Developer ID Application: Ada Example (ABCDE12345)\" --identifier design.scrim.app.cli"),
        position(&l, "Contents/MacOS/figcraft"),
        l.iter().position(|x| x.contains("codesign --force") && x.ends_with("release-macos/Scrim.app")).unwrap(),
    );
    assert!(cli < exe && exe < app, "{cli} < {exe} < {app}");
    for i in [cli, exe, app] {
        assert!(l[i].contains("--options runtime") && l[i].contains("--timestamp") && !l[i].contains("--deep"), "{}", l[i]);
    }
    assert!(!l.iter().any(|x| x.contains("--entitlements")), "no entitlements are needed");
    let submits: Vec<&String> = l.iter().filter(|x| x.contains("notarytool submit")).collect();
    assert_eq!(submits.len(), 2, "the app, then the image");
    assert!(submits.iter().all(|x| x.contains("--keychain-profile scrim-notary") && x.contains("--wait")), "{submits:?}");
    assert!(submits[0].contains("Scrim-notarize.zip") && submits[1].contains(&p.dmg("0.7.0").display().to_string()), "{submits:?}");
    let dmg = p.dmg("0.7.0").display().to_string();
    assert!(position(&l, "codesign --force --timestamp --sign") < position(&l, &format!("submit {dmg}")), "the image is signed before it is sent");
    assert!(l.iter().any(|x| x.contains("--type open --context context:primary-signature")));
    // Credentials come only from the profile.
    for never in ["--password", "--apple-id", "--team-id", "Maelig"] {
        assert!(!l.iter().any(|x| x.contains(never)), "{never} in the plan");
    }
}

#[test]
fn skip_notarize_signs_and_builds_the_image_only() {
    let (plan, _) = the_plan(true);
    let l = lines(&plan);
    assert!(l.iter().any(|x| x.contains("hdiutil create")) && l.iter().any(|x| x.contains("codesign --force --timestamp --sign")));
    for absent in ["notarytool", "stapler", "spctl", "ditto -c -k", "copy Scrim.app out of it"] {
        assert!(!l.iter().any(|x| x.contains(absent)), "{absent} with --skip-notarize");
    }
    assert!(l.last().unwrap().contains("SHA-256"));
}

#[test]
fn missing_rust_targets_are_added_first() {
    let a = Args { dry_run: true, skip_notarize: true, identity: Some(DEV_ID.into()), profile: DEFAULT_PROFILE.into() };
    let plan = plan(&a, &Paths::new(Path::new("/t")), "s", "0.7.0", DEV_ID, &["x86_64-apple-darwin"]);
    assert_eq!(describe(&plan[0]), "[targets] rustup target add x86_64-apple-darwin");
}

/// Runs the plan: each program "makes" what the plan says it makes; `fail` names a step that
/// fails, `answers` gives programs' output by their first two words.
struct Fake {
    made: BTreeSet<PathBuf>,
    ran: Vec<String>,
    fail: Option<&'static str>,
    answers: Vec<(&'static str, String)>,
    makes: Vec<(String, PathBuf)>,
}

impl Fake {
    fn new(plan: &[Step], p: &Paths) -> Self {
        let mut makes = Vec::new();
        for t in TARGETS {
            for b in BINARIES {
                makes.push(("cargo build".to_string(), p.target.join(t).join("release").join(b)));
            }
        }
        for b in BINARIES {
            makes.push((format!("lipo -create -output {}", p.work.join("bin").join(b).display()), p.work.join("bin").join(b)));
        }
        makes.push(("hdiutil create".into(), p.dmg("0.7.0")));
        makes.push(("ditto".into(), p.shipped_app()));
        makes.push(("ditto -c -k --keepParent".into(), p.work.join("Scrim-notarize.zip")));
        makes.push((format!("xcrun stapler staple {}", p.work.join("Scrim.app").display()), p.ticket()));
        let _ = plan;
        let answers = vec![
            ("lipo -archs", "x86_64 arm64".to_string()),
            ("figcraft --version", "Scrim 0.7.0 (abc123)".to_string()),
            ("codesign -dvv", format!("CodeDirectory v=20500 size=1 flags=0x10000(runtime) hashes=1\nAuthority={DEV_ID}\nTimestamp=1 Nov 2026\n")),
            ("xcrun notarytool", "{\"id\":\"sub-1\",\"status\":\"Accepted\"}".to_string()),
        ];
        Self { made: BTreeSet::new(), ran: Vec::new(), fail: None, answers, makes }
    }
}

impl Exec for Fake {
    fn run(&mut self, argv: &[String], _: &[(String, String)]) -> Result<String, String> {
        let line = argv.join(" ");
        self.ran.push(line.clone());
        if self.fail.is_some_and(|f| line.contains(f)) {
            return Err("failed".into());
        }
        for (prefix, path) in &self.makes {
            if line.starts_with(prefix.as_str()) || (prefix == "ditto" && line.starts_with("ditto")) {
                self.made.insert(path.clone());
            }
        }
        let key = |l: &str| l.split_whitespace().take(2).collect::<Vec<_>>().join(" ");
        let k = key(&line.replace("/t/release-macos/bin/", ""));
        Ok(self.answers.iter().find(|(a, _)| k.starts_with(a) || line.contains(a)).map(|(_, o)| o.clone()).unwrap_or_default())
    }
    fn act(&mut self, action: &Action) -> Result<(), String> {
        let p = Paths::new(Path::new("/t"));
        match action {
            Action::Assemble => {
                for x in ["Contents/MacOS/scrim", "Contents/MacOS/figcraft", ""] {
                    self.made.insert(if x.is_empty() { p.work.join("Scrim.app") } else { p.work.join("Scrim.app").join(x) });
                }
            }
            Action::Stage => {
                self.made.insert(p.work.join("dmg/Scrim.app"));
            }
            Action::FromImage => {
                self.made.insert(p.copied_out());
            }
            _ => {}
        }
        self.ran.push(format!("{action:?}"));
        Ok(())
    }
    fn exists(&self, path: &Path) -> bool {
        self.made.contains(path)
    }
}

#[test]
fn the_whole_plan_runs_when_each_step_makes_what_the_next_needs() {
    let (plan, p) = the_plan(false);
    let mut fake = Fake::new(&plan, &p);
    execute(&plan, &mut fake, &mut |_| {}).unwrap();
    assert_eq!(fake.ran.len(), plan.len());
}

#[test]
fn no_step_runs_without_what_an_earlier_step_makes() {
    let (plan, p) = the_plan(false);
    // The build "fails" to make the Intel slices: lipo never runs, nor anything after it.
    let mut fake = Fake::new(&plan, &p);
    fake.makes.retain(|(_, path)| !path.starts_with(p.target.join("x86_64-apple-darwin")));
    let e = execute(&plan, &mut fake, &mut |_| {}).unwrap_err();
    assert!(e.contains("x86_64-apple-darwin/release/figcraft") && e.contains("didn't make"), "{e}");
    assert!(!fake.ran.iter().any(|l| l.starts_with("lipo") || l.contains("codesign") || l.contains("notarytool")), "{:?}", fake.ran);
    // Each step that needs a file refuses to run on a plan that starts there.
    for (i, step) in plan.iter().enumerate().filter(|(_, s)| !s.needs.is_empty()) {
        let mut fresh = Fake::new(&plan, &p);
        let e = execute(&plan[i..], &mut fresh, &mut |_| {}).unwrap_err();
        assert!(e.contains("didn't make") && fresh.ran.is_empty(), "step {} ({}) ran without its input: {e}", i + 1, describe(step));
    }
}

#[test]
fn a_failed_step_stops_the_run() {
    let (plan, p) = the_plan(false);
    let mut fake = Fake::new(&plan, &p);
    fake.fail = Some("codesign --force --options runtime");
    assert!(execute(&plan, &mut fake, &mut |_| {}).unwrap_err().contains("failed"));
    assert!(!fake.ran.iter().any(|l| l.contains("hdiutil") || l.contains("notarytool")));
}

#[test]
fn a_signature_without_the_runtime_the_identity_or_a_timestamp_is_refused() {
    let (plan, p) = the_plan(true);
    for wrong in [
        format!("CodeDirectory v=20500 flags=0x0(none)\nAuthority={DEV_ID}\nTimestamp=1\n"),
        "CodeDirectory flags=0x10000(runtime)\nAuthority=Maelig Local Development\nTimestamp=1\n".to_string(),
        format!("CodeDirectory flags=0x10000(runtime)\nAuthority={DEV_ID}\n"),
        "Signature=adhoc\nCodeDirectory flags=0x10002(adhoc,runtime)\n".to_string(),
    ] {
        let mut fake = Fake::new(&plan, &p);
        fake.answers.retain(|(a, _)| *a != "codesign -dvv");
        fake.answers.push(("codesign -dvv", wrong.clone()));
        assert!(execute(&plan, &mut fake, &mut |_| {}).unwrap_err().contains("signature"), "{wrong}");
    }
}

#[test]
fn a_rejected_submission_fetches_its_log_with_the_profile_and_stops() {
    let (plan, p) = the_plan(false);
    let mut fake = Fake::new(&plan, &p);
    fake.answers.retain(|(a, _)| *a != "xcrun notarytool");
    fake.answers.push(("xcrun notarytool", "{\"id\":\"sub-9\",\"status\":\"Invalid\"}".into()));
    let e = execute(&plan, &mut fake, &mut |_| {}).unwrap_err();
    assert!(e.contains("Invalid") && e.contains("sub-9"), "{e}");
    assert!(fake.ran.iter().any(|l| l == "xcrun notarytool log sub-9 --keychain-profile scrim-notary"), "{:?}", fake.ran);
    assert!(!fake.ran.iter().any(|l| l.contains("stapler")), "nothing stapled after a refusal");
}

#[test]
fn notarytool_answers_are_read() {
    assert_eq!(notary_result("{\"id\":\"a\",\"status\":\"Accepted\",\"message\":\"ok\"}"), (Some("a".into()), Some("Accepted".into())));
    assert_eq!(
        notary_result("Conducting pre-submission checks...\n{\"id\":\"b\",\"status\":\"Invalid\"}"),
        (Some("b".into()), Some("Invalid".into()))
    );
    assert_eq!(notary_result("Error: HTTP 401"), (None, None));
}

#[test]
fn the_checksum_file_sits_beside_the_image() {
    assert_eq!(sha_file(Path::new("/t/release/Scrim-0.7.0-universal.dmg")), PathBuf::from("/t/release/Scrim-0.7.0-universal.dmg.sha256"));
}

/// Fix R2-1: the app is zipped, notarised, stapled and validated before the image is made from it;
/// then the image is signed, notarised, stapled and validated; then the app copied out of the
/// mounted image is validated.
#[test]
fn the_app_is_notarised_and_stapled_before_the_image_is_made() {
    let (plan, p) = the_plan(false);
    let l = lines(&plan);
    let app = p.work.join("Scrim.app").display().to_string();
    let dmg = p.dmg("0.7.0").display().to_string();
    let order = [
        format!("--sign \"{DEV_ID}\" {app}"),
        format!("ditto -c -k --keepParent {app}"),
        "submit /t/release-macos/Scrim-notarize.zip".to_string(),
        format!("stapler staple {app}"),
        format!("stapler validate {app}"),
        "the image's folder".to_string(),
        "hdiutil create -volname Scrim -srcfolder".to_string(),
        format!("codesign --force --timestamp --sign \"{DEV_ID}\" {dmg}"),
        format!("submit {dmg}"),
        format!("stapler staple {dmg}"),
        format!("stapler validate {dmg}"),
        "copy Scrim.app out of it".to_string(),
        "stapler validate /t/release-macos/from-image/Scrim.app".to_string(),
        "spctl --assess --type execute -vv /t/release-macos/from-image/Scrim.app".to_string(),
        "codesign --verify --deep --strict --verbose=2 /t/release-macos/from-image/Scrim.app".to_string(),
        "SHA-256".to_string(),
    ];
    let at: Vec<usize> = order.iter().map(|o| position(&l, o)).collect();
    assert!(at.windows(2).all(|w| w[0] < w[1]), "out of order: {:?}", order.iter().zip(&at).collect::<Vec<_>>());
}

#[test]
fn the_image_is_never_made_from_an_app_that_isnt_stapled() {
    let (plan, p) = the_plan(false);
    // Stapling writes the ticket into the app; staging the image needs it, so the executor
    // refuses to make the image from an app that wasn't stapled.
    let staging = plan.iter().position(|s| s.action == Action::Stage).unwrap();
    assert!(plan[staging].needs.contains(&p.ticket()), "{:?}", plan[staging].needs);
    let mut fake = Fake::new(&plan, &p);
    fake.makes.retain(|(_, path)| *path != p.ticket());
    let e = execute(&plan, &mut fake, &mut |_| {}).unwrap_err();
    assert!(e.contains("CodeResources") && e.contains("didn't make"), "{e}");
    assert!(!fake.ran.iter().any(|l| l.contains("hdiutil") || l.contains("Stage")), "no image: {:?}", fake.ran);
}

#[test]
fn a_refused_app_submission_stops_before_any_image() {
    let (plan, p) = the_plan(false);
    let mut fake = Fake::new(&plan, &p);
    fake.answers.retain(|(a, _)| *a != "xcrun notarytool");
    fake.answers.push(("xcrun notarytool", "{\"id\":\"sub-app\",\"status\":\"Invalid\"}".into()));
    let e = execute(&plan, &mut fake, &mut |_| {}).unwrap_err();
    assert!(e.contains("sub-app") && e.contains("Invalid"), "{e}");
    assert!(fake.ran.iter().any(|l| l == "xcrun notarytool log sub-app --keychain-profile scrim-notary"), "the log is fetched");
    assert!(!fake.ran.iter().any(|l| l.contains("stapler") || l.contains("hdiutil")), "nothing stapled, no image: {:?}", fake.ran);
}
