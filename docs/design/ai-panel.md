# The AI panel (Fix T4, 22 Oct 2026)

**Why:** the AI tab had become a long list: the agents' status, a warning, about sixteen Connect rows, Activity, Design lint and the lock. The composer was squeezed at the bottom. The person wants the AI tab to be a chat, and every connector in Settings. The reference was a screenshot the person supplied.

## The layout, top to bottom

- **The header** (`ai_panel.rs`):
  - **The conversation's title with a chevron.** A new chat reads "New chat".
  - **The Follow agents button,** a crosshair with the tooltip "Follow agents as they work". It runs `camera.follow-agents`.
    - **On:** accent on a selected fill.
    - **Paused, while an agent works (Fix N):** an amber dot, and the name "Follow agents as they work: paused, press to resume". Clicking resumes.
    - **Its tooltip** has a second line, following's status ("Following Codex · Poster").
  - **New chat** (+, `chat.new`).
  - **No Share button or avatar:** they aren't Scrim features.
- **The chevron's popover:**
  - "Search conversations…", matching titles and messages here only, ignoring case.
  - "This document | All documents" (`chat.scope`).
  - The conversations, newest first, the one shown checked (`chat.open`). Under All documents, each also names its document.
- **The body:** the transcript, scrolling and sticking to its end. When empty, a chat bubble and "Describe what you want to create or change." in the middle.
- **The composer**, pinned to the bottom (`ai_composer.rs`):
  - **Above the field, when needed:** the grace hint's line, and the line for a missing CLI.
  - **The field:** several lines, with the placeholder "Describe a change…".
  - **A row of controls:**
    - the selection (`chat.add-selection`);
    - an image (`chat.attach-image`), only for an agent whose CLI takes one;
    - the agent (`chat.agent`);
    - the settings gear (Settings ▸ Agents);
    - Send at the right, which is Stop while a run goes.
  - **Keys:** ⌘↩ (Ctrl+Enter) sends; Esc and Stop stop a run.
  - **R5 and S1 are unchanged:** the commands are the person's, the first-use dialog, no keys stored, `USER`/`LOGNAME` in the child's environment, the time cap, and Undo one step per edit.

**`ui.inspect` › `agent.aiPanel`** reports what was drawn: the header (with the Follow button's rectangle), the popover, the empty state, read-only, and the composer's controls, hint line, missing-CLI line and status.

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
5. **A missing or signed-out CLI:**
   - **Not installed** is known at once.
   - **Not signed in** is only known after a run stops before it began. The line then says "{agent} couldn't start: is it signed in?" with the link to Settings ▸ Agents, until a run starts.
6. **The selection button writes "Selected: Name (id), …" into the prompt,** where the person sees and can edit it. R5's automatic context line (locked decision 8: the page and the selection) stays.
7. **The image is data on the agent's profile** (`image_args`):
   - **Codex:** `-i/--image`, read from `codex exec --help` 0.163.0. It goes right after `exec`, so the next flag ends its list and the prompt isn't read as a file.
   - **Claude Code** runs with no built-in tools (`--tools ''`), so it can't open a path: the button is hidden for it.
   - **An image path** goes only to the agent's argument list: never the document, and never Diagnostics.
8. **The AI panel's own actions are commands,** the person's alone, like R5's. The new commands are `chat.new`, `chat.open`, `chat.scope`, `chat.add-selection`, `chat.attach-image`, `chat.agent`, `chat.dismiss-hint` and `chat.grace-setting`. The last is "Open the setting", which was a direct call before. Typing in the search field is window state, not a command.
9. **New chat while a run goes does nothing:** the run's lines belong to its conversation. The old Clear button is gone, and `chat.clear` now means a new chat. The conversation shown is kept.
10. **Conversations of an unsaved document** are keyed by its window's document number. After a restart that document is a new one, and its old conversations show as another document's.
11. **The three new icons** (crosshair, gear, Send's arrow) are drawn in `ai_icons.rs`: `icons.rs` is over the size limit.
12. **The old AI-tab Follow row and its Resume** are gone. The header's button does both, and following's status is in its tooltip.

## Tests

- `ai_panel_tests.rs` (11):
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
  - no sideways scroll or clipping at 240, 240 (the default) and 320 px.
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
