# Stage G verification (G3, 20 Oct 2026): the accent everywhere

Real window captures (`ui.screenshot`, the window's own pixels).
- **How they were taken:** at 2× on this display, then saved at 1440 points wide for the repository. The pixel checks below compare the full-size originals.
- **Before and after:** `before/` is the release build at `d860207` (G0, the code as the report found it), `after/` the build with G1 and G2.
- **The same conditions:** both were taken by the same script, from a fresh copy of `examples/ridgeline-app.figcraft` and a scratch HOME, with the pointer parked over the Inspector. The home screens' path line is painted over: it showed the scratch folder, and with it a user name.

| Capture | What |
|---|---|
| `green-light-editor` | Green, light theme: Home — Light selected (toolbar, rail, Share, the selected layer, the canvas selection, handles, size label, the frame's title) |
| `green-light-ai-tab` | The same with the AI tab |
| `green-light-path-edit` | Path edit mode on a vector (the path and its anchors) |
| `green-light-marquee` | A marquee being dragged on the canvas |
| `green-light-home`, `green-light-settings` | The home screen; Settings ▸ General (the accent presets) |
| `blue-light-editor`, `pink-dark-editor` | The editor on two other presets: blue in light, pink in dark |
| `orange-{light,dark}-*` | The default, every view in both themes: the pixel check |
| `after/live-1-blue`, `after/live-2-teal` | The accent switched while a frame is selected, without input |

**Before, with green chosen:**
- **Orange:** Share, the active tool, the selection outline and its handles, the size label and the selected frame's title.
- **Already green:** the rail's File icon (it read `t.accent_text`), the selected layer's edge and the tab underline.

**After:** all of them follow, in light and dark. The brand marks stay brand orange: the "S" on the file tab, in the home sidebar, in Open files and on the recent file's badge.

## The default is unchanged (pixel check)

Each `orange-*` capture after against before:

| Capture | Pixels that differ |
|---|---|
| orange-light-editor, -ai-tab, -path-edit, -marquee, -settings | **0** |
| orange-dark-editor, -ai-tab, -path-edit, -marquee, -settings | **0** |
| orange-light-home, orange-dark-home | two text rows only: the open file's path and the recent file's name. The before run's copy was called `ridgeline-before`, the after run's `ridgeline-after` (the script's naming, not the code) |

Ridgeline's page is dark (#1E1E1E), where #FF5A1F reads at 5.35:1, so the overlays keep it in both themes. On a light page the default's overlays use a darker orange (#FF4F10 on #F5F5F5, 3:1). That is covered by tests (`which_presets_were_adjusted`, the pen test) and not captured.

## Live repaint without input

On `after/`:
1. Dark theme, blue, Home — Light selected (`live-1-blue`).
2. The accent was set to teal from the control channel, with no pointer or key input after it.
3. The app drew 4 frames on its own, and the next capture (`live-2-teal`) shows it.

| | Blue overlay pixels on the canvas | Teal overlay pixels |
|---|---|---|
| live-1-blue | 4,424 | 0 |
| live-2-teal | 0 | 4,427 |

## Not verified

- **A custom (non-preset) accent, live:** only the presets were captured. The contrast rule is the same, and `pref.accent {value}` refuses colours it can't derive.
- **Overlays on a light page, live:** tests only (see above).
- **Live in the window:** comment pins, the prototype player, Dev Mode compare, prototype connections, the colour picker's rows and the alignment grid. They are swapped and covered by the "no orange painted" test only where they show in the editor and AI tab.
- **Windows and Linux.**
