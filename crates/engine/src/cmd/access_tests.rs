//! Read or write (C1.3): every registered command is classified, and run_command / batch are held
//! to the tools' switches.

use super::{Access, EQUIVALENT, READ, WRITE, access, check, check_spec};
use crate::cmd::CommandSpec;
use crate::cmd::mcp_tools::TOOLS;

#[test]
fn every_registered_command_is_classified_once() {
    let unclassified: Vec<&str> = crate::cmd::all_commands().iter().filter(|c| access(c.id).is_none()).map(|c| c.id).collect();
    assert!(unclassified.is_empty(), "classify these in cmd/access.rs (READ or WRITE): {unclassified:?}");
    for id in READ.iter().chain(WRITE) {
        let spec = crate::find_command(id).unwrap_or_else(|| panic!("{id} is listed but not registered"));
        assert!(!spec.history, "{id} is in the undo history: it writes, don't list it");
    }
    assert!(!READ.iter().any(|r| WRITE.contains(r)), "listed as both");
    for (cmd, tool) in EQUIVALENT {
        assert!(crate::find_command(cmd).is_some(), "{cmd}");
        assert!(TOOLS.iter().any(|t| t.0 == *tool), "{tool}");
    }
    assert_eq!(access("node.get"), Some(Access::Read));
    assert_eq!(access("node.create"), Some(Access::Write));
    assert_eq!(access("view.zoomIn"), Some(Access::Write), "the view is a change too");
}

#[test]
fn a_turned_off_tools_command_is_refused_through_run_command() {
    let off = |t: &[&str]| t.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert!(check("node.create", &[]).is_ok());
    let e = check("node.create", &off(&["create_node"])).unwrap_err();
    assert!(e.contains("create_node") && e.contains("turned off"), "{e}");
    assert!(check("node.set", &off(&["create_node"])).is_ok(), "only its own tool's");
    assert!(check("command.batch", &off(&["batch"])).is_err(), "run_command can't reach batch");
    assert!(check("select.all", &off(&["select"])).is_err());
    assert!(check("select.layersAt", &off(&["select"])).is_ok(), "a read under select.");
}

#[test]
fn a_turned_off_group_refuses_its_commands() {
    let reads: Vec<String> = TOOLS.iter().filter(|t| t.2).map(|t| t.0.to_string()).collect();
    let writes: Vec<String> = TOOLS.iter().filter(|t| !t.2).map(|t| t.0.to_string()).collect();
    assert!(check("page.list", &reads).unwrap_err().contains("every read-only tool"));
    assert!(check("view.zoomIn", &reads).is_ok(), "a change, with the reads off");
    assert!(check("view.zoomIn", &writes).unwrap_err().contains("every tool that makes changes"));
}

#[test]
fn an_unclassified_command_is_refused() {
    let spec = CommandSpec { id: "plugin.unheard-of", label: "x", params: "", history: false, run: |_, _| Ok(serde_json::Value::Null) };
    let e = check_spec(&spec, &[]).unwrap_err();
    assert!(e.contains("isn't classified"), "{e}");
}
