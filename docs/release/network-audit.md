# Network audit (Fix R-7)

Every way the Scrim app (`apps/figcraft`, with the crates it links) reaches another computer, read from the code at commit `9a0f9f31`. The privacy statements in the README and the About dialog claim only what this page shows. Re-run the searches under [How this was checked](#how-this-was-checked) before changing either.

## Summary

- **No telemetry, no update check, no crash reporting, no analytics, no font or asset downloads.**
  - The workspace has no HTTP client crate.
  - The only network stack is `collab-net`'s WebSocket transport, with TLS from rustls.
- **Two things contact a server without a fresh request each time.** Both concern servers the person chose earlier. They are listed as decisions under [Flagged](#flagged).
  1. A team library linked from a server is checked for updates every 30 seconds while the Assets panel shows a document that uses it. Each check reports which of its components the document uses.
  2. Home ▸ Online asks the servers holding the person's online files about them each time that section is opened.
- **Everything else is started by the person:**
  - joining or hosting a live session;
  - publishing or uploading to a server;
  - managing a file's sharing;
  - browsing a plugin directory;
  - running an agent;
  - clicking a link.
- **The agent endpoints are loopback only.** That covers the MCP server, the control channel and the bridge.
- **Plugins reach no network.**

## Outbound connections

| # | What | Where in the code | Started by | Where it goes |
|---|---|---|---|---|
| 1 | Join a live session; resolve a short code to its host | `crates/collab-net/src/guest.rs:216` (`connect`) → `net.rs:222` (`open`), `net.rs:229-240` (`dial`); `guest.rs:74` → `manage.rs:88` (`resolve`) | The person: Join, or opening an invite link | The host or server named in the invite (TLS for `wss://`) |
| 2 | Publish a file to a server; upload a copy | `guest.rs:93` (`publish`), `guest.rs:178-209` (`upload_file`); `link.rs:288` (`upload`) | The person: File ▸ Publish to server…, Share ▸ Online | The server address the person typed |
| 3 | A file's hosting and sharing on a server (links, revoking, server info) | `link.rs:297-316` → `manage.rs:50` (`hosting`), `manage.rs:105` (`share`); asked from `crates/ui-egui/src/live.rs:1118` (`ask_server`) | The person: the Share popover and the Publish dialog (once an address is typed) | That server |
| 4 | Home ▸ Online: what became of the files the person put online | `crates/ui-egui/src/online.rs:58-62` (`refresh`), called at `online.rs:90` and `online.rs:149` | Opening Home ▸ Online (see [Flagged](#flagged)) | The servers holding those files |
| 5 | Team libraries: newer versions, with a usage report | `crates/engine/src/library_server.rs:161-178` (`refresh_server_libraries`); `crates/ui-egui/src/assets_panel.rs:272-290`, interval `SERVER_REFRESH` = 30 s at `assets_panel.rs:45`; `crates/collab-net/src/link.rs:228-245` (`library_background`) | **Automatic** while the Assets panel shows a document using a library linked from a server (see [Flagged](#flagged)); also when the person adds such a library (`assets_panel.rs:1079`) | That library's server |
| 6 | Plugin directory: list, install | `crates/ui-egui/src/plugins_ui.rs:272`, `plugins_ui.rs:474` → `crates/collab-net/src/link.rs:319` (`plugins`) → `manage.rs:178` | The person: Plugins ▸ Browse, Install | The directory server the person entered |
| 7 | Agent runs ("Describe a change") | `crates/mcp/src/agent_run.rs:78` | The person | The agent program the person chose (for example Claude Code). That program talks to its own service, and Scrim gives it only Scrim's loopback MCP address |
| 8 | Registering Scrim with an agent client (Connect, Settings ▸ Agents) | `crates/ui-egui/src/connect_flow.rs:129`, `mcp_registration.rs:48`, `agent_clients_ui.rs:245` → `apps/figcraft/src/services.rs:318-330` (`run_program`) | The person | Runs the client's own CLI to add Scrim's loopback address to its config. Scrim sends nothing itself |
| 9 | Links opened in the browser | `crates/ui-egui/src/ui_requests.rs:117` (Help), `dialogs.rs:23` (issues), `agent_clients_ui.rs:562` (agent docs), `present.rs:1840` and `dev_mode.rs:644` (links in a document, checked by `crates/engine/src/doc_urls.rs`) | The person clicking | GitHub, the agent client's documentation, or the link's own address |

## Listening, and loopback only

| What | Where | Address |
|---|---|---|
| MCP endpoint (Settings ▸ MCP) | `crates/mcp/src/http.rs:271` | 127.0.0.1 only |
| Control channel (`--control`, for scripts and tests) | `apps/figcraft/src/control_server.rs:35` | 127.0.0.1 only; off unless the flag is given |
| MCP bridge (`scrim mcp-bridge`) | `crates/mcp/src/bridge/upstream.rs:42`, address from `crates/mcp/src/discovery.rs:62-69` | Loopback only: `discovery.rs` drops any other address |
| Attached MCP backend (`--attach`) | `crates/mcp/src/backend.rs:123` | The address given on the command line |
| Hosting a live session | `crates/collab-net/src/host.rs:72`; the address from `crates/engine/src/live.rs:1595-1609` | 127.0.0.1 (this computer), or this computer's local-network address when the person shares on the local network |
| Finding the local-network address | `crates/collab-net/src/link.rs:101` (`lan_ip`) | Sends no packet: a UDP `connect` only asks the system for the route |
| Waking the hub's own listener when it stops | `crates/collab-net/src/hub.rs:588` | Its own listening address |

## Not present

- **HTTP clients:** none in `Cargo.lock` (no `ureq`, `reqwest`, `hyper`, `curl`, `isahc` or `attohttpc`). The only network crates are `tungstenite` and `rustls`, both in `collab-net`.
- **Telemetry, analytics, crash reports, update checks:** no code. Settings ▸ Privacy already says so, at `crates/ui-egui/src/settings_privacy.rs`.
- **Font and asset downloads:** none. Fonts come from the system or are built in.
- **Plugins:**
  - A plugin's only route out is the `call` host function (`crates/plugin/src/sandbox.rs:468`), which runs engine commands.
  - The engine refuses every command that reaches a server while a plugin runs (`crates/engine/src/plugins.rs:143`, `refuse_network`).
  - Plugins run in `wasmi`, an interpreter with no system access.
- **A default server:** a release build names none. `SCRIM_DEFAULT_SERVER` is honoured only when set at build time; `release-macos` doesn't set it.
- **`figcraft-server`:** a separate program, not part of the app.

## Flagged

Each of these contacts a server without the person asking that time. Both are for the owner to decide.

1. **Team-library refresh every 30 seconds, with usage reports** (row 5). While the Assets panel is open on a document that uses a library linked from a server, Scrim asks that server every 30 seconds for a newer version. Each request reports which of the library's components the document uses (`library_server.rs:161-178`).
   - The person chose the library link, so the server is theirs or one they trust.
   - Options:
     - check only when the panel opens and on request;
     - drop the usage report, or make it a setting;
     - leave it as is, and say so in the privacy statement (the statement does say so).
2. **Home ▸ Online refresh** (row 4). Opening Home ▸ Online asks the servers that hold the person's online files about those files. Opening the section is a choice, but nothing on screen asks first.

## How this was checked

Each search ran over shipped code; tests were read separately:
- `TcpStream::connect`, `connect_timeout(`, `UdpSocket::`, `TcpListener::bind`, `to_socket_addrs(`.
- `Command::new(` (programs started).
- `open_url`, `hyperlink_to`, and `"https://` string literals.
- `telemetry|analytics|sentry|crash|update` names.
- HTTP client crates in `Cargo.lock`.
- The plugin host's imports (`func_wrap`).

Every hit is in a table above.
