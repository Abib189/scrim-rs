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
