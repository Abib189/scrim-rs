# scrim-rs

**The next Scrim:** a design tool where AI agents are first-class collaborators on the canvas and the code export is a guarantee, not a hand-off. Native on macOS, Windows, Linux and FreeBSD; in the browser through WebAssembly.

scrim-rs is based on **FigCraft by the ArtCraft team** (github.com/storytold/figcraft). It is an independent fork, not an ArtCraft product, and is not endorsed by the ArtCraft team. It keeps FigCraft's Rust engine, its egui interface and its renderer. It adds what Scrim, the earlier TypeScript editor, learned:
- the export guarantee, measured by the HTML-fidelity check;
- agent safety: locks, a stale-write guard and a change feed;
- a hardened `.fig` importer;
- the `query_nodes` query, the design lint and `analyze_design`;
- layout guides, and an SVG allow-list.

The specs and test cases for all of these are in [`docs/scrim-transfer/`](docs/scrim-transfer/README.md). How the fork relates to upstream, and the licence duties, are in [`docs/FORK.md`](docs/FORK.md).

> [!IMPORTANT]
> **Status: alpha, freshly forked (19 Oct 2026).** The engine and editor are FigCraft v0.7.0's. The Scrim ports land one at a time; see `docs/FORK.md` and the commit log.

## Get started

```sh
git clone https://github.com/Abib189/scrim-rs
cd scrim-rs
cargo run --release -p figcraft            # the desktop app
cargo run --release -p figcraft-cli -- --help
cargo xtask ci                             # fmt, clippy, tests, layering, assets, wasm
```

- **Toolchain:** Rust 1.95 or newer, with the `wasm32-unknown-unknown` target and the `rustfmt` and `clippy` components (`cargo xtask ci` needs them all).
- **Web build:** install [trunk](https://trunkrs.dev/), then run `cargo xtask wasm`.
- **Fonts:** without the optional `../craft-fonts` checkout, the app falls back to your system fonts.
- **Contributors and AI agents:** start with [`AGENTS.md`](AGENTS.md).

## Built for agents

Every capability is a command with one entry point: the same commands drive the menus, the CLI, the control channel and the MCP server.

```sh
figcraft-cli export design.figcraft -o out/               # headless render and export
figcraft-cli run script.json --in design.figcraft          # a script of commands, no window
figcraft-cli commands --json                               # every command and its parameters
figcraft-cli mcp [--attach PORT]                           # MCP over stdio, headless or attached
```

## Privacy and online sharing

The app works offline: no account, no telemetry, and no server is contacted until you share or join a file.

**Share ▸ Online** uploads the file to a server. In this fork that server is still FigCraft's public default (`collab.craftapps.dev`), which the ArtCraft team runs, not Scrim. The server stores files unencrypted, and anyone with a link can open them. Prefer **Share ▸ Local network**, your own server ([`docs/server.md`](docs/server.md)), or a build with no default server:

```sh
FIGCRAFT_DEFAULT_SERVER= cargo build --release -p figcraft
```

## Licence and credits

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option, like FigCraft.
- Copyright (c) 2026 ArtCraft Team and the FigCraft contributors, and the scrim-rs contributors.
- The required notices are in [NOTICE](NOTICE). Asset provenance is in [ATTRIBUTION.md](ATTRIBUTION.md).

scrim-rs is based on FigCraft by the ArtCraft team. The ArtCraft name and marks are their trademarks; they have been removed from this fork as FigCraft's brand terms require.
