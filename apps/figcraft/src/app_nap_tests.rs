use super::Awake;

#[test]
fn idle_holds_nothing_and_stopping_twice_is_fine() {
    let mut a = Awake::default();
    assert!(!a.held());
    a.set(false);
    a.set(false);
    assert!(!a.held());
}

#[test]
fn working_holds_one_activity_until_it_stops() {
    let mut a = Awake::default();
    a.set(true);
    a.set(true);
    assert_eq!(a.held(), cfg!(target_os = "macos"));
    a.set(false);
    assert!(!a.held());
}

/// The real activity, with options the system lists (`pmset -g assertions`; the app's own, which
/// allow idle sleep, aren't listed): taken while held, gone once let go.
#[cfg(target_os = "macos")]
#[test]
fn the_system_sees_the_activity_taken_and_released() {
    use objc2_foundation::NSActivityOptions;
    let reason = format!("scrim-app-nap-test-{}", std::process::id());
    let listed = || {
        let out = std::process::Command::new("pmset").args(["-g", "assertions"]).output().expect("pmset runs");
        String::from_utf8_lossy(&out.stdout).contains(&reason)
    };
    let mut a = Awake::default();
    assert!(!listed());
    a.set_with(true, NSActivityOptions::UserInitiated, &reason);
    assert!(listed(), "held: the system lists it");
    a.set_with(false, NSActivityOptions::UserInitiated, &reason);
    // The system updates its list right after the release; give it a moment.
    let gone = (0..50).any(|_| {
        std::thread::sleep(std::time::Duration::from_millis(20));
        !listed()
    });
    assert!(gone, "let go: the system no longer lists it");
}
