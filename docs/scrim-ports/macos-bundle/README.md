# R4: the double-clickable Scrim.app, checked live (22 Oct 2026)

`r4.py` with `target/bundle/Scrim.app` from `cargo xtask bundle-macos` (commit `54d45ea` plus the task), on a scratch `HOME` and a scratch settings folder (`SCRIM_SETTINGS_DIR`, Scrim's rules 9), MCP port 7690. `d3.py` is the control-channel helper. The log, with every path written as `<bundle>`, `<moved>` or `<scratch>`, is `logs/log.json`.

| Step | Result |
|---|---|
| `open -n Scrim.app` (Launch Services, no terminal) | Listed as a foreground app, `design.scrim.app` |
| Its MCP endpoint | `initialize` answered by `scrim` 0.7.0 on 7690 |
| Claude Desktop's entry | `<bundle>/Contents/MacOS/figcraft` with `mcp-bridge`; the file Scrim reads is the scratch one |
| That bridge, started as Desktop starts it | initialize, 25 tools, `create_node` reached the window; its process never listed by Launch Services, no app added while it ran; exited 0 |
| Written into the scratch configuration | Configured |
| A copy opened from another folder | Out of date, naming the old path; the copy unregistered (`lsregister -u`) and deleted |
| The person's Claude Desktop configuration and Scrim settings | Same timestamp, size and checksum before and after (read only) |

**Not verified here:** the screen was locked, so macOS drew no window. There's no capture, and Re-register (which a person presses in the window) was skipped. The UI harness covers it (`desktop_flow_tests.rs` › `a_bridge_from_another_copy_of_scrim_is_out_of_date_and_re_registered`). A real Claude Desktop launch and a notarised build weren't tried either. `docs/design/macos-bundle.md` has the rest.
