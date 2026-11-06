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

## Download and install

**macOS** (Apple silicon and Intel, macOS 11 or later):
1. Download `Scrim-<version>-universal.dmg` and `Scrim-<version>-universal.dmg.sha256` from the [Releases page](https://github.com/Abib189/scrim-rs/releases). No release is published yet; the first one will be listed there.
2. Check the download (optional), in the folder you saved both files to:
   ```sh
   shasum -a 256 -c Scrim-<version>-universal.dmg.sha256   # prints "…dmg: OK"
   ```
3. Open the image and drag Scrim to Applications.
4. Open Scrim. It is signed with a Developer ID and notarised by Apple, and carries its notarisation ticket, so macOS checks it once, without needing a connection, and asks "Scrim is an app downloaded from the internet. Are you sure you want to open it?": choose Open.

To check the app once it is installed: `spctl --assess --type execute -vv /Applications/Scrim.app` says `accepted, source=Notarized Developer ID`. If macOS refuses to open it, [`docs/release/macos.md`](docs/release/macos.md#gatekeeper-when-a-mac-wont-open-scrim) explains each message.

Windows, Linux and FreeBSD: build from source, below.

### What Scrim opens and saves

<!-- formats: made from the import registry; xtask's readme_formats_tests checks it -->
| Files | What Scrim does with them |
|---|---|
| `.scrim`, `.figcraft` | Scrim's own documents: opened and saved. A `.figcraft` file (FigCraft's) opens too and is never written over: Save asks for a `.scrim` beside it |
| `.scrim` | The earlier Scrim's JSON documents: imported as a new document |
| `.fig` | Figma files: imported as a new document (best effort) |
| `.sketch` | Sketch files: imported as a new document |
| `.penpot` | Penpot files: imported as a new document |
| `.free` | Lunacy files: imported as a new document |
| `.json` | Figma REST API file JSON, with its local variables: imported as a new document |
| `.svg` | SVG: imported as editable vectors |
| `.png`, `.jpg`, `.jpeg`, `.webp`, `.gif`, `.tif`, `.tiff`, `.bmp` | Images: opened as an image layer, or placed in a document |
<!-- formats end -->

What a file is comes from its contents, not its name, so the two kinds of `.scrim` are told apart. Affinity and Adobe XD files are refused with a note: their formats are closed, so export SVG from them. Scrim saves documents as `.scrim`; File ▸ Export writes PNG, JPG, WebP, SVG and PDF, and an experimental `.fig` copy.

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
- **Files:** Scrim saves `.scrim` files. It opens `.figcraft` files (FigCraft's, and Scrim's before November 2026) as they are and never writes them: saving one makes a `.scrim` beside it. It also imports `.fig`, `.sketch`, Penpot `.penpot`, Lunacy `.free`, Figma REST API JSON, the earlier Scrim's JSON `.scrim`, SVG and images, each with a report of what it couldn't bring across (`docs/design/scrim-format.md`).
- **Coming from FigCraft:** on its first start Scrim copies (never moves) FigCraft's settings and recent files.
- **Contributors and AI agents:** start with [`AGENTS.md`](AGENTS.md).

## Built for agents

Every capability is a command with one entry point: the same commands drive the menus, the CLI, the control channel and the MCP server.

**Connect Claude Code to the running app:** the app serves MCP over HTTP on 127.0.0.1 (port 7680, or the next free one; Settings ▸ MCP shows it). The AI tab's **Connect Claude Code** runs this for you after asking:

```sh
claude mcp add --transport http --scope user scrim http://127.0.0.1:7680/mcp
```

The endpoint refuses requests from other hosts and from web pages; Settings ▸ MCP can also require a token. **Without the app**, or for stdio-only clients, `scrim mcp` still works, headless or attached (`docs/mcp.md`):

```sh
scrim export design.scrim -o out/             # headless render and export
scrim run script.json --in design.scrim       # a script of commands, no window
scrim commands --json                         # every command and its parameters
scrim mcp [--attach PORT]                     # MCP over stdio, headless or attached
claude mcp add scrim -- scrim mcp             # connect Claude Code (docs/mcp.md)
```

`figcraft-cli` still works as an alias of `scrim` for existing configurations.

## Privacy and online sharing

The app works offline: no account, no telemetry, no update checks and no crash reports. It connects to another computer only for live sessions, files, team libraries and plugin directories you use, and for agents you run:
- **Live sessions:** joining one, or hosting one (on this computer, or on your local network when you choose that).
- **Files on a server:** Share ▸ Online, Publish to server, and Home ▸ Online, which asks those servers about your files when you open it.
- **Team libraries from a server:** while the Assets panel shows one and the window is in front, Scrim checks its server for a newer version when the panel comes into view, then at most every 5 minutes, and when you come back to the window (Libraries ▸ Check team libraries for updates checks at once). Each check tells the server which of the library's components the document uses.
- **Plugin directories** you browse; plugins themselves get no network.
- **Agents** you run with Describe a change talk to their own services.

Its agent connection (MCP) and the bridge listen and connect on this computer only. Every connection, with the code behind it, is in [`docs/release/network-audit.md`](docs/release/network-audit.md).

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
- Contact the maintainers: hello@scrim.design

Scrim is based on FigCraft by the ArtCraft team. The ArtCraft name and marks are their trademarks; they have been removed from this fork as FigCraft's brand terms require.
