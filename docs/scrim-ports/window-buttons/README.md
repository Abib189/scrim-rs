# Window buttons over the tab strip (Fix S3, 22 Oct 2026)

**The report:** on macOS, the red, yellow and green buttons overlapped the Home sidebar's "Local library" label.

## What was found

- **"Local library" isn't a scrim-rs label.** It's in the old TypeScript Scrim's Home sidebar (`~/Scrim/apps/studio/src/home/Home.tsx`, an Electron window with `titleBarStyle: 'hiddenInset'`). In scrim-rs, Home's sidebar starts below the 40 pt tab strip, so the buttons sit over the strip, never over the sidebar.
- **The strip already left room.** On macOS the window has a full-size content view (`TitleBar::Inset`), and the strip started at 76 pt. That number was written where the strip is drawn, with no test, and a secondary window's room for the buttons wasn't checked anywhere.

## What changed

- **One place:** `tabs::window_buttons_inset(TitleBar)` decides the room: 76 pt for `Inset` (macOS, main and secondary windows), an 8 pt margin otherwise. `Custom` draws its own buttons at the right. The strip reads it from the window's own title bar.
- **Tests** (`crates/ui-egui/src/tabs_inset_tests.rs`):
  - the values;
  - nothing smaller than the window is drawn in the buttons' area (x 0–72, y 0–30, fixed in the test from these captures) on Home and in the editor, light and dark;
  - with a native title bar the Home tab *is* in that area, so the test sees what it guards;
  - a window opened by `window.new` keeps `TitleBar::Inset`.
  - Changing the inset to 8 fails the second test.

## Real windows

Captured with `s3.py` (debug build, scratch HOME and `SCRIM_SETTINGS_DIR`, MCP on 7690). Each capture is one window by its id (`screencapture -l`, `window_id.swift`), never the screen. macOS 27.2, 2× display.

| Capture | Green button ends | First tab pixel | Gap |
|---|---|---|---|
| `light-1-home.png` | 68 pt | 76 pt (Home tab) | 8 pt |
| `light-2-editor.png` | 68 pt | 95.5 pt (Home icon; the tab isn't active) | 27.5 pt |
| `dark-1-home.png` | 68 pt | 76 pt | 8 pt |
| `dark-2-editor.png` | 68 pt | 95.5 pt | 27.5 pt |
| `dark-3-second-window.png` (from `window.new`) | 68 pt | 95.5 pt | 27.5 pt |

- The buttons span x 9–68 pt and y 9–23 pt.
- The captures were read with `scripts/ocr-captures.swift` before they were committed: no paths, names or addresses.

## Not checked

- **Full screen and a window being resized:** macOS hides or moves the buttons then, and the strip still leaves 76 pt.
- **Right-to-left layouts.**
- **Other macOS versions:** the button positions were measured on 27.2 only. Older versions draw smaller buttons, which fit in the same room.
- **The old TypeScript Scrim's label:** it was left alone. That repository is being archived (`docs/design/housekeeping.md`).
- **Windows and Linux:** they use `Custom`, with the buttons drawn by Scrim at the right. They weren't run here.
