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
- **No selection:** the tab says so, and offers the current page's frames.
- **Commands:**
  - `code.setFormat {format}`, which also reports what is selected;
  - `code.get {format?, ids?}`, which returns the code: an agent reads exactly what the person sees;
  - `code.copy {format?}` and `code.copyStandaloneHtml {ids?}` (the clipboard is the UI's: headless they return the text).

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
