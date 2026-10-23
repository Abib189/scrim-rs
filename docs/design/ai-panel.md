# The AI panel (Fix T4, 22 Oct 2026)

**Why:** the AI tab had become a long list: the agents' status, a warning, about sixteen Connect rows, Activity, Design lint and the lock. The composer was squeezed at the bottom. The person wants the AI tab to be a chat, and every connector in Settings. The reference was a screenshot the person supplied.

## The layout, top to bottom (Fix X, 23 Oct 2026)

Rebuilt to Scrim's AI panel design, in dark and light; the reference was frames 211, 257, 321 and 351 of the design file the person gave. The panel is flexible, 240 to 480 px wide; the design's 336 px is an example. Every colour is a role from the theme (`ai_panel_theme.rs`, `ai-panel-theme.md`), so the person's accent applies.

- **The header** (`ai_panel.rs`), 48 high, its rule `t.border`:
  - **The title:** the conversation's title (a new chat reads "New chat") in SemiBold 14, 16 from the left, with a 9 × 5 chevron 6 after it, which opens the conversations' popover. A title too long for the room is cut with "…" and the chevron follows what shows (Fix Z3); the whole title is the tooltip and in the accessible name ("Conversation: {title}").
  - **Two buttons,** 28 × 28, radius 8, 8 apart, 16 from the right, on `t.pressed`:
    - **Follow agents** (the crosshair, `camera.follow-agents`): on is the accent on its tint; off is `t.icon`; paused while an agent works shows the amber dot (Fix N). The tooltip has following's status.
    - **New chat** (+, `chat.new`): disabled while a run goes (Fix U3).
  - **No Share button or avatar:** they aren't Scrim features.
- **The working banner** (`ai_run_card.rs`), only while a composer run goes, directly under the header's rule:
  - 36 high, `t.field`;
  - three 6 px dots at 16, 26 and 36 in the working indicator's rhythm (opacity 1, 0.65, 0.35, moving; still with reduced motion);
  - "{agent} is working" at 52, and the time (m:ss) 16 from the right.
- **The chevron's popover:**
  - "Search conversations…", matching titles and messages here only, ignoring case.
  - "This document | All documents" (`chat.scope`).
  - The conversations, newest first, the one shown checked (`chat.open`). Under All documents, each also names its document.
- **The body:**
  - **Empty:**
    - a 64 px `t.pressed` disc with the accent's 32 px core;
    - 24 below it, "Describe what you want to create or change." (SemiBold 20);
    - 10 below that, "Your agent edits the canvas while you watch." (13, 140 %, secondary).
    - The block is centred between the header's rule and the status line, 12 above the middle. No suggestion rows.
  - **A conversation:**
    - **Your message:** a right-aligned bubble, radius 14, at most the width less 56, 16 from the right, 14 × 11 inside, 24 below the banner or header, 18 above what follows.
    - **The bottom fade** (Fix Z4): a 12 px fade from transparent into `t.panel` at the transcript's bottom edge, only while there is more below (scrolled up); none at the bottom or when it all fits.
    - **The agent's replies and notes:** text, 16 from each side. A reply is drawn from its Markdown (Fix Z2, decision 24); notes, failures and your own message stay plain text.
    - **The run's card, after the message that started it** (`ai_run_card.rs`):
      - **The box:** the width less 32, radius 12, 16 inside, `t.field` with a 1 px `t.border_strong` edge.
      - **The heading:** "Run · 42 s" (then "Done · …" or "Stopped · …") in SemiBold 13, and at its right "Undo is one step".
      - **The rows:** 36 apart, the run's agent edits from the change log (decision 15), the newest 5, with "{n} earlier" above when there are more. A done row is a `t.success` circle with a check that reads on it; while the run works, the newest is active (a 2 px accent ring and dot, its label SemiBold). No pending rows (decision 16).
      - **The bar:** while it works, a 4 px indeterminate bar (a 30 % accent segment moving along the `t.border_strong` track; still with reduced motion), gone when the run ends. No percentage (decision 16).
      - **Two buttons:** 34 high, radius 10, 8 apart, 16 below the bar or the rows. Select layers runs `chat.select-run-layers`; Undo run runs `chat.undo-run`, held while the run goes (decision 17).
        - **After Undo run** (Fix Z7, decision 26): the heading reads "Undone · 42 s" and the rows are dimmed (`t.text_secondary`, an empty ring). Select layers is off ("Those layers were removed by Undo"); Undo run becomes Redo run (`chat.redo-run`), off once something changed since ("Something changed since: Redo is no longer one step").
        - **An off button** is drawn in the `disabled` token, can't be focused or pressed, and says why in its tooltip.
        - **A refused press** shows its words on one line under the buttons (`t.text_secondary`, 12 px, wrapped), gone on the next action; no toast.
        - **From a 300 px panel** they sit side by side, equal.
        - **Below 300 px** they stack, each the full width (Fix Y2). They also stack wherever a label wouldn't keep 12 px from its button's edges in half the width, as a longer language's might.
- **The status line**, always 10 above the composer: a 6 px dot at 20 and Inter 12 at 32. "Following is on" with a `t.success` dot, or "Following is off" with a secondary one. The grace hint's line and the missing-CLI line stay above it.
- **The composer**, pinned to the bottom, 16 from the panel's left, right and bottom, with no rule above it (`ai_composer.rs`, `ai_chips.rs`):
  - **The box:** radius 16, `t.field`, its edge `theme::field_stroke`; 16 left and right and 14 top and bottom inside; at least 88 high, growing with the text.
  - **The field:** Inter 14, "Describe a change…".
  - **A 32 px row under the field,** 8 apart:
    - the agent chip (a pill with its name and an 8 × 4 chevron, which opens the agent menu: the agents, then "Agent settings…");
    - the Selection chip (`chat.add-selection`): a round 32 px icon chip (Fix Z6), a 14 × 14 rounded dashed square with a pointer arrow in it;
    - an image chip (`chat.attach-image`), only for an agent whose CLI takes one: a round 32 px icon chip, a picture;
    - **the icon chips:** `t.selected_row`, `t.pressed` hovered, the focus ring; the icon in `t.icon`, stroke 1.5 (`ai_icons.rs`). Their accessible names and tooltips are unchanged: "Add the selection to the prompt" and "Attach an image".
    - Send at the far right: a 36 px `t.accent_fill` circle with an arrow in `t.accent_on`, which is Stop (a 16 × 16 square, radius 3) while a run goes.
  - **Narrow:** the agent's name shortens with "…" (decision 14); the icon chips stay 32 px.
  - **Keys:** ⌘↩ (Ctrl+Enter) sends; Esc and Stop stop a run. Why Send can't go is its tooltip.
  - **R5 and S1 are unchanged:** the commands are the person's, the first-use dialog, no keys stored, `USER`/`LOGNAME` in the child's environment, the time cap, and Undo one step per edit.

**`ui.inspect` › `agent.aiPanel`** reports what was drawn:
- **the header:** the title as drawn (`titleShown`), its tooltip, its rectangle and the chevron's (Fix Z3); the Follow button's state, rectangle, tooltip and `followSetting`; New chat's state; the header's rectangle;
- **the body:** the popover, the empty state and its line, read-only, the banner, the bubbles, the replies (each block's kind, its text as shown and its rectangle, Fix Z2), the bottom fade's rectangle while it shows (Fix Z4) and the card (heading, note, its state, rows with their state, bar, buttons, whether Undo run can be used, rectangle; since Fix Z7 each button's label, whether it can be used and its tooltip, and the refusal line);
- **the composer:** its controls (since Fix Z6, `selectionChip` and `imageChip`, each with its icon, name and rectangle; `compactChips` is gone), rectangle, status line, hint line, missing-CLI line and status;
- **`theme`:** the colour drawn for each role (`ai-panel-theme.md`).

## Where everything else went

- **Settings ▸ Agents** is the "AI & agents" section, renamed. Its id stays `ai`, and `settings.open {section: "agents"}` opens it too. Top to bottom:
  1. **The connections,** the AI tab's until now (`settings_agents.rs` › `connections`, with the same commands): the MCP endpoint's state and its clients; Claude Code's Connect; Claude Desktop's Register and Re-register; and every other client with its status, registration date, "Tools not yet used live" or "used live" line, and Connect.
  2. **Agents at work,** from Settings ▸ General: following, the reveal and its speed, the dots, and the grace. While a "grace too short" hint has been said this run, its words stay beside the grace picker.
  3. **Describe a change** (R5d).
- **The Inspect tab's end:** Activity, Design lint and "Keep agents off the selection", unchanged, with the same `ai.*` and `object.toggleLock` commands (`ai_tab::review`).
- **Nothing was deleted.**
  - **Captured first:** every labelled row of the old AI tab, from the old code before the change (`ai-panel-old-ai-tab.txt`, at `43aace8`).
  - **Checked:** a test opens Settings ▸ Agents and finds each connector row there, role and words, and that each row's command is still in the registry.

## Conversations

- **Kept on this computer,** in the settings folder: `conversations.json`, owner-only (0600), written to a temporary file and renamed. They're keyed by document (the file's path, or `unsaved:{uid}`) and never in the document file.
- **No keys,** and nothing of them reaches Settings ▸ Diagnostics, which keeps only the agent's name and how long a run took, as before.
- **A new chat is made on its first send,** titled from that prompt (48 characters at most). The newest 200 conversations are kept, with each one's last 500 lines.
- **Another document's conversation opens read-only:** a note says "From {document}: open that document to continue.", and Send is refused, because a run belongs to its document.

## Decisions (overrule any)

1. **Activity, Design lint and the lock moved to the end of the Inspect tab.** The brief asks for a chat-only AI tab and doesn't say where these go. The Inspect tab is the panel's existing review view and needs no new navigation. Dev Mode does turn on hover redlines. A left-rail entry of their own would be the alternative.
2. **Settings ▸ Agents is the old "AI & agents" page, renamed,** with the connections first. The page was long, and the connections are what people come to it for.
3. **Agents at work moved from General to Agents,** so the grace hint can sit next to its picker, as the brief asks.
4. **The grace hint:**
   - **In the AI panel,** it is one dismissible line above the composer while it is pending. Dismissing it, or its link to the setting, ends it.
   - **In Settings ▸ Agents,** its words stay beside the picker for the rest of the run.
   - **Nowhere else.**
5. **A missing or signed-out CLI** (Fix Y1): one visible line the person can act on, 8 above the status line, in Inter 12 secondary text, with an "Agent settings" link in `t.accent_text` that opens Settings ▸ Agents (`settings.open`, as the agent menu's last item). The link moves to the next row whole rather than breaking.
   - **Not installed** is known at once: "{agent} isn't installed." (the agent picked, else Claude Code).
   - **Not signed in** is only known after a run stops before it began: "{agent} couldn't start: is it signed in?", until a run starts or the person dismisses it (× , `chat.dismiss-hint {hint: "agent"}`).
6. **The selection button writes "Selected: Name (id), …" into the prompt,** where the person sees and can edit it. R5's automatic context line (locked decision 8: the page and the selection) stays.
7. **The image is data on the agent's profile** (`image_args`):
   - **Codex:** `-i/--image`, read from `codex exec --help` 0.163.0. It goes right after `exec`, so the next flag ends its list and the prompt isn't read as a file.
   - **Claude Code** runs with no built-in tools (`--tools ''`), so it can't open a path: the button is hidden for it.
   - **An image path** goes only to the agent's argument list: never the document, and never Diagnostics.
8. **The AI panel's own actions are commands,** the person's alone, like R5's. The new commands are `chat.new`, `chat.open`, `chat.scope`, `chat.add-selection`, `chat.attach-image`, `chat.agent`, `chat.dismiss-hint` (`{hint?: grace|agent}`, Fix Y1: the grace hint by default, or the line about an agent that couldn't start) and `chat.grace-setting`. The last is "Open the setting", which was a direct call before. Typing in the search field is window state, not a command.
9. **New chat waits for a run to end:** the run's lines belong to its conversation. Since Fix U3 the + button shows it is disabled and says "Stop the run first", and `chat.new` refuses with those words instead of doing nothing. The old Clear button is gone, and `chat.clear` now means a new chat, refused the same way. The conversation shown is kept.
10. **Conversations of an unsaved document** are keyed by its window's document number. After a restart that document is a new one, and its old conversations show as another document's.
11. **The new icons** are drawn in `ai_icons.rs` (the crosshair and, since Fix X2, the chevrons), because `icons.rs` is over the size limit. Since Fix X3, Send's arrow and Stop's square are drawn with the button in `ai_chips.rs`, and the gear is gone (decision 14).
12. **The old AI-tab Follow row and its Resume** are gone. The header's button does both, and following's status is in its tooltip.
13. **U3's dimmed colour is a new token, `disabled`,** equal to `menu_dim` (`text_tertiary`) in both themes: the codebase had no token named for disabled controls, and disabled menu items already use that colour. Like them it isn't held to 3:1 (WCAG exempts inactive controls).
14. **The composer's gear moved into the agent chip's menu** (Fix X3): "Agent settings…", the menu's last item, opens Settings ▸ Agents. The design has no gear. Since Fix Z6 the Selection and image chips are round icon chips at every width, so the agent's name keeps its room (before, they showed their words from about 300 px).
15. **The card's rows say what the run did, by name** (Fix Z1, `chat_run::rows`, replacing Fix X4's command labels). Each command or batch of the run gives rows from the layers it touched, as the change log captured them then (`agents_touched.rs`: created, edited, moved, deleted, with the layer's name at that moment):
    - **Created:** "Created {name}" for each frame, section, group or layer right on a page it made, with " ({n} layers)" for the layers it made inside. Its other creations are "Created {name}" for one, else "Created {n} layers".
    - **Edited:** "Edited {name}" for one layer, else "Edited {n} layers". A single move into another parent is "Moved {name} into {parent}".
    - **Deleted:** "Deleted {name}", or "Deleted {a} and {b}", else "Deleted {n} layers". The name is kept from before the deletion, so the row reads the same after the layer is gone.
    - **A batch the agent labelled with a sentence** (it has a space and isn't one of the registry's labels) shows that label.
    - **Rows that say the same, one after another,** are merged with a count; the newest 5 show, with "{n} earlier" above. Long rows are cut with "…".
    - **Names are the person's own content** and are shown. No prompt, path or key reaches a row.
    - A batch's own steps aren't rows: the batch's are, as one.
16. **No pending rows and no percentage:** the agent has no plan to measure, so the card never shows work it doesn't know about. While it works, the newest row is the active one and the bar is indeterminate.
17. **"Undo is one step" is true:** the history keeps whole documents, so `chat.undo-run` puts back the document as it was when the run began, as one undo step (and one Undo brings the run back). It is refused:
    - while the run goes ("Stop the run first", U3's words), so the button is held then;
    - when the person edited the document since the run began, or an agent did after it ended (those would be undone too);
    - in another document, and in a file shared live.
    It is noted in Diagnostics ("Claude Code run undone: 7 edits"). `chat.select-run-layers` selects the layers the run touched that are left.
    Since Fix Z7 each refusal is "not available now" in fixed words, never "invalid parameters" (decision 26).
18. **The empty state's core is the accent's mark (`t.accent`), not its fill:** the mark is the colour derived to read at 4.5:1 on the panel; the fill is for things with a label on them.
19. **Follow off is no longer dimmed** (Fix X1 overrules V3's look): the crosshair is `t.icon` on the rest fill, and the status line and tooltip say "Following is off".
20. **The Follow icon on its tint is the accent moved until it reads** (`readable_mark`, Fix X5): the default orange came out at 4.499:1 on its own tint in light; the hue is kept.
21. **The grace hint's words are secondary text,** no longer `t.warning`: the role table has no warning colour. The paused dot on Follow keeps `t.warning`.
22. **Nothing is written under the composer any more:** the run's progress is the banner, and why Send can't go, when it is only waiting for words or a choice, is its tooltip. What the person must do (install the agent, sign in) is the visible line above the status line (decision 5, Fix Y1).
23. **The card's time** reads "42 s" under a minute and "1:05" from a minute; the banner always m:ss, as in the design.
24. **An agent's reply is a small, safe Markdown subset** (Fix Z2; the parser `ai_markdown.rs`, the drawing `ai_reply.rs`):
    - **What it draws:** **bold**, *italic* (or _italic_), `inline code` (monospace on `t.field`), "- " and "* " bullets and "1." numbered lists with a hanging indent, paragraphs (lines in one paragraph join with a space; a blank line starts the next), `#` to `######` headings as SemiBold at the body's size (13 px, no bigger type in a narrow panel), links as their text in `t.accent_text`, and fenced code in a `t.field` box, monospace and wrapped.
    - **What it never does:** a link opens nothing and its address isn't shown; raw HTML is text; a marker without its pair (`2 ** 3`) stays as written; nothing scrolls sideways (a long word or line of code is broken to fit).
    - **Only the agent's replies:** your own message, notes and failures are plain text, so what you typed is shown as you typed it.
25. **The title's accessible name keeps its "Conversation: " prefix** (Fix Z3): the brief asks for the whole title as the accessible name; a screen reader also needs to know the button opens the conversations, so the name is "Conversation: {whole title}". The tooltip is the whole title alone (it was "Conversations").
26. **After Undo run, the card's buttons say what they can do** (Fix Z7). In the real run, presses after Undo run were logged as "invalid parameters". **The cause:** Undo run is itself an edit after the run began, so a second Undo run hit "the document changed after the run began" before "already undone", and Select layers found none of the layers. Both were refused with `bad()`, which Diagnostics calls "invalid parameters"; and the card kept both buttons live.
    - **Refusals** of `chat.undo-run`, `chat.redo-run` and `chat.select-run-layers` (and "Stop the run first", for New chat too) are "not available now" with fixed words (`chat_run::REFUSALS`). Diagnostics says "chat.undo-run: refused (already undone)". "Invalid parameters" is left to malformed calls.
    - **Undo run on an undone run** says "The run is already undone", checked first.
    - **Redo run** (`chat.redo-run`) puts the run back as the undo found it, one undo step ("Redo run"), while the change log has nothing after the undo but the undo itself. Any other edit, ⌘Z and ⌘⇧Z included, makes it "Something changed since". A selection doesn't. After a redo, Undo run works again: the run's own undo and redo aren't changes since the run began.
    - **The refusal line goes on the next action:** when the document's revision moves (any edit or selection) or another card button is pressed. Typing in the composer doesn't clear it.
    - **⌘Z after Undo run** brings the run back through the history, but the card stays "Undone" with Redo run off. The card knows the run from the change log, not by comparing documents every frame.
27. **Real Inter weights, everywhere in the UI** (Fix AA1). The UI took Inter only from the craft-fonts build input, which the builds here (and the installed app) don't have. So every panel drew in egui's default font, and "SemiBold" was that same face under another name: the real window showed it.
    - **Now:** `theme::install_fonts` uses the app's bundled variable Inter (`assets/fonts/inter/`) when craft-fonts has none. Proportional is Inter at `wght` 400; the `semibold` family is the same file at 600; the new `italic` family is `InterVariable-Italic.ttf` (`theme::inter_faces`). egui can't make up a weight, but epaint 0.36 sets a variable font's axes per face (`FontTweak::coords`).
    - **Effects beyond the AI panel:** every panel now draws in Inter, and the roles that already asked for SemiBold elsewhere (`TextStyle::Heading`, the panels' section titles) get it. The brief asked for no other panel text to change. Keeping egui's default face elsewhere would have meant two typefaces in one window, and the design is Inter throughout. Overrule this, and the AI panel can get its own families instead.
    - **In the AI panel:** SemiBold is the title, the empty state's heading, the agent's name over its reply, the card's heading, the active row, and Markdown bold and headings. Everything else is Regular. Markdown *italic* is the italic face, no longer a slanted regular. `ui.inspect` › `agent.aiPanel.fonts` gives each role's family and size.

## Tests

- `ai_panel_tests.rs` (12):
  - no connector, status or warning in the AI tab;
  - every captured row is in Settings ▸ Agents, with its commands;
  - `ui.inspect`'s header, popover, empty state and composer, with every icon button named;
  - the Follow button toggles and shows paused;
  - a new chat is lazy and kept, and is in neither the document nor Diagnostics;
  - search and scope;
  - read-only;
  - the selection button;
  - the image button, with `--image` reaching the run;
  - the hint line;
  - no sideways scroll or clipping at 240, 240 (the default) and 320 px;
  - New chat during a run (Fix U3): disabled in `ui.inspect`, drawn in the `disabled` token, no focus or click action, a click changes nothing, `chat.new` and `chat.clear` refuse with "Stop the run first", and it works again once the run ends. It fails with the button drawn enabled.
- `chat_ui_tests.rs`:
  - `command_return_sends_and_escape_stops` (⌘↩ and Esc);
  - `with_no_agent_installed_it_says_so_with_a_link_to_settings_agents`.
- `field_edges_tests.rs`: the S2 border audit now opens the popover too, in both themes and six accents; the lowest edge is still above 3:1.
- `chat_tests.rs` and `headless_tests.rs` (engine): the commands are the person's and check their parameters; the image goes where the profile takes it; `settings.open` accepts `agents`.
- `apps/figcraft` › `services_tests.rs`: `conversations.json` is written owner-only and read back.
- **Tests that moved with their features:**
  - the connect, Claude Desktop and client tests open Settings ▸ Agents and scroll (`live_tests::scroll_settings_to`);
  - the Activity, lint and lock tests open the Inspect tab;
  - the follow-resume tests use the header's button.

## Captures and the live run (Fix T6)

- **The panel and Settings ▸ Agents,** light and dark, at 240 px: `docs/scrim-ports/ai-panel/`. They are offscreen captures, because the screen was locked.
- **The grace, centring and the reveal in the real app:** `docs/scrim-ports/follow-agent/t6/`.
- **Settings ▸ Agents' first heading is "Connections"** beneath the page's own "Agents", so the page doesn't read "Agents, Agents".

## Fix X: tests and captures

- `ai_panel_theme_tests.rs`:
  - the guard against colours written in code;
  - `readable_on`;
  - every role reading in both themes with every accent (decision 20);
  - the accent changing only its own roles.
- `ai_panel_tests.rs`:
  - the header, empty state and status line;
  - the composer and its "Agent settings…" item;
  - the banner, bubble and card while working and done, with both buttons;
  - nothing clipped mid-run at 240, 320 and 400 px in both themes.
- `chat_run_tests.rs` (engine):
  - the rows;
  - both commands the person's on every route;
  - Undo run held during the run, restoring the document exactly as one step, refused after the person's edit, noted without the prompt;
  - Select layers.
- **Captures:** `docs/scrim-ports/ai-panel/x/`, offscreen, both themes, three accents, 336 and 240 px, idle and mid-run.

## Fix Z: tests

- `ai_markdown_tests.rs` (Z2): asterisks and bullets, an unmatched `**` (and raw HTML) left as written, code fences (closed and not), bold inside a bullet with a heading and a link, empty input and very long lines.
- `ai_reply_tests.rs` (Z2): a reply's blocks in `ui.inspect` with no marker left as text, while your message and a note keep theirs; at 240, 320 and 400 px in both themes, with Spanish text, an 80-character word and a long line of code, every block stays 16 inside the panel and nothing scrolls sideways. It fails with the reply drawn as one plain label, and the width check fails when a code block isn't wrapped.
- `ai_panel_z_tests.rs` › `a_long_title_is_cut_with_an_ellipsis_and_the_chevron_follows_it` (Z3): a 48-character title at 240, 320 and 400 px in both themes is cut with "…" (a prefix of the title), the chevron 6 after the text and clear of Follow, the buttons 8 apart and 16 from the right, the accessible name and the hovered tooltip the whole title. It fails with the old header (the title clipped without "…", the tooltip "Conversations").
- `ai_panel_z_tests.rs` › `the_transcript_fades_at_the_bottom_only_with_more_below` (Z4): light and dark, no fade for a short transcript nor at the bottom of a 60-line one; scrolled up with the wheel, a 12 px fade inside the panel, into the panel's own colour (`theme.fade`).
- `ai_panel_z_tests.rs` › `selection_and_image_are_round_icon_chips_at_every_width` (Z6): at 240, 280, 336 and 400 px, light and dark, with the image chip and without, Selection (and the image) are 32 × 32, named where they're drawn, 8 apart after the agent chip, clear of Send and inside the panel, and no "Selection" word is drawn. It fails with the text chips. `the_icon_chips_keep_their_tooltips`: hovered, each shows its name.
- `ai_panel_theme_tests.rs` (Z6): the chips' icon reads at 3:1 on the chip and on its hovered fill, in both themes with every accent.
- `ai_fonts_tests.rs` (AA1): every SemiBold role uses the SemiBold family, led by Inter SemiBold, and every Regular role uses Inter Regular; italic is the italic file; the harness's font set is the app's; SemiBold lays out about 3% wider than Regular at 100 px (it was equal); no word the panel draws falls back past Inter. Three of the four fail on the old fonts, the fourth since it also checks the faces.
- `chat_run_tests.rs` (Z7, engine):
  - `after_undo_run_the_buttons_are_refused_in_words_never_as_invalid_parameters` reproduces the real run. It fails on the old code with "chat.undo-run: invalid parameters" ("the document changed after the run began").
  - `redo_run_restores_the_run_as_one_step_and_undo_run_works_again`;
  - `redo_run_is_refused_once_something_changed_since_the_undo` (an edit, and ⌘Z);
  - `redo_run_is_the_persons_alone` (and refused while a run goes).
- `ai_panel_z_tests.rs` (Z7):
  - `after_undo_run_the_card_is_undone_and_redo_run_puts_it_back`, both themes: the heading, the dimmed rows, Select layers off (disabled for screen readers, not focusable, the `disabled` token, a press runs nothing), Redo run restoring the run as one step;
  - `redo_run_is_off_once_something_changed_since`, with its tooltip;
  - `a_refused_press_says_why_under_the_buttons_until_the_next_action`, at 240 and 336 px: the line, no toast, Diagnostics' words, gone after the next action.
