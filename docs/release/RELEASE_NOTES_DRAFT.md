# Scrim <version> (draft release notes)

> Draft, not published. Fill in the version and the checksum once `cargo xtask release-macos` has made the image ([`macos.md`](macos.md)), and read it through before it goes on the release page.

Scrim is a design tool where people and AI agents work on the same canvas. It is an independent fork of FigCraft by the ArtCraft team, with the work of the earlier TypeScript Scrim carried over. This is its first release as Scrim.

## Download

- **macOS:** `Scrim-<version>-universal.dmg`.
  - Works on Apple silicon and Intel Macs, macOS 11 or later.
  - Signed with a Developer ID and notarised by Apple.
  - SHA-256: `<sha256>`
- **Check the download:** run `shasum -a 256 -c Scrim-<version>-universal.dmg.sha256` in the folder you downloaded both files to.

## What changed since the fork

### Files

- **`.scrim` is Scrim's own format** (Fix AG).
  - Files are saved as `.scrim`.
  - A `.figcraft` file still opens, and is never written over: Save asks for a `.scrim` beside it.
  - What a file is comes from its contents, not its name.
- **More files open.**
  - Figma `.fig` (best effort), Sketch, Penpot, Lunacy's `.free`, Figma REST API JSON, the earlier Scrim's JSON `.scrim`, SVG and images (Fixes AG and AH).
  - Each import opens as a new, unsaved document, and its report says what was left out or approximated.
  - Affinity and Adobe XD files are refused with a note: their formats are closed, so export SVG from them instead.
- **One name for a document everywhere** (Fix AF): the tab, the title bar, Home and agents all use the same name. A new document is "Untitled", then "Untitled 2", and so on.

### Working in the editor

- **Scrim opens on Home**, not on a new Untitled file (Fix AD).
- **Option-drag duplicates.** Holding or releasing Option during the drag decides it, and the copy is one Duplicate in the undo history (Fix AE).
- **The AI panel was redesigned** (Fixes AA to AC).
  - Real Inter SemiBold and Italic.
  - A run card that follows Undo and Redo, and summarises the agent's changes.
  - A + menu in place of the Selection and Image chips; contextual suggestions; an empty state; an "Undone" message.
  - The panel's header no longer scrolls away.
  - The panel stops redrawing while nothing happens.

### Safety and lost work (Fix AI)

Ported by hand from the fixes upstream FigCraft made since the fork, each shown by a failing test first (`docs/design/upstream-sync.md`).

**Security:**
- TLS libraries updated past their advisories.
- A plugin can't upload the document or reach a server.
- A document's library path is never followed off this computer.
- Only the client an id was given to can resume a live-session seat.
- A crafted JPEG or a small zip bomb can't crash Scrim or fill its memory.
- Several unbounded costs when reading `.fig` files are now bounded.

**Lost work:**
- Page ruler guides survive save and reopen.
- A number that isn't finite never makes a file that won't open, and files already damaged that way open again.
- A save never drops images it couldn't load.
- A crash during a version preview recovers your work, not the version.
- Save never silently overwrites changes another app made to the file. It offers Save as a copy, Overwrite or Reload.

**Everyday fixes:**
- Quit and ⌘Q ask about unsaved changes, with Don't Save, Cancel and Save; closing a window asks the same.
- Someone alone in a live session who moves the pointer stays connected.
- Recovery copies are written even while the window is hidden.
- An agent's request for a huge document gets an answer that fits, with a note saying how to read the rest.
- A FigCraft board file says it isn't supported, and is never saved over without asking.
- Two edits are faster on large documents.

## Known limits

- **Boards** (FigCraft's whiteboard files) aren't supported. One opens as a normal design with a notice, and saving it removes its board features.
- **Plugins get no network**, by design.
- **Translations:**
  - Spanish has every string, and Latin American Spanish reads it where its own wording doesn't differ. Neither has had a native speaker's review: both need one.
  - The thirteen other interface languages show English for 432 to 532 of the 2,148 strings (a fifth to a quarter), mostly those added since the fork.
- **The app icon** is a placeholder.
- **Live sessions:** a host whose window is covered or minimised applies guests' edits once it draws again. The session stays connected meanwhile.
- **Status: alpha.** Keep copies of work that matters.

## Privacy

- Scrim has no telemetry, no update check and no crash reporting.
- It connects to another computer only for what you start: a live session, a file on a server, a team library from a server, or a plugin directory. One exception: while the Assets panel shows a team library from a server, Scrim checks it for updates every 30 seconds.
- Its agent connection (MCP) listens on this computer only.

Details: [`network-audit.md`](network-audit.md).

## Credits

Scrim is based on FigCraft by the ArtCraft team; it is not an ArtCraft product. Dual-licensed MIT OR Apache-2.0. Contact the maintainers through the address in the README and the About window.
