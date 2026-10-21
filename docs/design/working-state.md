# One working state, and a slower reveal (Stage I.1)

**IA, 21 Oct 2026.** What the code does today, read from it, and the plan for IB–IE.

**The report:** an agent built a poster in six steps, 2–3 s apart, because it thinks between tool calls. The dots above the artboard disappeared between steps, and the reveal was too fast to follow.

## How "working" is decided today

**The cause is confirmed.** `working_indicator::alpha` fades the dots out once the agent has been idle `IDLE` (0.6 s) since `last`. `last` moves only when an agent's *edit* lands (`working_indicator::tick`, on `landed.iter().any(|l| l.agent)`). Reads (screenshots, `inspect_document`, `lint_design`) never move it. So every pause over 0.6 s + 0.4 s ends the indicator, and the next edit fades it in again.

**Everything that reads some notion of "an agent is working"**, each from a different source:

| Where | Reads | Source today |
|---|---|---|
| The dots (`working_indicator::paint`) | `alpha(began, last)` | the last agent edit, 0.6 s idle |
| The outline and its pulse (same function) | the same `alpha` | the same |
| The AccessKit status ("{agent} is working on {artboard}", `working_indicator::accessibility`) | drawn only while the row is drawn | the same |
| `ui.inspect` › `agent.working` (`follow_agent::working_text`) | the indicator's artboard | the same |
| The load budget's "something animates" (`follow_agent::budget`) | `indicator.artboard().is_some()` | the same |
| Follow's status, "Following {agent} · {artboard}" (`follow_agent::status`) | `follow.agent`, `follow.artboard` | the newest agent edit's client; never ends |
| The AI tab's status, "Connected · Codex" (`ai_tab::connection`, `cmd::mcp::status` › `clients`) | every client that sent `initialize` in the last 10 minutes | `mcp.client` notices |
| The AI tab's Activity feed (`ai_tab::activity`) | `history.changes`, the newest 30 | the change log (history, not state) |
| The reveal (`reveal::tick`) | each landed batch | the change log; not a working state |

## Where MCP requests enter

| Route | Session id | `clientInfo` | How it reaches the window |
|---|---|---|---|
| **The app's HTTP endpoint** (`figcraft_mcp::http` → `Server::handle_session`) | Yes: `Mcp-Session-Id`, issued on `initialize` (J1) | Yes, per session (J1) | Every tool call, reads included, is a `ControlRequest` with `route: "mcp (http)"` and `client` (`apps/figcraft/src/mcp_endpoint.rs`). `initialize` sends `mcp.client` without waiting |
| **`scrim mcp --attach PORT`** (a stdio server in the CLI, `Backend::Attached`) | No (stdio) | Held by the CLI's `Server`, **never sent to the app** | Each call is a control-channel request (`route: "control channel"`, `client: None`) on one TCP connection that lasts as long as the stdio client |
| **Headless `scrim mcp`** | No | In-process only | No window: nothing to indicate |
| **The control channel itself** (scripts, tests, I4's captures) | No | No | `route: "control channel"` |

- **Ending a session:** the HTTP endpoint answers `DELETE /mcp` with 405 ("no session to end"). Streamable HTTP lets a client end its session with `DELETE` and its `Mcp-Session-Id`, so nothing ends a session today. An `--attach` client ending closes its TCP connection, and the control server's connection thread returns; nothing is told.

## Paper, for reference only

- **What I could verify:** Paper's MCP server is configured in this session, so its tool definition is readable. `finish_working_on_nodes` is described as "MUST call this when done working. Remove the working indicator from artboards you were editing." It takes a `fileId` and optional `nodeIds`, and "with no arguments" releases all of them.
- **What I couldn't verify:** how Paper decides an agent is working before it calls that, or what happens when it never calls it. No Paper code was read; nothing is copied.

## The plan

### IB: one working state, per client (`crates/ui-egui/src/working_state.rs`)
- **A client** is keyed by its HTTP session id. An `--attach` client is keyed by its control connection, and anything else by its route. It carries its display name (J1's rules).
- **Working** means, for any client:
  - a request of its is queued or waiting (a screenshot waits for a frame);
  - **or** its last request of any kind was less than the grace period ago;
  - **or** it began work and hasn't finished.
- **It ends** when:
  - the client calls `finish_working`;
  - its session ends (HTTP `DELETE`, or the `--attach` connection closing);
  - **or** the grace period runs out. Expiry is at exactly last request + grace, on the window's clock.
- **The sources:**
  - every request answered for an MCP client (HTTP; `--attach` once it names its client);
  - every agent edit that lands from any route, so the control channel and plugins keep I2's behaviour. Their reads don't count, because they aren't MCP clients.
- **`--attach` names its client:** the CLI's server sends `mcp.client` on `initialize`, as the HTTP endpoint does. The control server remembers it per connection and tags that connection's requests with it. When the connection closes, it tells the window the session ended.
- **HTTP `DELETE /mcp`** with a known `Mcp-Session-Id` ends that session (200); otherwise 404. `GET` stays 405.
- **The grace period:** `Settings.agent_grace_secs`, default 15, saved (`settings_version` 5). The command `pref.agentGrace {seconds: 5|15|30|60}` refuses other values, and has its MCP route. Settings ▸ General ▸ Agents at work: "Keep the working indicator for … after the last agent request".
- **`finish_working {ids?}`:** a new MCP tool over the command `agent.finish-working`.
  - **Access:** Read, so it runs with only read-only tools on. It's listed in Tool access and `list_commands`.
  - **Effect:** it asks the window to end the calling client's state (`Session::request_ui`, which isn't a dialog, so the control channel keeps it). It never touches the document, undo or the revision.
  - **The reply:** `{finished: true, client}`, plus `ids` echoed. With `ids`, the state still ends for the whole client (see disagreement 3).
- **The tool descriptions:** each tool that isn't read-only gets one short line: "When you have finished working on the canvas, call finish_working so the person sees you are done." The instructions string says it once too.
- **Every reader reads the one state.** The dots, the outline and its pulse, the AccessKit status, `ui.inspect`, the load budget, follow's "Following {name}" and the AI tab all read it, so nothing keeps its own timer:
  - **The name** is the most recently active working client's; when none is working, the last edit's.
  - **The dots** fade in over 160 ms when work first starts, stay continuous across gaps, and fade out over 400 ms only when the state ends.
  - **The static row** (Animations Off, reduced motion, degraded) stays for the whole state too.
  - **"Show working indicator" off** still hides only the dots.
- **AccessKit:** "{name} is working on {artboard}" for the whole state. When it ends, "{name} finished" (polite) for 2 s.

### IC: reveal speed
- **`Settings.reveal_speed`:** `slow | normal | fast`, default normal (`settings_version` 5). The command `pref.revealSpeed {speed}`, with its MCP route. Settings ▸ General, next to "Reveal agent work".

| Speed | Entrance | Stagger | Batch cap |
|---|---|---|---|
| Fast (today's) | 220 ms | 40 ms | 1.2 s |
| Normal | 480 ms | 110 ms | 4 s |
| Slow | 700 ms | 180 ms | 6 s |

- **The rest stays:**
  - the stagger shrinks to fit the cap;
  - over 60 layers, top-level groups only;
  - ease-out, the 4% scale;
  - the degradation order. Step 3 caps at half the speed's cap: 0.6 s at Fast, as today.
- **A batch mid-reveal already queues behind** the running stagger (`first = last_start + STAGGER`). That stays, with the speed's stagger, so each step reads as its own reveal.
- **The camera flight** (320 ms) and the first entrance both start when the batch lands, and the flight ends while the entrance is still running at every speed. So nothing fights, and the flight is unchanged. I'll re-measure that in IE.

## Where I disagree with the brief, or read it a certain way (overrule any)

1. **The Activity feed stays history.** It lists the change log, which isn't a state. The AI tab shows the working state on its own line in the Agents section ("Codex is working on Poster"), and the feed is unchanged.
2. **Reads over the plain control channel don't keep the indicator alive.** Its scripts and tests (`ui.screenshot`, `ui.inspect`) aren't agents at work; otherwise every capture would keep the dots on. Its edits still do. `--attach`, which is an MCP client, counts fully once it names its client.
3. **`finish_working`'s `ids`:** the state is per client, not per artboard, so the call ends the client's state whatever `ids` says. `ids` are echoed in the reply. Paper's per-artboard release would need an indicator per artboard, which I2 decided against: one indicator, on the latest artboard.
4. **The grace command takes only 5, 15, 30 or 60**, the values Settings offers; anything else is refused, and nothing is ever below 5.
5. **"In flight"** is a request queued for the window or waiting for a frame. Requests are answered on the UI thread in a frame, so that window is short; the grace period carries the gaps.
6. **"{name} finished"** is spoken once, for 2 s, and only when the state ends: by `finish_working`, the session ending, or the grace running out.
7. **The tool-description line goes on every tool that isn't read-only**, including `select`, `open_file`, `undo` and the input tools. That's the plainest reading of "every mutating tool", and the cost is one short sentence.

## IB as built

- **The state** (`crates/ui-egui/src/working_state.rs`): clients keyed by session (HTTP `Mcp-Session-Id`, or an `--attach` connection's `attach-N`), else client name, else route.
  - **One run:** it begins when the first client starts. It ends when the last one stops: at its finish, or at its last request + grace exactly, even when the frame that notices comes later.
  - **A new run during the last one's fade out** carries on from that fade's opacity, so there's no dip.
  - **`ui.inspect` › `agent`** gains `workingClients` and `finished`.
- **Where activity is noted** (`control::handle_route`, the path every agent request takes):
  - **an MCP request of any kind,** before it runs, so the client's own `finish_working` lands after it;
  - **the plain control channel** only when the request edited;
  - **a plugin's edits,** as they land.
- **The transports:**
  - `Server::with_session_hook` reports HTTP sessions starting and ending.
  - `DELETE /mcp` ends one.
  - `Backend::set_session` gives each request its known session.
  - `Server::announcing_clients` makes `scrim mcp --attach` send `mcp.client`. The control server keys that connection, tags its requests with the client and session, and sends `mcp.session-ended` when it closes.
- **`agent.finish-working`** is in the no-revision list next to Layers' collapse (`cmd::view_only`). The window ends the caller's state from the UI request.
- **Readers:**
  - **The indicator:** its alpha comes from the state. The wave runs on the clock, so it doesn't restart between steps. The artboard is let go once the run has ended and faded.
  - **AccessKit:** one live node; "{agent} finished" for 2 s where the row was.
  - **The AI tab:** a plain-text working line under the status. The indicator's node is the one live region, so it isn't announced twice.
  - **Follow's "Following {agent}":** names the working client; a nameless one falls back to J1's name for the latest edit.
  - **The load budget:** counts the state as activity.
- **Settings ▸ General ▸ Agents at work:** "Keep the working indicator for 5 s / 15 s / 30 s / 60 s after the last agent request".

## IC as built

- **`Settings.reveal_speed`** is `slow | normal | fast`, normal by default; any other value reads normal (`settings_version` 6). `pref.revealSpeed {speed}` has its MCP route. Settings ▸ General shows "Reveal speed: Slow / Normal / Fast" under "Reveal agent work".
- **`reveal::Speed`:**

  | Speed | Entrance | Stagger | Cap |
  |---|---|---|---|
  | `FAST` | 220 ms | 40 ms | 1.2 s |
  | `NORMAL` | 480 ms | 110 ms | 4 s |
  | `SLOW` | 700 ms | 180 ms | 6 s |

  `FAST` is Stage I2's constants exactly.
- **Each entrance keeps its own duration,** so changing the speed mid-reveal never warps one already running.
- **Degradation step 3 halves the speed's cap** (0.6 s at Fast, as before). The stagger shrinks first; the entrance is never shortened for load.
- **Steps queue:** a batch landing mid-reveal starts one stagger after the running batch's last entrance. None of the running entrances restart or are cut.
- **The camera flight is unchanged** (320 ms). It and the first entrance both start when a batch lands, and at every speed the entrance (220–700 ms) is still running when the flight ends. Nothing finishes "before the first reveal ticks", so there was nothing to lengthen.
- **Tests:**
  - **`reveal_tests.rs`:** Stage I2's tests now run at Fast and still pass unchanged, which proves Fast is the old behaviour. New:
    - the speed table;
    - every speed and its degraded cap, for 1, 6, 36 and 60 layers;
    - 36 layers at Normal last over 2 s and at most 4 s;
    - a step mid-reveal queues behind, at every speed;
    - render-only at Normal and Slow.
  - **`crates/mcp/tests/follow_agent.rs`:** both new settings and `finish_working` over MCP, none of it touching the document.

## ID: the tests

- **In the window, on the real request path** (`working_state_window_tests.rs`: requests on the control queue with route, client and session, as the HTTP endpoint queues them):
  - six steps of 6–36 layers 2.5 s apart keep the dots drawn in **every** frame; `finish_working` then fades them out in 400 ms ("Codex finished");
  - read-only calls (the screenshot's render, `inspect_document`, `lint_design`) keep it alive 12.5 s past the last edit with a 5 s grace period;
  - with no finish, it ends at the grace period exactly (5 s and 15 s), its fade counting from then;
  - a session closing (`mcp.session-ended`) ends it;
  - with two clients, one finishing leaves it on for the other, named after the latest;
  - an unknown name is shown as announced, and a missing one reads "An agent";
  - `finish_working` adds no revision or undo step.
- **Found by the first test:** a step landing on an artboard outside the view had no dots for the frames before the camera arrived. The row now stays on the artboard shown until the new one is in view.
- **Elsewhere:**
  - `working_state_tests.rs`: the state's own clock;
  - `agent_work_tests.rs`: the commands, Read, no revision;
  - `crates/mcp/tests/working.rs`: the tool, the hint, restricted access, sessions, `DELETE`;
  - `control_server_tests.rs`: `--attach`;
  - `reveal_tests.rs`: the speeds;
  - `crates/mcp/tests/follow_agent.rs`: the MCP routes.
