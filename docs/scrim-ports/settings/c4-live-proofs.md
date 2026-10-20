# C4 live proofs (the release build of 7322bb8 + C4, scratch HOME, 20 Oct 2026)

## A request with a non-loopback Host or Origin is rejected (curl → http://127.0.0.1:7600/mcp, ping)

```
loopback Host                          → 200
Host evil.example:7600                 → Host must be 127.0.0.1, localhost or [::1] with this port 403
loopback Host, Origin https://evil.example → requests from a web page are refused (Origin is not this server) 403
Host attacker.example + its Origin     → Host must be 127.0.0.1, localhost or [::1] with this port 403
```

## A disabled tool disappears from tools/list and is refused (over HTTP)

```
before:  24 tools; create_node listed: True
Tool access (Settings, via mcp.set-tool): 23 of 24 enabled
after:   23 tools; create_node listed: False
direct call: {"error":{"code":-32602,"message":"the tool `create_node` is turned off in Scrim (Settings ▸ Tool access)"},"id":3,"jsonrpc":"2.0"}
agent turns it back on: {"id":4,"jsonrpc":"2.0","result":{"content":[{"text":"invalid parameters for `mcp.restore-tools`: an agent can't turn MCP tools back on: that's the person's, in Scrim's Settings","type":"text"}],"isError":true}}
```

## One human edit and one agent edit, separate feed entries (history.changes, what the AI tab's Activity shows)

The person drew a rectangle with real window input (`ui.input`: R, then a drag on the canvas); the agent made an ellipse over HTTP (`run_command shape.ellipse`). Revision 13 is an earlier drag sent with `ui.pointer`, which is automation and is rightly logged as the control channel's.

```
revision 13 | agent | agent (control channel) | gesture | Rectangle | ['Page 1', 'Rectangle']
revision 24 | person | person | gesture | Rectangle | ['Page 1', 'Rectangle']
revision 27 | agent | agent (mcp (http)) | shape.ellipse | Ellipse | ['Page 1', 'Ellipse']
```

## The real Connect (the owner's HOME, the app started in a folder with no local `scrim` entry)

After Run and the one confirmation (captures 12–14), from that folder:

```
Checking MCP server health…
scrim: http://127.0.0.1:7600/mcp (HTTP) - ✔ Connected
scrim:
  Scope: User config (available in all your projects)
  Status: ✔ Connected
  Type: http
  URL: http://127.0.0.1:7600/mcp

```

The owner's local-scope `scrim` entries (in `~` and `~/Scrim`, for the TypeScript Scrim on 4848) were not touched, and still win in those folders. `claude mcp remove scrim --scope user` undoes the new one.
