# Right panel redesign (Stage B): the plan

**Stage B of the rename task, 19 Oct 2026.** Stage A (the rename) is done; its record is `docs/design/rebrand.md`. This file is B0. Each later step (B1–B5) is its own commit and stop point, and updates this file where it decides something.

**The person designs mostly *with* agents.** Every mode switch, tab switch, toggle and new button is therefore a command in the registry, reachable over MCP (`run_command`, and over the control channel `engine.execute`), with a parity entry. Nothing is UI-only. Previews never write to the document.

## What is there now (BEFORE)

Screenshots are in `panel-redesign/before/`, taken from the packaged app at 9d7fdc6 on `examples/ridgeline-app.figcraft`, 1440 × 900, the right panel 240 px wide. Each state is shot in light and dark: `none-*`, `frame-*`, `text-*` (the "9:41" text), `multiple-*` (three frames), and `inspect-frame-*`, the Inspect tab B2 draws from.

- **The header row:** who is here, Present (with its menu), Share.
- **The tabs row:**
  - the tabs **Design** and **Prototype** (`app.ui.right_tab`), switched by clicking only: there is no command, and `ui.set {rightTab}` is the control channel's test hook;
  - the **zoom** dropdown at the right, whose items are commands (`view.zoom*`, `view.pixelPreview`, the view toggles).
- **A third panel state, Inspect (Dev Mode),** is reached from the toolbar's mode switch: Draw · Design · Prototype · Inspect. It holds the handoff status, annotations, properties, assets, and a **Code** section: six generators (`codegen::CodeLang`: CSS, HTML + CSS, React, Leptos, SwiftUI, Compose), px or rem, and Copy.
- **The tabs draw in `right.rs`,** which is frozen at 2,326 lines. New code goes in new modules.

## The target

```
┌──────────────────────────────────┐
│ (people)            ▷ ▾  [Share] │  header: unchanged
├──────────────────────────────────┤
│ [ Design | Animate ]      29% ▾  │  mode switch + zoom dropdown
├──────────────────────────────────┤
│ Design  Prototype  Code  AI      │  tabs (Design mode only)
├──────────────────────────────────┤
│ …the tab's content…              │
```

- **Mode switch (B1):** a two-segment control, **Design | Animate**, with the zoom dropdown moved onto its row.
  - **Design** mode shows the tabs.
  - **Animate** mode is a **placeholder only**. The panel says that animation comes in a later stage and offers to go back to Design. No timeline, no keyframes, no playback, no export; nothing reads or writes the document.
- **Tabs (B1):** Design, Prototype, Code, AI.
  - **Design** is today's Design tab, with "the Scrim touch": Port 1's tokens, nothing new invented. The active tab is marked the way Port 1 marks the selected layer, a 2 px accent edge under the label, with the neutral selection fill; section headers stay on Port 1's 11/12/13 type scale and 28 px rhythm.
  - **Prototype** is today's tab, **untouched** (its content and Smart Animate included).
- **Inspect stays** where it is (the toolbar's Inspect mode, `ui.set {rightTab: "inspect"}`). In Inspect the tabs row shows no active tab. Dev Mode handoff isn't the Code tab's job.
- **Width:** the four labels fit 240 px in English and Spanish ("Diseño · Prototipo · Código · IA"). A locale whose labels don't fit draws them with less padding, then clips the last label, which keeps its full name as its AccessKit name and tooltip. B5 checks the longest catalog.

### Commands (B1), each with an MCP route and a parity entry

| Command | Does | Returns |
|---|---|---|
| `panel.mode.design`, `panel.mode.animate` | Switch the panel's mode | `{mode, tab}` |
| `panel.tab.design`, `panel.tab.prototype`, `panel.tab.code`, `panel.tab.ai` | Switch the tab, and the mode to Design | `{mode, tab}` |

- **How they reach the UI:** they are engine commands that ask the UI through `Session::request_ui`, as `view.assets` does (`cmd/view.rs` › `ui`). Over MCP they run through `run_command` when attached to the app. Headless, they answer `{mode, tab}` from the session's own copy and draw nothing.
- **Their state** is kept in the session, so an agent can read it back. `ui.inspect` reports `panelMode` and `panelTab`.
- **Parity:** catalog entries under View ▸ Panel, as Scrim's own items (`catalog::OWN`, which `cargo xtask parity` doesn't count), and a row in `docs/ui-parity.md`.
- **Remembered per window:** each window keeps its own mode and tab. The last one chosen is saved in the preferences, so a new window or the next start opens with it. `crates/engine/src/lib.rs` is frozen at its length, so the preference's lines are paid for by moving code out, as Port 3 did.
- **Keyboard and screen readers:** the mode switch is two AccessKit toggles, named "Design mode" and "Animate mode"; the tabs keep `a11y::tab` with their names. F6 keeps its order (toolbar → layers → canvas → properties), and Tab moves through mode switch → zoom → tabs → content.

### Code tab (B2)

- **From the existing generators** (`codegen::CodeLang`, `dev_mode::code`): a format dropdown with the six languages, px/rem for the web ones, the code, **Copy**, and **Copy standalone HTML**.
- **Copy standalone HTML** copies a complete page (`<!doctype html>`, the stylesheet inline, no external requests) for the selection, built on the HTML exporter that Port 2's fidelity check measures.
  - **As built (B2):** the page is for **one layer**, the first selected. Two exported fragments in one page could collide on class names. The stylesheet moves into the head, and the layer's name, escaped, is the title.
  - **Nothing fetched:** the page carries `Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:`. Images are already inlined by the export as `data:` URIs, so even a missing image's fallback `url(image.png)` fetches nothing.
- **No selection:** the tab says so, and offers the current page's frames.
- **Commands** (as built, B2: `crates/engine/src/cmd/code.rs`):
  - `code.setFormat {format, unit?}` switches what the Code and Inspect tabs show;
  - **reading the code** is the existing `inspect.code {id?, language?, unit?}`, so no second getter: an agent reads exactly what the person sees;
  - `code.copy` (`inspect.code`'s parameters and result) and `code.copyStandaloneHtml {id?}` ask the UI to put the text on the clipboard. Headless, they return the text.

  The same table rules apply: MCP route, parity entry, AccessKit names.
- **Previews only.** Nothing here writes the document.

### AI tab v1 (B3)

No chat, no API keys, no provider settings. The tab is about the agents already working on the canvas.

1. **Connection:**
   - **The control channel's state:** "Agents can connect: control channel on 127.0.0.1:PORT", or "The agent channel is off".
   - **The copyable connect command,** from `agents::connect_command`. It is attached (`scrim mcp --attach PORT`) when the channel is on, headless otherwise. `crates/mcp/tests/connect_docs.rs` already ties the text to `docs/mcp.md`.
   - **Starting the channel from the app** (a new listener) is **not** in v1. It needs its own security review; today `--control` is the way.
2. **Activity feed:**
   - Port 3's change log (`history.changes`, MCP `get_changes`), newest first: who (person, or the agent and its route), what (label), when (revision), and which layers.
   - **Clicking an entry** selects its layers that still exist (`select.set`). Removed ones are listed as removed.
3. **Undo per agent action:**
   - An agent entry shows **Undo** when it is the latest step on the undo stack; it runs `edit.undo` on that step.
   - Under later edits it is disabled, with the reason ("Later edits came after this; undo them first"). The history is one stack, and v1 doesn't undo out of order.
4. **Lint findings:** Port 6's `lint.run`, grouped by rule with a count badge per group. Clicking a finding selects its layers. Profile and ignore stay `lint.setProfile` / `lint.ignore`.
5. **Lock toggle:** "Keep agents off the selection" runs `object.toggleLock` on the selection. Port 3 makes locks bind agents, so a locked layer is refused to them with a reason, and the person's own edits are unchanged.
   - A document-wide "agents read-only" switch would be a new engine rule: listed under Later.
6. **Commands:** `ai.refresh`, `ai.selectChange {revision}`, `ai.undoChange {revision}`, `ai.selectFinding {rule, index}`, and `ai.toggleLock`, besides the existing ones they call. Every badge and list is read from the same commands an agent can run.

**As built (B3)** (`crates/engine/src/cmd/ai.rs`, `crates/ui-egui/src/ai_tab.rs`):
- **The commands:**
  - `ai.status`: the channel (on or off), the port, and the connect command;
  - `ai.copyConnectCommand`;
  - `ai.selectChange {revision}`: selects the layers that still exist, and reports the removed ones. Pages, and parents listed only because a child changed, are left out;
  - `ai.undoChange {revision}`: an agent's change only, while it is the last entry in the log and the top undo step is its own ("Agent: …"); otherwise refused with the reason;
  - `ai.findings`: `lint.run` grouped by rule, largest group first;
  - `ai.selectFinding {rule, index?}`.
- **Plan items dropped:**
  - **`ai.refresh`:** the tab re-reads on every frame it draws. The findings are cached per document revision.
  - **`ai.toggleLock`:** the tab runs the existing `object.toggleLock`, already a command.
- **The port:** the app records its control port on the session (`agents.control_port`) when `--control` binds.
- **Reads have no side effects.** The tab calls the read commands' functions directly, not through `Session::execute`. `execute` would end a live text preview and log every per-frame read in the command journal (`ai_tab_tests.rs` › `reading_the_tab_every_frame_leaves_no_trace`).

### Appearance in Preferences (B4)

- **Extend what exists:**
  - the theme commands `view.themeLight`, `view.themeDark` and `view.themeSystem`;
  - Port 1's accent derivation (`accent.rs`: `derive`, `on_fill`, six presets, `DEFAULT_ACCENT`).
- **Theme:** Dark / Light / System, as now.
- **Accent:** the presets, including **#ff5a1f**, and a custom hex value.
  - **Text on the accent is chosen automatically** to reach AA (4.5:1). `accent::on_fill` already does this: near-black on orange, white where that is the one that reads.
  - A custom colour the derivation refuses is refused with its reason, never forced.
  - **The default stays in one constant,** `accent::DEFAULT_ACCENT`.
- **Commands:** `pref.accent {value | preset}` and `pref.theme {theme}`, the latter alongside the `view.theme*` ones.
- **Command palette:** one exists (the Actions palette, `ui.set {palette}`), so B4 adds the new commands to it and builds no second palette.

**As built (B4)** (`crates/engine/src/cmd/appearance.rs`, `crates/ui-egui/src/appearance_prefs.rs`):
- **The default, written once:** `appearance::DEFAULT_ACCENT` in the engine. `accent::DEFAULT_ACCENT` and `theme::ACCENT` in the UI read it. The preference saves the default as empty, so changing the constant reaches everyone who never chose a colour.
- **The commands:**
  - `pref.accent {preset? | value?}`: neither means the default;
  - `pref.theme {theme}`, beside the existing `view.theme*`;
  - `pref.appearance {}`, which reads the theme, the accent, the matching preset and the six presets.
- **Checked in the engine:** what doesn't depend on a theme, namely a hex colour, not too grey, light or dark (Port 1's first rule).
- **Derived in the UI,** per theme, by `Tokens::with_accent` through Port 1's `accent::derive`: the mark (also the focus ring and coloured text), the fill, its hover, the label on it (4.5:1, white or near-black) and the tint. The default keeps Port 1's hand-checked tokens exactly.
- **Preferences:** the six swatches (AccessKit toggles named "Orange accent" and so on), a hex field, Reset, and a sample primary button with its label's measured contrast. All of them run `pref.accent`.
- **Not yet following the accent:** the canvas chrome (selection outlines, handles, guides, about 150 uses of the constant `theme::ACCENT`, most in the frozen `canvas.rs`) stays orange. Moving it to the tokens is a follow-up: it needs room in `canvas.rs` and a pass over each overlay's contrast on the canvas.

### Verification (B5)

- **CI**, including the locale check.
- **AccessKit names** for every new control, by test.
- **AFTER screenshots** at three widths (the 240 px default, the narrowest and a wide panel) in `panel-redesign/after/`.
- **Contrast checks:** every new text pair and the accent label, in both themes.
- **Nothing else changed visually:** compared against the BEFORE shots outside the panel.
- **An agent check over MCP only:** switch mode and tabs, read the code, read the feed, select a change, undo an agent step, lock.

## Later stage: Animation (decided now so B1 doesn't box it in)

**Out of this task, completely:** no timeline strip, no keyframe model, no playback, no export. The Prototype tab and its Smart Animate are untouched.

**What the placeholder commits to:**
- `panel.mode.animate` exists, so agents and tests can reach the mode now.
- The Animate panel draws only its note. Its content area belongs to the later stage, which replaces the placeholder in one module.

**Decisions left for the animation stage** (recorded, not taken):
1. **The model:**
   - keyframed properties on layers, in the document format (a format version step, with `.figcraft` files from before it still opening);
   - or animation derived from Smart Animate between frames;
   - or both.
2. **The timeline's place:** a strip under the canvas (the canvas gives up height), or inside the right panel. The placeholder doesn't assume either.
3. **Playback:** in Present, the canvas, or both. Reduced-motion preferences apply (`Prefs.reduce_motion`).
4. **Export:** CSS animations (they fit the export guarantee and `cargo xtask fidelity`), video, or Lottie. Each needs its own fidelity check.
5. **Agents:** keyframe commands in the registry from the start, with the same MCP and parity rules as everything here.
6. **References:** OpenPencil and Jitter are references for structure only. Their icons, strings and assets are never used; we draw our own.

## Rules that apply to every B step

- `cargo xtask ci`, read in a separate step, before every commit and push; new files staged first. A timing test that fails under load is rerun alone, then the suite once, and both results go in the commit message.
- Every new string goes into every locale catalog the completeness check requires (Spanish).
- Files over 1,000 lines are frozen; new code goes in new modules, and `cargo xtask layers` holds.
- No `unwrap`, `expect` or `panic` in shipped code. Previews never write to the document.

## B5: verification (19 Oct 2026)

**CI:** `cargo xtask ci`, the locale checks included (Spanish has every new string and command). The result is in the B5 commit message.

**AccessKit names, by test:**
- the mode toggles "Design mode" and "Animate mode";
- the four tabs, with the selected one marked, and the panel region named after what it shows;
- the AI tab's buttons (connect command, each feed entry, "Undo …"), its rule rows and its checkbox;
- the six accent swatches ("Teal accent" and the others).

The tests are `panel_view_tests.rs`, `ai_tab_tests.rs` and `appearance_prefs_tests.rs`. F6 still cycles toolbar → layers → canvas → properties (`a11y_tests.rs`, unchanged and passing).

**AFTER screenshots** (`panel-redesign/after/`, 1440 × 900, light and dark):
- the four BEFORE states: `none`, `frame`, `text`, `multiple`;
- `prototype-frame`, `code-frame`, `ai`, `animate`;
- Preferences with the teal accent (`preferences-teal`), and the Code tab in teal (`code-frame-teal`);
- **three panel widths**, 240 (the default and the narrowest), 360 and 480: `width-*-frame` and `width-*-ai`.

**How the screenshots were made.** The screen was locked for the whole of Stage B, so no window could draw (`CGSSessionScreenIsLocked`). The shots come from `crates/ui-egui/src/offscreen_tests.rs`: the app draws its frames headless, egui tessellates them, and the triangles are filled in software as the GPU fills them. Run it with `SCRIM_SHOTS_DIR=<dir> cargo test -p figcraft-ui-egui --lib offscreen -- --ignored`. It is close to the app, but not pixel-identical:
- **Text inside the designs is missing:** tests don't index the system fonts.
- **A glyph the bundled UI font lacks draws as a box,** such as the `▸` in "Share ▸ Online"; the app gets it from the system fonts.
- **Edges differ slightly:** there is no MSAA.
- **The window has an "Untitled" tab:** a new session starts with one.

Retake the shots in the app once the screen is unlocked; the offscreen ones are for review, not a baseline.

**Nothing else changed visually.**
- **The method:** the same offscreen renderer drew the four BEFORE states from the tree before the redesign (5923921, in a temporary worktree). `compare_outside_the_right_panel` then counted the pixels that differ outside the right panel.
- **The result:** 0 in all eight shots (four states × two themes).
- **The check works:** given two different states, it reports 17,855 differing pixels.

**Contrast:** `panel_view_tests.rs` › `the_new_panels_text_and_marks_read_at_aa` checks every text and mark the redesign draws, for all six presets in both themes, at 4.5:1 or better:
- the tab and mode labels on the panel, the field and their fills;
- the accent edge, and the agent names in the feed (also on hover);
- the badge counts and the notes;
- the label on the accent.

`every_accent_reads_at_aa_in_both_themes` adds four custom colours (one of them a dark navy and one a bright yellow). The sample in Preferences shows its own ratio (teal on light: 6.2:1).

**An agent check over MCP only:** the release app was started with `--control`, and `scrim mcp --attach PORT` drove it (all checks passed):
- `serverInfo` is `scrim`;
- `panel.mode.animate`, then each `panel.tab.*`, read back through `inspect_ui` (`panelMode`, `rightTab`);
- `code.setFormat`, `inspect.code`, and `code.copyStandaloneHtml` (a doctype and the CSP);
- `ai.status` (channel on, the attach command with the port);
- the agent renames a layer, the change shows in `get_changes` as the agent's, `ai.selectChange` selects it, and `ai.undoChange` undoes it (the name comes back). A second undo is refused: "later edits came after this change".
- `ai.findings` (117 findings in 5 rules, largest first) and `ai.selectFinding`;
- the agent's `object.toggleLock` is refused, because locks are the person's (Port 3);
- `pref.accent` (teal, then a grey colour refused, then the default) and `pref.theme`.

**Left for later:**
- **The canvas chrome follows the default orange,** not a chosen accent (B4, above).
- **Port 3's refusal wording:** an agent trying to lock an unlocked layer is refused with "… is locked", which describes the state the edit would make.
- **A timing test fails under heavy load:** `idle_tests::the_prototype_curve_preview_plays_then_rests` failed on B0's docs-only commit and twice during B3, and passed alone each time. Its cause isn't found yet (`docs/design/panel-redesign.md` › this section; the B0 and B3 commits).
- **The screenshots should be retaken in the app** when the screen is unlocked.
