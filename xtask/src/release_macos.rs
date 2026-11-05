//! `cargo xtask release-macos` (Fix R, `docs/release/macos.md`): Scrim for macOS as it is released.
//! A universal app (Apple silicon and Intel), signed with a Developer ID Application identity with
//! the hardened runtime, on a signed `.dmg` that Apple notarises and the ticket is stapled to.
//!
//! 1. **Preconditions:** macOS; a clean tree whose commit is pushed (the app names its commit); the
//!    two Rust targets (added with rustup when missing); the tools; the identity (below); the
//!    minimum macOS the same in the build and the Info.plist ([`MIN_MACOS`]).
//! 2. **Build** `figcraft` (the app) and `scrim` (the CLI and MCP bridge) in release mode for both
//!    targets, in one cargo command, and `lipo` each into a universal binary with both slices.
//! 3. **Assemble** `Scrim.app` exactly as `bundle-macos` does ([`crate::bundle_macos::assemble`]):
//!    `Contents/MacOS/figcraft` (the app) and `scrim` beside it, the Info.plist with the `.scrim`
//!    document type, the icon, the version and the commit, which `figcraft --version` must name.
//! 4. **Sign** inside out, never `--deep`: `scrim`, then `figcraft`, then the bundle, each with the
//!    hardened runtime (`--options runtime`) and a secure timestamp. No entitlements: see
//!    `packaging/macos/entitlements.plist` for why none is needed (not sandboxed, no JIT, no
//!    libraries loaded from outside the app).
//! 5. **Notarise the app** (zipped with `ditto -c -k --keepParent`, `notarytool submit
//!    --keychain-profile <profile> --wait`) and **staple** its ticket to it (Fix R2-1), so the app
//!    on the image carries it: a copy dragged out opens offline too. When Apple refuses a
//!    submission, its log says why and nothing goes on.
//! 6. **Disk image** of the stapled app and an `Applications` link, made without attaching a device
//!    (`hdiutil makehybrid`, then compressed), signed, notarised and stapled in turn.
//! 7. **Verify:** `spctl` on the app and the image, `stapler validate` on both, `codesign --verify
//!    --deep --strict`, and the app copied out of the mounted image still validates.
//! 8. A **SHA-256** file beside the image, in `shasum -c` form.
//!
//! The result is in `<target>/release/` (cargo's own release folder: the image, its checksum and
//! the stapled app); the work in `<target>/release-macos/`. Nothing is uploaded.
//!
//! **Identity.** It is always named: `--identity` or `SCRIM_SIGN_IDENTITY`, and it must be exactly
//! one of the identities the keychain lists for code signing, and a Developer ID Application one.
//! Anything else is refused (a local development certificate signs nothing others can open).
//!
//! **Credentials.** Notarisation reads them from the Keychain profile (`--notary-profile`, default
//! [`DEFAULT_PROFILE`]); this never sees, prints or writes them.
//!
//! **Flags:** `--dry-run` prints the plan and runs no step (it only reads the tree, the targets and
//! the keychain's identity names); `--skip-notarize` signs and builds the image only, with neither
//! submission.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The targets the universal binaries hold.
pub const TARGETS: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];
/// The minimum macOS: Big Sur, the first release on Apple silicon, with the Metal features wgpu
/// uses. The Info.plist template says the same ([`min_macos_matches`]).
pub const MIN_MACOS: &str = "11.0";
/// The binaries in the app, and the cargo packages that make them.
pub const BINARIES: [&str; 2] = ["figcraft", "scrim"];
const PACKAGES: [&str; 2] = ["figcraft", "figcraft-cli"];
/// The Keychain profile `notarytool store-credentials` made.
pub const DEFAULT_PROFILE: &str = "scrim-notary";
const DEVELOPER_ID: &str = "Developer ID Application: ";

/// The command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args {
    pub dry_run: bool,
    pub skip_notarize: bool,
    pub identity: Option<String>,
    pub profile: String,
}

/// Read the arguments; `env_identity` is `SCRIM_SIGN_IDENTITY` (the flag wins).
pub fn parse(args: &[String], env_identity: Option<String>) -> Result<Args, String> {
    let mut a = Args { dry_run: false, skip_notarize: false, identity: None, profile: DEFAULT_PROFILE.into() };
    let mut it = args.iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--dry-run" => a.dry_run = true,
            "--skip-notarize" => a.skip_notarize = true,
            "--identity" => a.identity = Some(it.next().ok_or("--identity needs the identity's name")?.clone()),
            "--notary-profile" => a.profile = it.next().ok_or("--notary-profile needs the profile's name")?.clone(),
            other => {
                return Err(format!(
                    "unknown argument `{other}` (release-macos [--dry-run] [--skip-notarize] [--identity <name>] [--notary-profile <name>])"
                ));
            }
        }
    }
    if a.identity.as_deref().is_some_and(|i| i.trim().is_empty()) {
        return Err("--identity can't be empty".into());
    }
    if a.identity.is_none() {
        a.identity = env_identity.filter(|s| !s.trim().is_empty());
    }
    if a.profile.trim().is_empty() {
        return Err("--notary-profile can't be empty".into());
    }
    Ok(a)
}

/// The identity names in `security find-identity -v -p codesigning`'s listing (`  1) <sha1> "<name>"`).
pub fn identity_names(listing: &str) -> Vec<String> {
    listing
        .lines()
        .filter(|l| l.trim_start().split(')').next().is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())))
        .filter_map(|l| {
            let (a, b) = (l.find('"')?, l.rfind('"')?);
            (b > a).then(|| l[a + 1..b].to_string())
        })
        .collect()
}

/// The identity to sign with: `given`, when it is a Developer ID Application identity and exactly
/// one of the `listed` ones has its name.
pub fn choose_identity(given: Option<&str>, listed: &[String]) -> Result<String, String> {
    let Some(id) = given else {
        return Err("no signing identity: pass --identity \"Developer ID Application: <name> (<team>)\" or set SCRIM_SIGN_IDENTITY \
                    (`security find-identity -v -p codesigning` lists them)"
            .into());
    };
    if !id.starts_with(DEVELOPER_ID) {
        return Err(format!("`{id}` isn't a Developer ID Application identity: only one of those signs an app other Macs open"));
    }
    match listed.iter().filter(|n| n.as_str() == id).count() {
        1 => Ok(id.to_string()),
        0 => Err(format!("the keychain has no code-signing identity named exactly `{id}`")),
        n => Err(format!("{n} code-signing identities are named `{id}`: remove the old ones from the keychain")),
    }
}

/// The Info.plist template's minimum macOS is [`MIN_MACOS`], the one the binaries are built for.
pub fn min_macos_matches(template: &str) -> Result<(), String> {
    let want = format!("<key>LSMinimumSystemVersion</key>\n  <string>{MIN_MACOS}</string>");
    if template.contains(&want) {
        Ok(())
    } else {
        Err(format!("packaging/macos/Info.plist.in's LSMinimumSystemVersion isn't {MIN_MACOS}, the macOS the binaries are built for"))
    }
}

/// Where everything goes.
#[derive(Clone, Debug)]
pub struct Paths {
    pub target: PathBuf,
    pub work: PathBuf,
    pub out: PathBuf,
}

impl Paths {
    pub fn new(target: &Path) -> Self {
        Self { target: target.to_path_buf(), work: target.join("release-macos"), out: target.join("release") }
    }
    fn slice(&self, target: &str, bin: &str) -> PathBuf {
        self.target.join(target).join("release").join(bin)
    }
    fn universal(&self, bin: &str) -> PathBuf {
        self.work.join("bin").join(bin)
    }
    fn app(&self) -> PathBuf {
        self.work.join("Scrim.app")
    }
    fn stage(&self) -> PathBuf {
        self.work.join("dmg")
    }
    fn raw(&self) -> PathBuf {
        self.work.join("raw.dmg")
    }
    pub fn dmg(&self, version: &str) -> PathBuf {
        self.out.join(format!("Scrim-{version}-universal.dmg"))
    }
    pub fn shipped_app(&self) -> PathBuf {
        self.out.join("Scrim.app")
    }
    /// The signed app, zipped for its own notarisation.
    fn zip(&self) -> PathBuf {
        self.work.join("Scrim-notarize.zip")
    }
    /// What stapling writes into the app: the ticket.
    pub fn ticket(&self) -> PathBuf {
        self.app().join("Contents/CodeResources")
    }
    /// Where the image is mounted (read-only) to copy the app out of it.
    fn mount(&self) -> PathBuf {
        self.work.join("mounted")
    }
    /// The app as copied out of the mounted image.
    pub fn copied_out(&self) -> PathBuf {
        self.work.join("from-image/Scrim.app")
    }
}

/// What a step does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Run a program.
    Run { argv: Vec<String>, env: Vec<(String, String)> },
    /// Clear what an earlier run left in the work folder and the outputs.
    Clear,
    /// `Scrim.app` from the universal binaries ([`crate::bundle_macos::assemble`]).
    Assemble,
    /// The image's folder: the app and an `Applications` link.
    Stage,
    /// The `.sha256` file beside the image.
    Checksum,
    /// Mount the image read-only, copy the app out of it as a person would, and detach.
    FromImage,
}

/// What a step's output must show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expect {
    Nothing,
    /// `lipo -archs` lists both slices.
    BothArchs,
    /// `--version` names the commit.
    Commit(String),
    /// `codesign -dvv`: the hardened runtime, signed by `identity` with a secure timestamp.
    Signed {
        identity: String,
    },
    /// `notarytool submit` accepted; otherwise its log is fetched with the profile.
    Accepted {
        profile: String,
    },
}

/// One step of the plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub stage: &'static str,
    pub action: Action,
    /// Files an earlier step makes; the step doesn't run without them.
    pub needs: Vec<PathBuf>,
    pub expect: Expect,
}

fn run(stage: &'static str, argv: &[&str], needs: &[&Path]) -> Step {
    Step {
        stage,
        action: Action::Run { argv: argv.iter().map(|s| s.to_string()).collect(), env: Vec::new() },
        needs: needs.iter().map(|p| p.to_path_buf()).collect(),
        expect: Expect::Nothing,
    }
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap_or_default()
}

/// The plan for `args` at commit `sha`, version `version`, signing with `identity`; `missing`: the
/// Rust targets rustup must add first.
pub fn plan(args: &Args, p: &Paths, sha: &str, version: &str, identity: &str, missing: &[&str]) -> Vec<Step> {
    let mut steps = Vec::new();
    for t in missing {
        steps.push(run("targets", &["rustup", "target", "add", t], &[]));
    }
    steps.push(Step { stage: "clear", action: Action::Clear, needs: Vec::new(), expect: Expect::Nothing });
    let mut argv: Vec<String> = ["cargo", "build", "--release", "--locked"].map(String::from).to_vec();
    for pkg in PACKAGES {
        argv.extend(["-p".into(), pkg.into()]);
    }
    for t in TARGETS {
        argv.extend(["--target".into(), t.into()]);
    }
    let env = vec![("MACOSX_DEPLOYMENT_TARGET".into(), MIN_MACOS.into()), ("FIGCRAFT_BUILD_SHA".into(), sha.into())];
    steps.push(Step { stage: "build", action: Action::Run { argv, env }, needs: Vec::new(), expect: Expect::Nothing });
    steps.push(run("universal", &["mkdir", "-p", s(&p.work.join("bin"))], &[]));
    for bin in BINARIES {
        let (arm, intel, uni) = (p.slice(TARGETS[0], bin), p.slice(TARGETS[1], bin), p.universal(bin));
        steps.push(run("universal", &["lipo", "-create", "-output", s(&uni), s(&arm), s(&intel)], &[&arm, &intel]));
        steps.push(Step { expect: Expect::BothArchs, ..run("universal", &["lipo", "-archs", s(&uni)], &[&uni]) });
    }
    let fig = p.universal("figcraft");
    steps.push(Step { expect: Expect::Commit(sha.into()), ..run("universal", &[s(&fig), "--version"], &[&fig]) });
    let unis: Vec<PathBuf> = BINARIES.iter().map(|b| p.universal(b)).collect();
    steps.push(Step { stage: "assemble", action: Action::Assemble, needs: unis, expect: Expect::Nothing });
    let app = p.app();
    let (inner_cli, inner_app) = (app.join("Contents/MacOS/scrim"), app.join("Contents/MacOS/figcraft"));
    let sign = |path: &Path, extra: &[&str]| {
        let mut argv = vec!["codesign", "--force", "--options", "runtime", "--timestamp", "--sign", identity];
        argv.extend_from_slice(extra);
        argv.push(s(path));
        run("sign", &argv, &[path])
    };
    // Inside out: the nested CLI, the main executable, then the bundle (never --deep).
    steps.push(sign(&inner_cli, &["--identifier", "design.scrim.app.cli"]));
    steps.push(sign(&inner_app, &[]));
    steps.push(sign(&app, &[]));
    steps.push(run("sign", &["codesign", "--verify", "--deep", "--strict", "--verbose=2", s(&app)], &[&app]));
    for code in [&app, &inner_cli] {
        steps.push(Step { expect: Expect::Signed { identity: identity.into() }, ..run("sign", &["codesign", "-dvv", s(code)], &[code]) });
    }
    let profile = args.profile.as_str();
    let submit = |stage: &'static str, what: &Path| Step {
        expect: Expect::Accepted { profile: profile.into() },
        ..run(
            stage,
            &["xcrun", "notarytool", "submit", s(what), "--keychain-profile", profile, "--wait", "--timeout", "2h", "--output-format", "json"],
            &[what],
        )
    };
    // The app is notarised and stapled first, so the image holds an app that carries its own
    // ticket: a copy dragged out of it opens offline too (Fix R2-1).
    let ticket = p.ticket();
    if !args.skip_notarize {
        let zip = p.zip();
        steps.push(run("notarize app", &["ditto", "-c", "-k", "--keepParent", s(&app), s(&zip)], &[&app]));
        steps.push(submit("notarize app", &zip));
        steps.push(run("staple app", &["xcrun", "stapler", "staple", s(&app)], &[&app]));
        steps.push(run("staple app", &["xcrun", "stapler", "validate", s(&app)], &[&ticket]));
    }
    let (stage, raw, dmg) = (p.stage(), p.raw(), p.dmg(version));
    // Notarising, the image is staged only from the stapled app: the ticket is a file it needs.
    let staged_from = if args.skip_notarize { vec![app.clone()] } else { vec![app.clone(), ticket.clone()] };
    steps.push(Step { stage: "dmg", action: Action::Stage, needs: staged_from, expect: Expect::Nothing });
    steps.push(run(
        "dmg",
        &["hdiutil", "makehybrid", "-hfs", "-hfs-volume-name", "Scrim", "-hfs-openfolder", s(&stage), "-o", s(&raw), s(&stage)],
        &[&stage.join("Scrim.app")],
    ));
    steps.push(run("dmg", &["hdiutil", "convert", s(&raw), "-format", "UDZO", "-imagekey", "zlib-level=9", "-o", s(&dmg)], &[&raw]));
    steps.push(run("dmg", &["codesign", "--force", "--timestamp", "--sign", identity, s(&dmg)], &[&dmg]));
    steps.push(run("dmg", &["codesign", "--verify", "--strict", "--verbose=2", s(&dmg)], &[&dmg]));
    if !args.skip_notarize {
        steps.push(submit("notarize dmg", &dmg));
        steps.push(run("staple dmg", &["xcrun", "stapler", "staple", s(&dmg)], &[&dmg]));
        steps.push(run("staple dmg", &["xcrun", "stapler", "validate", s(&dmg)], &[&dmg]));
    }
    let shipped = p.shipped_app();
    steps.push(run("verify", &["ditto", s(&app), s(&shipped)], &[&app]));
    if !args.skip_notarize {
        let out = p.copied_out();
        steps.push(run("verify", &["spctl", "--assess", "--type", "execute", "-vv", s(&shipped)], &[&shipped]));
        steps.push(run("verify", &["spctl", "--assess", "--type", "open", "--context", "context:primary-signature", "-vv", s(&dmg)], &[&dmg]));
        steps.push(run("verify", &["xcrun", "stapler", "validate", s(&shipped)], &[&shipped]));
        // What a person gets: the app copied out of the mounted image keeps its ticket.
        steps.push(Step { stage: "verify", action: Action::FromImage, needs: vec![dmg.clone()], expect: Expect::Nothing });
        steps.push(run("verify", &["xcrun", "stapler", "validate", s(&out)], &[&out]));
        steps.push(run("verify", &["spctl", "--assess", "--type", "execute", "-vv", s(&out)], &[&out]));
    }
    steps.push(run("verify", &["codesign", "--verify", "--deep", "--strict", "--verbose=2", s(&shipped)], &[&shipped]));
    steps.push(Step { stage: "checksum", action: Action::Checksum, needs: vec![dmg], expect: Expect::Nothing });
    steps
}

/// One line of the printed plan.
pub fn describe(step: &Step) -> String {
    let what = match &step.action {
        Action::Run { argv, env } => {
            let env: Vec<String> = env.iter().map(|(k, v)| format!("{k}={v}")).collect();
            let argv: Vec<String> = argv.iter().map(|a| if a.contains(' ') { format!("\"{a}\"") } else { a.clone() }).collect();
            [env, argv].concat().join(" ")
        }
        Action::Clear => "clear the work folder and the previous image, checksum and app".into(),
        Action::Assemble => "assemble Scrim.app (as bundle-macos: Info.plist, icon, figcraft and scrim)".into(),
        Action::Stage => "the image's folder: Scrim.app and an Applications link".into(),
        Action::Checksum => "write the image's SHA-256 beside it (shasum -a 256)".into(),
        Action::FromImage => "mount the image read-only (hdiutil attach -nobrowse -readonly), copy Scrim.app out of it (ditto), detach".into(),
    };
    format!("[{}] {what}", step.stage)
}

/// What runs the steps: the machine, or a test's stand-in.
pub trait Exec {
    /// Run `argv` with `env`; its output (stdout and stderr), or why it failed with that output.
    fn run(&mut self, argv: &[String], env: &[(String, String)]) -> Result<String, String>;
    /// Do one of the actions that aren't a program.
    fn act(&mut self, action: &Action) -> Result<(), String>;
    fn exists(&self, path: &Path) -> bool;
}

/// Run `plan`, stopping at the first step that fails, or that lacks a file an earlier step makes.
pub fn execute(plan: &[Step], x: &mut dyn Exec, log: &mut dyn FnMut(&str)) -> Result<(), String> {
    for (i, step) in plan.iter().enumerate() {
        let n = i + 1;
        if let Some(missing) = step.needs.iter().find(|p| !x.exists(p)) {
            return Err(format!("step {n} ({}) needs {}, which an earlier step didn't make", describe(step), missing.display()));
        }
        log(&format!("{n}/{} {}", plan.len(), describe(step)));
        let started = std::time::Instant::now();
        let out = match &step.action {
            // notarytool exits non-zero when Apple refuses: its answer still goes to the check,
            // which fetches the log.
            Action::Run { argv, env } if matches!(step.expect, Expect::Accepted { .. }) => x.run(argv, env).unwrap_or_else(|e| e),
            Action::Run { argv, env } => x.run(argv, env).map_err(|e| format!("step {n} ({}) failed: {e}", describe(step)))?,
            other => {
                x.act(other).map_err(|e| format!("step {n} ({}) failed: {e}", describe(step)))?;
                String::new()
            }
        };
        check(&step.expect, &out, x).map_err(|e| format!("step {n} ({}): {e}", describe(step)))?;
        log(&format!("{n}/{} [{}] done in {:.1} s", plan.len(), step.stage, started.elapsed().as_secs_f64()));
    }
    Ok(())
}

/// The submission's id and status in `notarytool --output-format json`'s answer.
pub fn notary_result(out: &str) -> (Option<String>, Option<String>) {
    let v: Option<serde_json::Value> =
        out.lines().rev().find_map(|l| serde_json::from_str(l.trim()).ok()).or_else(|| serde_json::from_str(out.trim()).ok());
    let get = |k: &str| v.as_ref().and_then(|v| v.get(k)).and_then(serde_json::Value::as_str).map(str::to_string);
    (get("id"), get("status"))
}

fn check(expect: &Expect, out: &str, x: &mut dyn Exec) -> Result<(), String> {
    match expect {
        Expect::Nothing => Ok(()),
        Expect::BothArchs => {
            let archs: Vec<&str> = out.split_whitespace().collect();
            if archs.contains(&"arm64") && archs.contains(&"x86_64") { Ok(()) } else { Err(format!("not universal: {}", out.trim())) }
        }
        Expect::Commit(sha) => {
            if crate::bundle_macos::version_has_commit(out, sha) {
                Ok(())
            } else {
                Err(format!("`--version` doesn't name the commit {sha}: {:?}", out.trim()))
            }
        }
        Expect::Signed { identity } => {
            let runtime = out.lines().any(|l| l.starts_with("CodeDirectory") && l.contains("(runtime)"));
            let by = out.lines().find_map(|l| l.strip_prefix("Authority=")) == Some(identity.as_str());
            let stamped = out.lines().any(|l| l.starts_with("Timestamp="));
            if runtime && by && stamped {
                Ok(())
            } else {
                Err(format!("signature: hardened runtime {runtime}, signed by the identity {by}, secure timestamp {stamped}"))
            }
        }
        Expect::Accepted { profile } => match notary_result(out) {
            (_, Some(status)) if status == "Accepted" => Ok(()),
            (id, status) => {
                let log = match &id {
                    Some(id) => {
                        let argv: Vec<String> = ["xcrun", "notarytool", "log", id, "--keychain-profile", profile].map(String::from).to_vec();
                        x.run(&argv, &[]).unwrap_or_else(|e| e)
                    }
                    None => "(no submission id: notarytool's answer is above)".into(),
                };
                Err(format!(
                    "Apple didn't accept the image (status {}, submission {}). Its log:\n{log}",
                    status.as_deref().unwrap_or("unknown"),
                    id.as_deref().unwrap_or("unknown")
                ))
            }
        },
    }
}

/// The machine: programs run, and the actions done, for real.
struct Machine<'a> {
    root: &'a Path,
    paths: &'a Paths,
    sha: String,
    version: String,
}

impl Exec for Machine<'_> {
    fn run(&mut self, argv: &[String], env: &[(String, String)]) -> Result<String, String> {
        let (prog, rest) = argv.split_first().ok_or("an empty command")?;
        let prog = if prog == "cargo" { std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()) } else { prog.clone() };
        let mut cmd = Command::new(prog);
        cmd.args(rest).current_dir(self.root).stdin(std::process::Stdio::null());
        for (k, v) in env {
            cmd.env(k, v);
        }
        // The build prints as it goes; the rest are short and their output is checked.
        if argv.first().is_some_and(|p| p == "cargo" || p == "rustup") {
            return if cmd.status().is_ok_and(|s| s.success()) { Ok(String::new()) } else { Err("see above".into()) };
        }
        let o = cmd.output().map_err(|e| e.to_string())?;
        let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
        for line in text.lines().take(40) {
            eprintln!("    {line}");
        }
        if o.status.success() { Ok(text) } else { Err(format!("exit {}\n{text}", o.status.code().unwrap_or(-1))) }
    }

    fn act(&mut self, action: &Action) -> Result<(), String> {
        let p = self.paths;
        match action {
            Action::Run { .. } => Err("not an action".into()),
            Action::Clear => {
                let _ = std::fs::remove_dir_all(&p.work);
                let _ = std::fs::remove_dir_all(p.shipped_app());
                let dmg = p.dmg(&self.version);
                let _ = std::fs::remove_file(&dmg);
                let _ = std::fs::remove_file(sha_file(&dmg));
                std::fs::create_dir_all(&p.work).and_then(|()| std::fs::create_dir_all(&p.out)).map_err(|e| e.to_string())
            }
            Action::Assemble => {
                let exes: Vec<(&str, PathBuf)> = BINARIES.iter().map(|b| (*b, p.universal(b))).collect();
                crate::bundle_macos::assemble(self.root, &p.work, &exes, &self.version, &self.sha, MIN_MACOS).map(|_| ())
            }
            Action::Stage => {
                let stage = p.stage();
                let _ = std::fs::remove_dir_all(&stage);
                std::fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
                let ok = Command::new("ditto").arg(p.app()).arg(stage.join("Scrim.app")).status().is_ok_and(|s| s.success());
                if !ok {
                    return Err("ditto couldn't copy the app".into());
                }
                #[cfg(unix)]
                return std::os::unix::fs::symlink("/Applications", stage.join("Applications")).map_err(|e| e.to_string());
                #[cfg(not(unix))]
                return Err("the image is made on macOS only".into());
            }
            Action::FromImage => {
                let (mount, out) = (p.mount(), p.copied_out());
                let dmg = p.dmg(&self.version);
                let _ = std::fs::remove_dir_all(out.parent().unwrap_or(&out));
                std::fs::create_dir_all(&mount).map_err(|e| e.to_string())?;
                let attach = Command::new("hdiutil")
                    .args(["attach", "-nobrowse", "-readonly", "-noautoopen", "-mountpoint"])
                    .arg(&mount)
                    .arg(&dmg)
                    .stdin(std::process::Stdio::null())
                    .output()
                    .map_err(|e| e.to_string())?;
                if !attach.status.success() {
                    return Err(format!("hdiutil attach: {}", String::from_utf8_lossy(&attach.stderr).trim()));
                }
                let copied = std::fs::create_dir_all(out.parent().unwrap_or(&out)).is_ok()
                    && Command::new("ditto").arg(mount.join("Scrim.app")).arg(&out).status().is_ok_and(|s| s.success());
                // Detached whatever the copy did.
                let detached =
                    Command::new("hdiutil").arg("detach").arg(&mount).stdin(std::process::Stdio::null()).output().is_ok_and(|o| o.status.success());
                match (copied, detached) {
                    (true, true) => Ok(()),
                    (false, _) => Err("couldn't copy Scrim.app out of the mounted image".into()),
                    (true, false) => Err(format!("couldn't detach {}", mount.display())),
                }
            }
            Action::Checksum => {
                let dmg = p.dmg(&self.version);
                let name = dmg.file_name().and_then(|n| n.to_str()).ok_or("the image's name")?;
                let o = Command::new("shasum").args(["-a", "256", name]).current_dir(&p.out).output().map_err(|e| e.to_string())?;
                let line = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !o.status.success() || line.len() < 64 {
                    return Err("shasum failed".into());
                }
                eprintln!("    {line}");
                std::fs::write(sha_file(&dmg), format!("{line}\n")).map_err(|e| e.to_string())
            }
        }
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

/// The checksum file of image `dmg`.
pub fn sha_file(dmg: &Path) -> PathBuf {
    let mut s = dmg.as_os_str().to_os_string();
    s.push(".sha256");
    PathBuf::from(s)
}

fn output(cmd: &mut Command) -> Option<String> {
    cmd.stdin(std::process::Stdio::null()).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// The command: `args` after `release-macos`.
pub fn release(root: &Path, args: &[String]) -> bool {
    let args = match parse(args, std::env::var("SCRIM_SIGN_IDENTITY").ok()) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("release-macos: {e}");
            return false;
        }
    };
    if !cfg!(target_os = "macos") {
        eprintln!("release-macos: builds the macOS release, on macOS only");
        return false;
    }
    let mut problems = Vec::new();
    let git = |a: &[&str]| output(Command::new("git").args(a).current_dir(root));
    let sha = git(&["rev-parse", "HEAD"]).unwrap_or_default();
    if sha.is_empty() {
        problems.push("can't tell the commit (run in the git checkout)".to_string());
    }
    if git(&["status", "--porcelain"]).is_none_or(|s| !s.is_empty()) {
        problems.push("the tree has changes: commit them first (the app names the commit it is built from)".into());
    }
    match git(&["rev-list", "--count", "@{upstream}..HEAD"]) {
        Some(n) if n == "0" => {}
        Some(n) => problems.push(format!("{n} commit(s) aren't pushed: push first, so the commit the app names is public")),
        None => problems.push("the branch has no upstream to compare with: push it first".into()),
    }
    match std::fs::read_to_string(root.join("packaging/macos/Info.plist.in")) {
        Ok(t) => {
            if let Err(e) = min_macos_matches(&t) {
                problems.push(e);
            }
        }
        Err(e) => problems.push(format!("packaging/macos/Info.plist.in: {e}")),
    }
    let tools = ["lipo", "codesign", "hdiutil", "ditto", "shasum", "plutil", "iconutil", "spctl"];
    for t in tools.iter().filter(|t| output(Command::new("which").arg(t)).is_none()) {
        problems.push(format!("`{t}` isn't installed"));
    }
    if !args.skip_notarize {
        for t in ["notarytool", "stapler"].iter().filter(|t| output(Command::new("xcrun").args(["--find", t])).is_none()) {
            problems.push(format!("`xcrun {t}` isn't there: install Xcode or its command line tools"));
        }
    }
    let installed = output(Command::new("rustup").args(["target", "list", "--installed"])).unwrap_or_default();
    let missing: Vec<&str> = TARGETS.iter().copied().filter(|t| !installed.lines().any(|l| l.trim() == *t)).collect();
    let listed = output(Command::new("security").args(["find-identity", "-v", "-p", "codesigning"])).map(|l| identity_names(&l)).unwrap_or_default();
    let identity = match choose_identity(args.identity.as_deref(), &listed) {
        Ok(id) => id,
        Err(e) => {
            problems.push(e);
            args.identity.clone().unwrap_or_else(|| "<identity: pass --identity>".into())
        }
    };
    let version = crate::workspace_version();
    let paths = Paths::new(&crate::bundle_macos::target_dir(root));
    let steps = plan(&args, &paths, &sha, &version, &identity, &missing);
    if args.dry_run {
        eprintln!("release-macos --dry-run: Scrim {version} ({sha}), the plan (nothing runs):");
        for (i, s) in steps.iter().enumerate() {
            eprintln!("  {:>2}. {}", i + 1, describe(s));
        }
        for t in &missing {
            eprintln!("  (rustup would add the target {t}: it isn't installed)");
        }
        for p in &problems {
            eprintln!("  would stop first: {p}");
        }
        eprintln!("  output: {} and {}", paths.dmg(&version).display(), sha_file(&paths.dmg(&version)).display());
        return true;
    }
    if !problems.is_empty() {
        for p in &problems {
            eprintln!("release-macos: {p}");
        }
        return false;
    }
    let mut machine = Machine { root, paths: &paths, sha, version: version.clone() };
    let started = std::time::Instant::now();
    // Each stage's time, printed as the next one starts.
    let mut stage = String::new();
    let mut stage_start = std::time::Instant::now();
    let mut log = |line: &str| {
        let this = line.split_once('[').and_then(|(_, r)| r.split_once(']')).map_or("", |(s, _)| s);
        if this != stage {
            if !stage.is_empty() {
                eprintln!("release-macos: {stage} took {:.0} s", stage_start.elapsed().as_secs_f64());
            }
            stage = this.to_string();
            stage_start = std::time::Instant::now();
        }
        eprintln!("release-macos: {line}");
    };
    let result = execute(&steps, &mut machine, &mut log);
    log("[end]");
    if let Err(e) = result {
        eprintln!("release-macos: {e}");
        return false;
    }
    eprintln!(
        "release-macos: done in {:.0} s: {} ({}notarised), {}",
        started.elapsed().as_secs_f64(),
        paths.dmg(&version).display(),
        if args.skip_notarize { "not " } else { "" },
        sha_file(&paths.dmg(&version)).display()
    );
    true
}

#[cfg(test)]
#[path = "release_macos_tests.rs"]
mod tests;
