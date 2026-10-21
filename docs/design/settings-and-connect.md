# Stage C: Settings, one-click MCP connect, CLI connect in the AI tab (plan)

**C0, 20 Oct 2026.** Stage C's brief, its two addenda, and what the code holds today. The later steps (C1–C4, and the small fixes and the fonts) are each a commit; this file records what they decide.

**References:** OpenPencil's settings dialog and AI tab, for structure and interaction only. No strings, branding, icons or code are copied. The rebrand test also refuses "OpenPencil" in interface strings from C1 on.

## What exists (survey at 7ccc97b)

### The home screen
- **Files:**
  - `crates/ui-egui/src/home.rs` (611 lines) draws the screen and its sidebar;
  - `home_recents.rs` (967) holds the Recents lists, `online.rs` (326) the Online section;
  - the tests are `home_tests.rs` and `home_recents_tests.rs`.
- **Layout:** the sidebar is not an egui panel. `home::sidebar` (home.rs:203–235) paints into a 240 px rect inside one `CentralPanel`:
  - the logo and wordmark;
  - the search field;
  - three rows, **Recents, Online and Templates**: 32 px tall on a 34 px step, a 16 px icon, a 12 px label, `t.selected_row` / `t.hover` fills, and AccessKit `Role::Tab`.
- **Nothing sits at the bottom.**
- **Under the limit:** none of these files is over 1,000 lines, but the Settings row goes in a new module anyway (`home_settings_row.rs`), with one call from `sidebar()`.

### How an agent reaches a running window today
1. **The control channel:** `figcraft --control PORT` (or `FIGCRAFT_CONTROL_PORT`) starts a JSON-lines TCP server on 127.0.0.1 (`apps/figcraft/src/control_server.rs`). Every request runs on the UI thread as `Author::agent("control channel")` (`crates/ui-egui/src/control.rs`), so locks, the stale-write guard (`since`) and "Agent: …" undo labels apply.
2. **MCP:** `scrim mcp` (stdio JSON-RPC, `crates/mcp`) runs headless on its own `Session`. `scrim mcp --attach PORT` forwards every tool to the control channel above. The app itself does not depend on `figcraft-mcp`.
3. **The connect command** the AI tab shows today is `agents::connect_command`: `claude mcp add scrim -- scrim mcp`, or the `--attach` form. It only works when the app was started with `--control`.

There is **no HTTP server** in the workspace (no hyper, axum or tiny_http). The collab server speaks WebSocket through tungstenite over `std::net`.

### Other things the brief assumes
| Brief | What exists | Plan |
|---|---|---|
| Cmd+, opens Settings | Cmd+, is bound to `app.preferences`, which opens the **Preferences** dialog (dialogs.rs:205–331) | **Settings replaces Preferences.** Every existing row moves into a Settings section; `app.preferences` stays as an alias that opens Settings at General; `settings.open {section?}` is the new command |
| "Wire to the existing toolbar config; keep the keyboard dock and grab" | **No toolbar configuration exists.** `toolbar.rs` has a fixed `GROUPS` table, and there is no "keyboard dock". "Grab" is only a cursor | C3 adds the configuration (`Prefs.toolbar`) and makes the toolbar draw from it. "Keyboard dock" and "grab" are recorded as not existing, so nothing can break |
| Snap to geometry | `view.snapGeometry` sets `ViewState::snap_geometry`, **which no snapping code reads**. Snapping uses `snap_objects` and `snap_pixel` | General shows **Snap to objects** and **Snap to pixel grid**, which work. Exposing the dead switch is refused; it is listed as a follow-up |
| Animations: Follow system / On / Off | `Prefs.reduce_motion: bool`, OR-ed with the system setting | A three-way `Prefs.motion` (`system` \| `on` \| `off`). Preferences without it read `reduce_motion` |
| Auto-preserve unsaved work | Recovery copies are **always on**, with no preference (`apps/figcraft/src/recovery.rs`) | A toggle, on by default, with a warning when it is turned off. Off stops new recovery copies; existing ones are kept |
| Display name for shared rooms | `Prefs.user_name` (`pref.userName`) is already sent to peers and shown at the cursor | Moves into General as-is |
| Theme: one source of truth with the moon icon | The tab bar's toggle runs `view.themeLight` / `view.themeDark`; Preferences used the same commands | Both run **`pref.theme`** (B4); `view.theme*` stay as menu aliases of it. A test changes one and asserts the other follows |
| Process spawning "through the platform seam" | `Services` has no such method; only private `Command` helpers exist (services.rs) | `Services::run_program(program, args: &[String]) -> Result<Output, String>`: desktop only, an argument array, never a shell string. The web build has no programs and shows Copy only |
| Diagnostics | No event log; `env_logger` to stderr | A new in-memory ring buffer of typed events. Only an allow-listed set of fields, never document text, prompts or tokens. Kept in the settings folder by its retention setting |

## Decisions (and where I disagree)

1. **The default port 7600 collides with OpenPencil.** OpenPencil's MCP server listens on 127.0.0.1:7600 on this machine right now (`claude mcp list`). With the brief's default, Scrim would fall back to 7601 here on every first run.
   - **Implemented as briefed:** 7600, the next free port, persisted.
   - **I recommend overruling it with a port of Scrim's own,** e.g. 7680, before anyone's configuration depends on it.
2. **`claude mcp add` scope.** Without `--scope`, Claude Code adds the server for the *current directory's project*. Run from the app, that is the app's working directory (often `/`), so the entry would be invisible in every real project.
   - **The command uses `--scope user`:** `claude mcp add --transport http --scope user scrim http://127.0.0.1:<port>/mcp`.
   - **This machine already has a `scrim` entry,** scoped to the `~/Scrim` project: `http://127.0.0.1:4848/mcp`, the TypeScript Scrim. A user-scope entry doesn't touch it; inside `~/Scrim` the project entry wins. The Connect flow reports what `claude mcp get scrim` finds and offers **Replace** for an entry at the same scope only.
3. **The HTTP endpoint is hand-written on `std::net`, with no new dependency.** It serves MCP's streamable-HTTP transport in its JSON-response form:
   - **`POST /mcp`:** a JSON-RPC message in, `application/json` out. A notification gets `202`.
   - **`GET /mcp`:** `405`, since there is no server-initiated stream.
   - **Limits:** `Content-Length` only (no chunked bodies), 32 MiB, per-connection timeouts.

   It runs the **same `figcraft_mcp::Server`**, with a new in-app backend that hands every call to the control queue. Calls therefore go through `control::handle`, as `Author::agent("mcp (http)")`, and locks, the stale-write guard and the undo labels apply unchanged.
   - **New dependency:** `apps/figcraft` → `figcraft-mcp`, a workspace crate (layering allows apps above crates).
   - **Not on wasm:** the browser can't listen.
4. **Security:**
   - binds 127.0.0.1 only;
   - refuses a `Host` other than `127.0.0.1:<port>` or `localhost:<port>`, and any `Origin` that isn't loopback, with `403`;
   - an optional bearer token, off by default. The Settings toggle generates a random token kept in the settings folder; the Connect command then carries `--header "Authorization: Bearer …"`.
5. **Tool access:**
   - **The list:** persisted in `Prefs.mcp_disabled_tools`.
   - **Where it applies:** in `Server` itself. `tools/list` hides a disabled tool, and `tools/call` refuses it with the reason.
   - **Which routes:**
     - the HTTP endpoint reads the app's preferences;
     - `scrim mcp --attach` asks the app with a new control method, `mcp.policy`;
     - headless `scrim mcp` reads `state.json` from the settings folder when it exists.
   - **"Move" can't be disabled** applies to the toolbar section, not to tools. Tools have no such floor, but `list_commands` and `inspect_ui` stay on so an agent can say what it can't do.
6. **The AI tab's connect UI:** C2's Connect flow **replaces** the Connection card (addendum 2). A test fails if "--control" or "scrim mcp" appear in the AI tab's strings. `ai.status` / `ai.copyConnectCommand` (B3) are kept for agents and describe the HTTP endpoint.
7. **The Connect flow is one module** (`connect_flow.rs`), used by the AI tab and by Settings ▸ AI & agents and ▸ MCP. Run asks once, then runs `claude` with an argument array.
   - **The status row:**
     - **Connected:** a client initialised over the endpoint in the last 10 minutes, and named itself;
     - **Not connected;**
     - **Server stopped.**
8. **Settings sections built now:** General, Toolbar, AI & agents, MCP, Tool access, Privacy, Diagnostics.
   - **Skipped,** without placeholder pages: Usage, Media, Cloud storage, Language.
   - **Language** stays reachable: the existing language row moves into General. The brief skips a Language *section*, not the setting, and removing it would take away a working preference.
9. **Persistence:**
   - **Where:** all Settings live in `Prefs`, saved to `design.scrim.app/state.json`.
   - **A version:** `state.json` gains `"settings_version": 1` (built in C3d; the preferences' keys are snake_case). A missing or invalid key falls back to its default (serde `default`, plus per-field checks on load); an unknown later version is read as far as it goes, never refused.
   - **Commands:** every change is a registry command, so it has an MCP route too.
10. **The `.figcraft` extension:**
    - **Allowed:** the rebrand test matches "FigCraft" case-sensitively, so the lowercase extension (and the Open file card's "Open .figcraft, .svg or images") passes.
    - **Kept,** per the rebrand decisions' group 6.
    - **Renaming it is a later decision.** It needs a compatibility importer for `.figcraft`, a new UTI and MIME type, and the file associations of both.
11. **Fonts (addendum 2): Inter and Source Serif 4, bundled, and Geist as well.** "Brand colour system mockups" also uses Geist in 499 layers (more than Inter's 70 and Source Serif 4's 20). Without it, the Missing fonts window would still open for that file on a machine without Geist. Geist is OFL 1.1 too, so it is bundled with the other two. **Overrule** if you want only the two you named; the window would then list Geist there.
    - **Conflict:** this goes against the rule "Fonts live in craft-fonts, never in this repo" in AGENTS.md / CLAUDE.md. The addendum is the owner's newer instruction, so the fonts commit updates the rule to name this exception. If you'd rather keep the rule, the alternative is to put these two families into craft-fonts and build with it.
    - **The source of the files:** this Mac already has both installed, the same OFL 1.1 files upstream ships:
      - Inter 4.001 (git-9221beed3, rsms/inter) and Source Serif 4 4.004 (adobe-fonts/source-serif), in `~/Library/Fonts`;
      - they are copied from there, with their SHA-256 recorded, rather than downloaded;
      - the OFL text comes from Inter's own `OFL.txt`, with each family's copyright line read from its font files.
    - **Desktop only:** they are embedded on the desktop. The web build doesn't embed them (about 4 MB) and fetches nothing, so the browser keeps its fallback. That is reported, not lazily loaded: a lazy load would be a network fetch.
    - **On this machine the Missing fonts dialog doesn't list them anyway** for "Brand colour system mockups", since they are installed. The test asserts it with the system fonts out of the way.
12. **The lint after the fonts resolve:** the counts on "Brand colour system mockups" are already 569 Overused font, 157 Low contrast text and 116 Tiny body text (959 in all) with the fonts resolved. The lint reads font names, not font files, so no change is expected. C-fonts reports the measured result.

## The six recent files (addendum 1, item 6; report only, nothing modified)

Each was opened read-only with the `scrim` CLI (`info`, then `render` of every page):

| File | Pages / frames | Missing fonts | Renders |
|---|---|---|---|
| formaglyph-logo-concepts | 3 / 14 | none | Yes, all 3 pages (checked by eye: page 1) |
| formaglyph-glyph-workbench | 10 / 86 | 8 families (Lato, Merriweather, Montserrat, Nunito Sans, Open Sans, Poppins, Roboto, Source Sans 3) | Yes, all 10 pages. On the type study the 8 missing families draw in the fallback sans (Merriweather shows as sans) |
| Brand colour system mockups | 3 / 12 | none (Inter and Source Serif 4 are installed) | Yes, all 3 pages |
| Posters | 1 / 23 | Poppins (5 layers) | Yes; those 5 text layers in the fallback |
| Minimal Poster Series | 1 / 5 | none | Yes |
| Night Bloom Poster | 1 / 1 | none | Yes |

## C1 as built

- **`crates/mcp/src/http.rs`:** the endpoint.
  - HTTP/1.1 framing (`Content-Length` only, keep-alive, 64 KiB of headers, 32 MiB bodies, idle timeouts);
  - the Host, Origin and bearer checks, before the body is read;
  - `POST /mcp` with JSON replies, `202` for notifications, `405` for `GET` / `DELETE`;
  - one thread owning the `Server`, at most 16 connections;
  - `Endpoint::start` tries the port, then the next 19;
  - `stop` joins its accept thread, so a restart gets the same port back.
- **`apps/figcraft/src/mcp_endpoint.rs`:** the in-app backend. Every MCP tool call is a control request on the same queue as `--control`, with `route: "mcp (http)"`, so the author is `agent (mcp (http))`.
  - An `initialize` reports its `clientInfo` to the UI through the `mcp.client` control method; the status shows Connected for 10 minutes after.
  - `Desktop` owns the handle; `Services::mcp_restart` and `mcp_configure` reach it. The `Services` trait moved out of ui-egui's frozen `lib.rs` into `services.rs` (1,730 → 1,675 lines).
- **File confinement:** HTTP requests run inside `file_access::confined` with the root folder (your home folder by default). An app started with `--automation-*-root` keeps those roots instead, so access is never widened.
- **The root folder** is a second default the brief left open: "Use default" means your home folder, not the whole disk.
- **The commands:** `mcp.status`, `mcp.connect-claude-code`, `mcp.copy-command`, `mcp.restart-server`, `mcp.toggle-auth`, `mcp.set-root-dir`, all in `crates/engine/src/cmd/mcp.rs`, with Spanish labels.
- **The token:** 64 hex digits from std's randomly keyed hasher, which is seeded from the OS. No new dependency.
- **Tests:**
  - `crates/mcp/tests/http.rs` (round trip, keep-alive; refused: five foreign Hosts, four Origins and a missing Host; token; 405, 404, 413 and 411; port fallback; same port after a restart; client report);
  - `apps/figcraft/src/mcp_endpoint_tests.rs` (route, client report; a refused request never reaches the UI; a token applies at once);
  - `crates/engine/src/cmd/mcp_tests.rs`;
  - `crates/mcp/tests/connect_docs.rs` (the HTTP command is the one `docs/mcp.md` and the README show).

## C2 as built

- **`crates/ui-egui/src/connect_flow.rs`:** one flow, used by the AI tab now and by Settings in C3.
  1. **Connect Claude Code** (`mcp.connect-claude-code`) shows the exact command, with Run and Copy (`mcp.copy-command`).
  2. Run (`mcp.run-connect`) opens **one confirmation**.
  3. Only then does Scrim run `claude mcp get scrim` and `claude mcp get figcraft`.
  4. A `scrim` entry pointing elsewhere is offered for **Replace** (remove at its scope, then add) or Keep it; one already pointing here, at user scope, is reported as done.
  5. Otherwise `claude mcp add --transport http --scope user scrim <url>`.
  6. Success, or Claude Code's own error text, is shown. An old `figcraft` entry gets **Remove the old entry** (`mcp.remove-old-entry`, confirmed again).
- **Running programs:** `Services::run_program` runs on a background thread, with an argument list and no shell (`std::process::Command`).
- **Finding `claude`:** `Services::find_program` looks along `PATH` and then the usual install folders. An app started from the Finder gets a short `PATH` without `~/.local/bin`, where `claude` lives here. Without `claude`, the panel offers Copy only and says why.
- **Agents can't run programs:** the confirmations are `dialog.*` requests, which an agent's command never opens (`an_agent_cannot_make_scrim_run_a_program`).
- **The AI tab's Connection card is gone,** along with its `--control` copy and the `scrim mcp` command. In its place:
  - a status row: Connected with the client's name (an `initialize` in the last 10 minutes), Not connected, or Server stopped with Restart;
  - the empty state: one line and **Connect Claude Code**;
  - the flow.
- **The string guard:** `the_ai_tab_shows_no_control_channel_or_scrim_mcp_copy` fails if "--control" or "scrim mcp" appears in a string literal of `ai_tab.rs` or `connect_flow.rs`. The activity feed, lint findings and lock from B3 are unchanged.
- **The rebrand test now also refuses "OpenPencil"** in interface strings.

## C3a as built: the Settings shell and General

- **`settings_dialog.rs`:** a rail of sections on the left (AccessKit tabs, focus ring), the section's pane on the right (it scrolls), and **Done**. Escape closes it.
  - **It opens from:**
    - the home sidebar's Settings row;
    - Cmd+, (`app.preferences`, which opens it at General);
    - the Actions palette, `settings.open {section?}`.
  - **An agent's `settings.open`** opens no window, since programmatic calls never open dialogs (AGENTS.md). It returns the settings instead.
  - **The rail lists only the sections built so far.**
- **`settings_general.rs`:** Appearance, then Animations, Shared rooms, Unsaved work, Snapping, Editing, Sharing and Language.
  - **Appearance:** the theme, Auto / Light / Dark through `pref.theme`, the same command as the tab bar's sun and moon; the accent row from B4.
  - **Animations:** Follow system / On / Off, through `pref.motion`, which also keeps the old `reduce_motion` in step.
  - **Shared rooms:** your name (`pref.userName`; shown beside your cursor and on comments).
  - **Unsaved work:** "Keep recovery copies of unsaved work" (`pref.preserveUnsaved`; desktop only). Off stops the recovery tick and the emergency save; existing copies stay. It sits beside "Save files automatically".
  - **Snapping:** "Snap to objects" (`view.snapObjects`) and "Snap to pixel grid" (`view.snapPixel`), the switches that work.
  - **Editing, Sharing and Language:** Preferences' rows, unchanged.
- **The Preferences window is gone;** its rows live in General.
- **`home_settings_row.rs`:** the Settings row, pinned 8 px above the bottom of the home sidebar below a hairline, the same 32 px row as the others, with our own gear. In a short window it stays below the rows, never over them. The three rows above are untouched, as are their positions; the test checks them.
- **The settings struct** gains `motion` and `preserve_unsaved`, both falling back when missing or invalid.

## C3b as built: Toolbar

- **The model (`crates/engine/src/cmd/toolbar.rs`):** `settings.toolbar`, a list of groups of `{tool, on}`. It is empty while the toolbar is the default, so a default toolbar keeps following the app's.
  - **`normalized`** drops unknown and repeated tools and empty groups, adds a missing tool shown in a group of its own, and keeps Move shown.
  - **The commands:** `toolbar.state`, `toolbar.toggle {tool, on?}`, `toolbar.move {tool, group, index?}`, `toolbar.group {tool, with}`, `toolbar.ungroup {tool}` and `toolbar.reset`. Agents reach them through `run_command` like every other command.
  - **Move can't be hidden;** `toolbar.toggle` refuses it with the reason.
- **The toolbar (`toolbar.rs`)** draws the groups from the preferences. Hidden tools are left out, and an empty group is dropped.
  - A group with the tools of a group as it ships keeps that group's name ("Shape tools"); another is named "Tools".
  - A hidden tool keeps its shortcut: the keymap never looked at the toolbar.
  - If the tool a group last showed was hidden or moved, the group shows its first tool.
  - The keyboard dock (F6, Tab, the arrows) and the grab work on whatever groups there are; the a11y tests still pass unchanged.
- **Settings ▸ Toolbar (`settings_toolbar.rs`):** one bordered box per group; one row per tool with its handle, icon, name, key, Show, and the buttons Up, Down, Join above and Own button.
  - **The mouse:** drag a row's handle onto another tool to put the two in one group (after that tool), onto a group's box to move it to the end of that group, or onto "Drop here for a button of its own".
  - **The keyboard:** every move has a button. Up and Down cross into the next group at its edge.
  - Every row shows the same buttons, disabled where they don't apply, so the columns line up.
  - **Reset to default** runs `toolbar.reset`; it is disabled while the toolbar is the default.
- **Screen readers** hear which of the section's buttons are disabled. `widgets::button` itself doesn't say so, so every other disabled button still reads as one you can press; `widgets.rs` is frozen at its size cap, so that fix waits for a split of the file (listed in the final report).

## C3c as built: AI & agents, MCP, Tool access

- **AI & agents (`settings_mcp::ai`):** a line saying agents bring their own models and keys, then the AI tab's Claude Code block (`ai_tab::connection`): the status row, Connect Claude Code and the one connect flow. Nothing to type.
- **MCP (`settings_mcp::mcp`):**
  - **Server:** the status, Restart, the address with Copy (`mcp.copy-address`, new), the version, and a line on loopback-only;
  - **Authentication:** Require authentication (`mcp.toggle-auth`);
  - **Files agents may use:** the folder, then Use default or Choose folder… (`mcp.set-root-dir`);
  - **Connect:** Connect Claude Code, opening the same flow inline.
- **Tool access (`settings_tools.rs`):** "N of M enabled", Restore defaults, a search box, and two groups, the read-only tools and the tools that make changes. Each group has an "All …" switch, then one switch per tool.
  - **The list** is `cmd::mcp_tools::TOOLS` in the engine (name, title, read-only), so Settings can show it without the mcp crate. `crates/mcp/tests/tool_access.rs` checks it against the server's tool definitions, their order and their `readOnlyHint`.
  - **The commands:** `mcp.tools`, `mcp.set-tool`, `mcp.set-tool-group`, `mcp.restore-tools`.
  - **Enforcement:** `Server` asks its backend (`Backend::disabled_tools`) on every `tools/list` and `tools/call`.
    - The app's HTTP backend reads a list it shares with the UI, updated by `McpHandle::configure`.
    - `--attach` asks the app over the control channel (`mcp.policy`, new).
    - Headless `scrim mcp` reads the app's saved `state.json` at start (`settings::saved_mcp`; the settings folder's path moved into the engine, `settings::config_base_dir`, so the app and the CLI share one copy).
    - A backend that can't answer turns nothing off.
  - **Per tool, not per command:** `run_command` and `batch` reach every command. Settings says so: turning those two off is what takes the commands away.
- **Agents can't widen their own access (a fix to C1):** before, an agent could call `mcp.toggle-auth {on: false}` or `mcp.set-root-dir` through `run_command`. Now an agent's call that turns a tool back on, turns authentication off or changes the root folder is refused (`person_only`). Narrowing is still allowed.

## C3d as built: Privacy and Diagnostics

- **Privacy (`settings_privacy::privacy`):** a statement, no switches.
  - Scrim sends no telemetry and has no usage statistics, crash reports or analytics.
  - It goes on the network only when asked: sharing or joining online, publishing, team libraries or plugins on a chosen server, and showing a file on the local network. (Checked against the code: the only network code is `collab-net` and the loopback MCP endpoint.)
  - The MCP server is loopback only; diagnostics stay on the computer.
  - No ping or usage-stats code was added.
- **Diagnostics (`settings::diagnostics`, `cmd/diagnostics.rs`, `settings_privacy::show`):**
  - **The events,** kept in the preferences, made only from a typed `Entry`, so nothing free-form gets in:
    - `reload`: a file reverted to its saved copy (File ▸ Revert);
    - `fontFallback`: the families a file opened without (family names only, at most 30);
    - `mcpServer`: started on its port, or couldn't start;
    - `mcpClient`: a client's name and version as it gave them;
    - `error`: a failed command's id (only a registered id) and its category, never the message;
    - `follow` (Fix L2, Fix N): following an agent paused, and which of its three reasons (`diagnostics::Pause`); the pause cleared, and why (the agent finished, the grace period expired, the agent disconnected, or resumed; `diagnostics::Cleared`);
    - `working` (Fix M0): a client beginning, ending or held in the working state, with the cause (first request, finish_working, session closed, grace period expired, disconnected), the client's name as announced and its session id cut to 8 characters (`diagnostics::{Change, Cause}`);
    - `reveal` (Fix L2): agents' new layers, how many entered and how many were drawn final at once, by reason (`diagnostics::Skip`). One event per burst, once the edits have been quiet 2 s (at most 30 s after the first skip it holds), only when something was skipped.
  - **Not recorded, because the fork has nothing to record:** "font retry" (there is no retry action; fonts are bundled or installed, never fetched) and MCP disconnects (HTTP clients never say they leave).
  - **The section:** Keep for (Don't keep / 1 / 7 / 30 days; a week by default; at most 1000 events), Copy, Save… and Clear; the list newest first.
  - **The commands:** `diagnostics.events`, `diagnostics.copy`, `diagnostics.save {path?}`, `diagnostics.clear` (the person's alone) and `diagnostics.retention`.
  - **The test the brief asks for:** `no_secrets_reach_the_events` drives every kind of event with a token, document text, a path and a prompt on hand, including in error messages, then scans what is kept for them and for any long hex run.
- **Also fixed:** `Settings::sanitized` now runs when the app loads its preferences. Before, it ran only for `scrim mcp`, so an out-of-range value (a port under 1024, an unknown animation mode) was used as written.
- **Skipped, without placeholder pages:** Usage, Media, Cloud storage and a Language section. They are later work; Language stays reachable in General.

## Order of the commits

1. **C0:** this plan.
2. **Fix:** the lock refusal wording.
3. **Fix:** the idle test, waiting on state rather than time.
4. **Fonts:** bundling, the Missing fonts dialog changes, and the font fallback diagnostics event (with C3's diagnostics log).
5. **C1:** the HTTP endpoint and its commands.
6. **C2:** the Connect flow and the AI tab.
7. **C3:** Settings, as several commits: the dialog shell and General; Toolbar; AI & agents with MCP and Tool access; Privacy with Diagnostics.
8. **C4:** verification.

## Verification (C4)

- **Screenshots** are real window captures at about 1920 × 1080, with text visible.
  - **Your input:** you may be using the machine, so I work on copies of files, never your originals.
  - **The capture window:** bringing a window to the front can put it under your pointer. During C0, a click landed in a test window. Nothing was saved and the file is unchanged, but captures should be taken when you're not using the mouse.
- **The real Connect:** it runs `claude mcp add` at user scope against your installed `claude` (2.1.296). It doesn't touch your `~/Scrim` project's `scrim` entry.
- **The diff:** 0 px outside the home sidebar, Settings and the AI tab, using `offscreen_tests.rs`'s comparison against the tree before Stage C. Comparing real captures pixel by pixel isn't stable across runs.

### C4 results so far

- **0 px outside the changed areas** (offscreen, 1440 × 900, light and dark; `offscreen_tests.rs` now also shoots the home screen):
  - **The editor** (no selection, a frame, a text layer, several frames; the toolbar included): 0 px between HEAD and the fonts commit `02356cf`.
    - Against `7ccc97b`, the tree before Stage C, only the canvas differs, by about 4,350 px. That is the fonts commit rendering document text in the bundled Inter, as addendum 2 asked; none of C1–C3 changes a pixel there.
    - So the default toolbar, now drawn from the preferences, is pixel-identical.
  - **Home:** the Settings row, 553 px (light) and 555 px (dark), all at the sidebar's bottom.
    - Otherwise, only two things differ, and both come from the setup: the open file's path text (the comparison tree was checked out elsewhere) and its "Edited …" time.
    - Nothing else differs.
- **Contrast:** `settings_text_meets_4_5_to_1` measures every text colour Settings uses on the dialog, and the rail's current row.
  - Light: the lowest is tertiary text at 6.12:1. Dark: the lowest is tertiary text at 5.16:1.
  - The `success` green (2.90:1 on light) colours only the status dots, always beside their text. It is below 3:1 for a graphic; it's an existing token used elsewhere, so it is reported, not changed.
- **The keyboard:** `settings_can_be_used_from_the_keyboard_alone`. Cmd+, opens it, Tab reaches the rail, Space picks Tool access, Tab reaches a tool's switch, Space turns it off, and Escape closes.
- **Live, in the release build** (`docs/scrim-ports/settings/c4-live-proofs.md`):
  - **Security:** a non-loopback `Host`, or a web page's `Origin`, is refused with 403 and its reason; loopback gets 200.
  - **Tool access:** with `create_node` off, `tools/list` drops from 24 tools to 23, a direct call is refused, and an agent's `mcp.restore-tools` is refused.
  - **The feed:** the person's drawn rectangle and the agent's ellipse are separate entries.
    - That took a fix (`7322bb8`): canvas gestures never reached the feed before, only commands did.
    - Real window input (`ui.input`) is the person's. `ui.pointer` is automation and is logged as the control channel's.
- **`ui.set {windowSize}`:** a new control hook that sizes the window for captures at about 1920 × 1080. osascript has no assistive access here, and the window size isn't persisted.
- **Real window captures** (`docs/scrim-ports/settings/`, indexed in its README):
  - the home sidebar, each of the seven Settings sections, the AI tab empty and showing the command, the Connect confirmation, and the AI tab connected;
  - every one with text, at 1920 × 1050 points (the menu bar takes the rest of the 1080);
  - captured in the seconds the screen stayed unlocked, through `ui.set {windowSize, settings}`.
- **The real Connect** ran against the installed `claude` 2.1.296 (proof in `c4-live-proofs.md`):
  - Run, then the one confirmation, both pressed with real window input; "Scrim was added to Claude Code for every project";
  - `claude mcp list` then reads `scrim: http://127.0.0.1:7600/mcp (HTTP) - ✔ Connected` at user scope, and the AI tab reads "Connected · claude-code".
  - **The app was started in a folder with no local `scrim` entry,** because the flow's `claude` runs in the app's working folder. In `~` it would have found the owner's local entry for the TypeScript Scrim and offered Replace. That entry wasn't touched.
  - **The owner's `state.json`** gained only Stage C's defaults and three diagnostics events; nothing else changed.
- **Fixed after the captures:**
  - The connect flow's success line was drawn in the `success` green, 2.9:1 on light: below 4.5:1 for text. It is now in the text colour.
  - The AI tab's feed lost its edits after a few hundred selection changes (`8c225cd`).
- **The fonts follow-up:** the real BEFORE and AFTER captures of "Brand colour system mockups" (a scratch HOME, so the installed fonts aren't seen): Missing fonts listed Geist, Inter and Source Serif 4 before, and no dialog opens after.
- **Not changed, reported:** `live.rs` draws "Sharing online · …" and "Sharing on your network · …" in the same green, which predates Stage C.

## C.1 (Stage C.1, from the Stage C report)

### C1.2 Port 7680, the registered address
- **The default is 7680** (`settings::DEFAULT_MCP_PORT`). The fallback to the next free port, kept, is unchanged.
- **Telling a chosen port from a defaulted one:** until C1.2 nothing let the person choose a port (no field, no command), so every saved port came from the default or its fallback. C1.2 adds `mcp.set-port {port}` (the person's alone), which records `port_chosen: true`. On load, a file older than `settings_version` 2 whose port is in 7600–7619 (the first default plus its 20 tries) and not `port_chosen` moves to 7680. A chosen one, or any other port, stays.
- **The registered address** (`mcp_registration.rs`): while the AI tab or Settings ▸ MCP shows, Scrim asks `claude mcp get scrim` once per address (an argument list, read-only). When Claude Code's entry points elsewhere it says "Registered address is out of date" with **Re-register** (`mcp.reregister`): one confirmation, then `claude mcp remove scrim --scope <its scope>` and the add, as Connect's Replace does. Afterwards it asks again.
- **A change to C2's rule, for you to overrule:** C2 ran nothing before the confirmation. The check now runs `claude mcp get` (read-only) just by showing the tab; anything that changes Claude Code still waits for the confirmation. The C2 tests now assert that.

### C1.3 Tool access is a boundary
- **`cmd/access.rs`** classifies every command, Read or Write, from registry metadata: undoable commands (`history`) write; 54 commands are listed as Read and 262 as Write. Before, those 316 were unclassified. The full list and the reasoning are in `docs/design/command-access.md`, and a test fails on any unclassified command.
- **The server** (`server.rs` › `carried_commands`) checks every command `run_command` and `batch` carry, a batch's steps at any depth included. It refuses:
  - a command that does what a turned-off tool does (`EQUIVALENT`: `node.create` is `create_node`, the `select.*` changes are `select`, `command.batch` is `batch`, …);
  - a Read command while every read-only tool is off, and a Write one while every tool that makes changes is off;
  - an unclassified command (deny by default).
- **Every route:** this is in `Server`, so HTTP, `--attach` and headless all get it. Each has a test, and refused calls reach neither the window nor the app.

### C1.4 The MCP root folder
- **The default is `~/Documents/Scrim`** (`settings::default_mcp_root`), never the home folder. It is made the first time a request on the app's MCP endpoint names a file (a `path` anywhere in its parameters; `mcp_ui::ensure_root`), and file access is confined to it from then on.
- **Until it exists, the endpoint's file access is confined to nothing.** Before, a folder that couldn't be resolved (a chosen one since deleted, or no home) left the endpoint unconfined: a gap, closed here.
- **A chosen folder** (`mcp.set-root-dir`) is never made or changed. Settings ▸ MCP shows the default as "Documents ▸ Scrim (…), made the first time an agent uses a file".

### C1.5 The success green
- **Light:** `success` is now #0B7A3E: 5.43:1 on the panel and menu, 4.93:1 on fields as text, and 4.15:1 or more as a mark on the selected row and the canvas. The old #14AE5C was 2.90:1.
- **Dark:** #14AE5C already reads at 4.74:1 or more on every dark surface, so it stays.
- **`live.rs`'s sharing status** ("Sharing online · …", "Live · …", "Joined …") was already drawn in `success`, so it now reads at AA with no code change. The connect flow's success line went back to the text colour in C4; it could use `success` again.
- **The test:** `theme_tokens_tests.rs` › `every_semantic_colour_reads_in_both_themes` checks every semantic colour in both themes:
  - text roles (success, danger, warning, accent text and the three text greys) at 4.5:1 on the panel, menu and field;
  - mark roles (the same status colours, accent, focus ring and both icon greys) at 3:1 on those surfaces plus the selected row and the canvas.
- **Pixels outside Stage C** change in light theme wherever `success` appears: Dev Mode's "Ready for dev" pill, the compare view's "added" colour, the home screen's shared-file mark and the live presence dot.

### C1.7 Overused font by family
- **`ai.findings`** gives the `overused-font` group a `families` list: [{family, name, count, findings}], the most used first. The lint names the family in each finding's `detail`. `ai.selectFinding` takes `family` to select one family's layers, or one of them with `index`.
- **The AI tab:** opening "Overused font" shows one row per family with its layer count; opening a family shows its layers, and its row selects them all. Every other rule is drawn as before, at the same positions.
- **Brand colour system mockups, report only:**
  - the Design lint's rule rows are 13, before and after;
  - opening "Overused font" showed 8 layer rows plus "and 561 more" before;
  - it now shows 2 family rows: Geist 499 and Inter 70.

### C1.6 Verification gaps closed
The record is `docs/scrim-ports/settings/c1/README.md`, with its captures.
- **Dark-theme captures** of all seven Settings sections.
- **Replace and the old figcraft entry's removal,** run against the installed `claude` with a temporary `CLAUDE_CONFIG_DIR`, so the owner's config was never touched. `claude mcp list` before showed `scrim` at 4848 and `figcraft`; after, only `scrim` at the live address, connected. The registered-address check (C1.2) showed "out of date" first.
- **WASM size:** 48,780,116 bytes before the fonts and 48,676,495 after (−0.10 MB). The fonts aren't in the web build, so nothing loads lazily.
- **Tool access live,** headless and through `--attach`: `create_node` is hidden; it is refused directly and through run_command, batch and a nested batch; another tool's command runs.
- **The keyboard alone:**
  - Settings opens through the menu shortcut (`ui.menu.invoke`, since Cmd+, is the native menu's).
  - Tab and Space reach Diagnostics ▸ Save…, which opens the system's save panel.
  - This found a bug, fixed in `8cb4790`: Tab walked out of Settings into the panels behind it. Settings is now modal.
