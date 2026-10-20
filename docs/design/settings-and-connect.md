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
   - **A version:** `state.json` gains `"version": 1`. A missing or invalid key falls back to its default (serde `default`, plus per-field checks on load); an unknown later version is read as far as it goes, never refused.
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
