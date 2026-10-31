# Opening on Home (Fix AD, 31 Oct 2026)

**Before:** only the first run started on the Home screen; every later launch made a new "Untitled" file and hid Home. A file opened from Finder arrived after launch, so it left that empty Untitled tab behind.

**Now:**
1. **Launching with no file** shows Home only. No document is made, so nothing is added to recents and nothing is autosaved. (`apps/figcraft/src/first_screen.rs`)
2. **A file named on the command line, or recovered from a run that ended unsaved,** opens as before, alone; the window shows it. A file from Finder or the Dock opens the same way, and no Untitled tab is left behind.
3. **File ▸ New, Home's New design file and the + tab** make a blank document, as before.
4. **Closing the last tab** returns to Home, with no document.
5. **No "restore the last session" setting** existed, and none was added: Home is the default.
6. **With no file open,** every command that acts on the current document refuses with `EngineError::NoDocument`. The window says it in the interface language: "No file is open. Open a file, or start a new design file from Home." (`windows::error_text`; Spanish in its catalog). The left rail, canvas and right panel (Design, Prototype, Code, AI) aren't drawn while Home shows, so nothing renders against a missing document.
7. **MCP:** the endpoint and `scrim mcp-bridge` start and connect with no document. A tool that needs one answers with a structured error, and nothing is made on the agent's behalf:
   ```json
   {"error": "no_document", "message": "No document is open in Scrim: the window is showing the Home screen.", "howToOpen": "Open a file with open_file {…} …, or make a blank one with run_command {\"command\": \"file.new\"}."}
   ```
   `open_file` and `file.import` work from Home and open the file in its own tab.

## Decision (overrule it)

**A file that arrives in a window with no files is shown.** When the session has no document, the first one made or opened by any route (an agent's `open_file`, File ▸ New run by an agent, a file from Finder) takes the window off Home, as the Open dialog always did. The engine asks for it (`doc.shown`, from `cmd::show_new_doc`, the one place a new document is added), and the window takes its requests in order, so Home asked for after it (Help ▸ Getting started) still wins. With files open nothing is asked: Home chosen from the tab bar stays, and an agent opening a file doesn't pull the person away. `ui.inspect`'s `home` now says whether Home shows (it said only whether it was asked for).

## Tests

- `apps/figcraft/src/first_screen_tests.rs`: no file named → no document, Home; a named file opens alone; a file that can't open leaves Home; a file from Finder after launch opens alone. The first, third and fourth fail on the old startup.
- `crates/ui-egui/src/home_start_tests.rs`: New design file and File ▸ New from Home make one document, and the window shows it; a file opened on an empty Home is shown; Home chosen with files open stays; closing the last document returns to Home with none; a document command with none open is refused in the interface's words (English, and the Spanish catalog's); autosave and recents do nothing with no document. Three fail on the old code (File ▸ New, the opened file, the words); the others already held and guard them.
- `crates/engine/src/cmd/no_document_tests.rs`: every command in the registry, run with no document: none panics, none makes a document except the ones whose job that is, and the document commands refuse with `NoDocument`. (The engine already passed this; it guards it.)
- `crates/mcp/tests/no_document.rs`: the structured error for `create_node` and `run_command` with no document, nothing made after asking twice, then `open_file` works. Both fail on the old code.
