# Scrim

**A design tool where people and AI agents design together on one canvas.** Agents are first-class collaborators, with the same commands as the person at the keyboard, and the code export is a guarantee, not a hand-off. Native on macOS, Windows, Linux and FreeBSD; in the browser through WebAssembly.

**Scrim is based on FigCraft by the ArtCraft team** (github.com/storytold/figcraft). It is an independent fork, not an ArtCraft product, and is not endorsed by the ArtCraft team. It keeps FigCraft's Rust engine, its egui interface and its renderer, and adds what Scrim, the earlier TypeScript editor, learned:
- the export guarantee, measured by the HTML-fidelity check;
- agent safety: locks, a stale-write guard and a change feed;
- a hardened `.fig` importer;
- the `query_nodes` query, the design lint and `analyze_design`;
- layout guides, and SVG files refused (never sanitised) when they could run script or reach outside.

The specs and test cases for all of these are in [`docs/scrim-transfer/`](docs/scrim-transfer/README.md). How the fork relates to upstream, and the licence duties, are in [`docs/FORK.md`](docs/FORK.md). The repository is still called `scrim-rs`.

> [!IMPORTANT]
> **Status: alpha.** The engine and editor are FigCraft v0.7.0's, with Scrim's ports on top (`docs/scrim-ports/`). The app icon is a placeholder.

## Get started

```sh
git clone https://github.com/Abib189/scrim-rs
cd scrim-rs
cargo run --release -p figcraft                              # the desktop app
cargo run --release -p figcraft-cli --bin scrim -- --help   # the CLI, `scrim`
cargo xtask ci                                               # fmt, clippy, tests, layering, assets, wasm
```

The Cargo packages keep FigCraft's names (`figcraft`, `figcraft-cli`…) so upstream merges stay cheap; what you install and run is Scrim (`docs/design/rebrand.md`).

- **Toolchain:** Rust 1.95 or newer, with the `wasm32-unknown-unknown` target and the `rustfmt` and `clippy` components (`cargo xtask ci` needs them all).
- **Web build:** install [trunk](https://trunkrs.dev/), then run `cargo xtask wasm`.
- **Fonts:** without the optional `../craft-fonts` checkout, the app falls back to your system fonts.
- **Coming from FigCraft:** Scrim opens `.figcraft` files as they are, and on its first start copies (never moves) FigCraft's settings and recent files.
- **Contributors and AI agents:** start with [`AGENTS.md`](AGENTS.md).

## Built for agents

Every capability is a command with one entry point: the same commands drive the menus, the CLI, the control channel and the MCP server.

**Connect Claude Code to the running app:** the app serves MCP over HTTP on 127.0.0.1 (port 7680, or the next free one; Settings ▸ MCP shows it). The AI tab's **Connect Claude Code** runs this for you after asking:

```sh
claude mcp add --transport http --scope user scrim http://127.0.0.1:7680/mcp
```

The endpoint refuses requests from other hosts and from web pages; Settings ▸ MCP can also require a token. **Without the app**, or for stdio-only clients, `scrim mcp` still works, headless or attached (`docs/mcp.md`):

```sh
scrim export design.figcraft -o out/          # headless render and export
scrim run script.json --in design.figcraft    # a script of commands, no window
scrim commands --json                         # every command and its parameters
scrim mcp [--attach PORT]                     # MCP over stdio, headless or attached
claude mcp add scrim -- scrim mcp             # connect Claude Code (docs/mcp.md)
```

`figcraft-cli` still works as an alias of `scrim` for existing configurations.

## Privacy and online sharing

The app works offline: no account, no telemetry, and no server is contacted until you share or join a file.

**Scrim builds in no share server**, so nothing is uploaded anywhere by default. **Share ▸ Online** asks for the address of a server you trust before it uploads anything; **Share ▸ Local network** needs no server. A server stores files unencrypted, and anyone with a link can open them, so use one you or someone you trust runs ([`docs/server.md`](docs/server.md)).

A build can name a default server when compiling (FigCraft's `FIGCRAFT_DEFAULT_SERVER` is still honoured):

```sh
SCRIM_DEFAULT_SERVER=wss://design.example.org/ cargo build --release -p figcraft
```

Coming from FigCraft: the settings Scrim copies leave out FigCraft's built-in server, so Online asks for one; a server you chose yourself is kept.

## Licence and credits

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option, like FigCraft.
- Copyright (c) 2026 ArtCraft Team and the FigCraft contributors, and the Scrim (scrim-rs) contributors.
- The required notices are in [NOTICE](NOTICE). Asset provenance is in [ATTRIBUTION.md](ATTRIBUTION.md).

Scrim is based on FigCraft by the ArtCraft team. The ArtCraft name and marks are their trademarks; they have been removed from this fork as FigCraft's brand terms require.
