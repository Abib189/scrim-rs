//! Fix AI C-2: the app menu's items run commands. The native menu itself (and a real ⌘Q) can't be
//! driven from a test: that ⌘Q is the Quit item's key equivalent is checked in the built app.

use crate::app_menu::command;

#[test]
fn quit_and_about_run_their_commands() {
    assert_eq!(command("scrim.quit"), Some("app.quit"));
    assert_eq!(command("scrim.about"), Some("help.about"));
    assert_eq!(command("something else"), None);
}
