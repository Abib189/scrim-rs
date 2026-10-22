# Personal data in the evidence (R1, 22 Oct 2026)

**The question:** do the evidence folders (`docs/scrim-ports/…`, the design captures) carry anything personal: the macOS login name or home folder, device or host names, email addresses, tokens or API keys, session ids, IP addresses other than 127.0.0.1, or the content of the person's own files?

## How it was checked

- **Text:** every tracked file under `docs/` and `scripts/` (565 files, 261 of them text: Markdown, JSON logs, JSONL, Python, TypeScript), with the patterns `cargo xtask pii-scan` now runs, and this machine's login and host name.
- **Captures:** all 298 tracked PNGs, read with Apple's Vision text recognition (`scripts/ocr-captures.swift`), and the text searched with the same patterns. Each hit was looked at.
- **History:** the text with `git log -S` over every commit; every version of every capture ever committed under `docs/scrim-ports/` and `docs/design/` (268 images) read the same way.

## What was found, and what changed in the working tree

| Where | What | Now |
|---|---|---|
| `docs/scrim-ports/settings/f2/f-03-panel-buttons.png`, `f-04-confirmation.png`, `f-05-written.png` | The Claude Desktop entry's `PATH` started with the real home folder (`/Users/<user>/.nvm/…`) | That line reads `"/Users/<user>/` |
| `docs/scrim-ports/settings/f2/f-13-settings-mcp.png` | The MCP root folder showed the scratch folder's path, part of which is the coding session's id | It reads `<scratch>/home-f/Documents/Scrim)` |
| `docs/design/rebrand/before/home-dark.png` | Home's recent file showed its full path in the person's home folder | It reads `/Users/<user>/scrim-rs/examples/ridgeline-app.figcraft` |
| `docs/scrim-ports/working-gap/logs/relay/wire.jsonl` | Two MCP session ids (their first 8 hex digits) from Fix M's relay | `<session-a>`, `<session-b>` |

- **Nothing else matched.** No login name, host name, `.local` name, real email address, bearer token, API key or other IP address is in any tracked text. The other emails and addresses are deliberate and listed in `xtask/pii-allow.txt` (example domains, the noreply maintainer placeholder, FigCraft's published contact, documentation IPs, the WiX UpgradeCode).
- **How the captures were changed:** the words were painted over in the background colour beside them and the placeholder written in the text's colour, so the rest of each capture is unchanged.

## Captures showing the person's own documents (listed, not changed)

These show **Brand colour system mockups** (the person's file) or the **Fixture** rebuilt from the person's old `.scrim` file with its names and positions. A retake on a scratch folder would still need those files, so they are listed for the person to decide:

- `docs/scrim-ports/layers/`: `brand-01-before-collapse.png`, `brand-02-after-collapse.png`, `brand-03-after-second-press.png`, `camera-01-before-click.png`, `camera-02-mid-flight.png`, `camera-03-after-click.png`, `camera-04-cancelled-by-scroll.png`, `fixture-01-before-collapse.png`, `fixture-02-after-collapse.png`, `fixture-03-after-second-press.png`, `h3-01-before.png`, `h3-02-tooltip-collapse.png`, `h3-03-collapsed.png`, `h3-04-tooltip-restore.png`, `h3-05-restored.png`, `h3-kb-01-button-focused.png`, `h3-kb-02-space-collapsed.png`, `h3-kb-03-space-restored.png`, `kb-01-start.png` to `kb-07-after-enter.png` (7 files).
- `docs/scrim-ports/settings/`: `fonts-before-missing-fonts.png`, `fonts-after-no-dialog.png`.
- **Found by their names in the text.** A capture showing these documents' canvas without any of their names visible wouldn't be found this way.

## In pushed history (not rewritten: the person decides)

Rewriting pushed history needs a force push, so it was not done. The older versions stay in these commits, all on `origin/main`:

| Commit | Date | Path | What |
|---|---|---|---|
| `46ea6e4` | 19 Oct 2026 | `docs/design/rebrand/before/home-dark.png` | the home folder, in a recent file's path |
| `26770bd` | 20 Oct 2026 | `docs/scrim-ports/settings/f2/f-03-panel-buttons.png`, `f-04-confirmation.png`, `f-05-written.png` | the home folder, in the `PATH` line |
| `26770bd` | 20 Oct 2026 | `docs/scrim-ports/settings/f2/f-13-settings-mcp.png` | the coding session's id, in a scratch path |
| `77c0f78` | 21 Oct 2026 | `docs/scrim-ports/working-gap/logs/relay/wire.jsonl` | two MCP session ids (8-digit prefixes of ids from a test app, now expired) |
| `221a69c`, `89640c0`, `e12fe5e` | 20 Oct 2026 | the 27 captures listed above | the person's own documents (still in the working tree too) |

No commit ever had the login name in text (`git log -S` over the whole history).

## The check: `cargo xtask pii-scan`

Part of `cargo xtask ci` and of GitHub's CI (`xtask/src/pii_scan.rs`). It fails on any of these in a tracked text file under `docs/` or `scripts/`:
- **home folders:** `/Users/<name>`, `/home/<name>` (at the start of a path), `C:\Users\<name>`, and the encoded form some tools use in folder names (`-Users-<name>-`);
- **device names:** `<name>.local`;
- **this machine's login and host name,** read when the scan runs and never written into the repository; generic names (`runner`, `root`, `user`…) are left out;
- **email addresses,** except the reserved example domains;
- **bearer tokens and API keys** (Anthropic- and OpenAI-style `sk-`, GitHub, Slack, AWS and Google key shapes);
- **session ids:** a JSON `"session"` hex value, an `Mcp-Session-Id` value, a UUID;
- **IP addresses** other than 127.0.0.1.

**Placeholders pass:** `/Users/<user>`, `<device>`, `<token>`, `<scratch>`, `<session-a>`. **Deliberate values** go in `xtask/pii-allow.txt`, one per line with its reason.

**What it can't see:**
- **Images.** Captures are read with `scripts/ocr-captures.swift` before they're committed (macOS only; it isn't part of CI).
- **A document's own content.** Whether a capture shows the person's file can't be told from a pattern.

**Tests** (`xtask/src/pii_scan_tests.rs`): each kind is found; placeholders, loopback, example domains and look-alikes (`settings.local.json`, `src/home/…`, version numbers) pass; the allow-list lets through only the listed value; this machine's names are found without being repeated in the report; findings carry their line.
