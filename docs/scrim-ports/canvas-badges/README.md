# Stage E verification (E3, 20 Oct 2026): canvas labels and status icons

Real window captures (`ui.screenshot`, the window's own pixels; this display is 1×, 1440 × 900 points) of `examples/ridgeline-app.figcraft`.
- **`before/`:** the release build at `c260cb9` (E0, the code as the owner's report found it).
- **`after/`:** the release build with E1 and E2.

Each set comes from a fresh copy of the file and a scratch home folder. The same script drove both, with the same camera, theme, window size and pointer (parked over the Inspector, so nothing on the canvas is hovered). The plan and what was built: `docs/design/canvas-badges.md`.

## The captures

| Name | What |
|---|---|
| `ridgeline-{dark,light}-z010` | 10%, the whole page |
| `ridgeline-{dark,light}-z022` | 22%, the owner's report: Home — Light's "Ready for dev" pill on Trail — Light's title, "Components" on "Landscape" |
| `ridgeline-{dark,light}-z050-section`, `-z050-frames` | 50%: the Components section; the three screens |
| `ridgeline-{dark,light}-z100-section`, `-z100-frames` | 100%: the same two places |
| `states-{dark,light}-z022`, `-z050` | Every state at once: Home — Light **Ready for dev**, Trail — Light **Changed** (marked, then a layer in it changed), Home — Dark **Completed**, the Components section **Ready for dev** |
| `states-{dark,light}-z100-long` | Home — Dark renamed "Home — Dark, the evening variant with every trail card, filter chip and saved route shown" (Completed) |

**What to look for:**
- **Before:** the green pill runs from Home — Light's label onto Trail — Light's title. The black "Components" pill sits on "Landscape". In the light theme the titles are dark grey on Ridgeline's dark page and can hardly be read.
- **After:** a 14 px icon follows each marked title: a ringed check, a half-filled ring, or a solid disc with a check.
  - A title that doesn't fit ends in an ellipsis before its icon.
  - "Components" is a quiet title above the section. Landscape's label is below it from 50% up; at 22% Landscape has no room left beside the section's title and hides.
  - Titles take the ink that reads on what is behind them: light on the dark page, dark purple on the white section.

## Pixel diff: 0 px outside the labels

Each after capture against its before capture. A label region, for each labelled layer at that capture's camera, is:
- the 18 px band above its top edge, from its left edge to its right edge plus 110 px, where the old pill ran past;
- for the section, also where its old pill sat inside its corner.

| Capture | Changed pixels in label regions | Changed elsewhere on the canvas | Changed in the panels |
|---|---|---|---|
| ridgeline-dark-z010 | 3,598 | **0** | 0 |
| ridgeline-dark-z022 | 4,016 | **0** | 0 |
| ridgeline-dark-z050-section | 2,809 | **0** | 0 |
| ridgeline-dark-z050-frames | 1,077 | **0** | 0 |
| ridgeline-dark-z100-section | 2,556 | **0** | 0 |
| ridgeline-dark-z100-frames | 1,077 | **0** | 0 |
| ridgeline-light-z010 | 3,680 | **0** | 0 |
| ridgeline-light-z022 | 4,561 | **0** | 0 |
| ridgeline-light-z050-section | 2,818 | **0** | 0 |
| ridgeline-light-z050-frames | 1,831 | **0** | 0 |
| ridgeline-light-z100-section | 2,547 | **0** | 0 |
| ridgeline-light-z100-frames | 1,569 | **0** | 0 |
| states-dark-z022 | 7,018 | **0** | 0 |
| states-dark-z050 | 1,829 | **0** | 0 |
| states-dark-z100-long | 1,095 | **0** | 0 |
| states-light-z022 | 7,436 | **0** | 0 |
| states-light-z050 | 2,828 | **0** | 0 |
| states-light-z100-long | 2,564 | **0** | 0 |

An earlier after run wasn't clean, and was discarded.
- **What happened:** a Layers row picked up keyboard focus, which outlines its layer on the canvas. Before that, a lock toggled on Trail — Light, recorded as the person's edit.
- **Why:** the script brought the window to the front (`window.focus`), so keys typed on the machine at the time went to it.
- **The fix:** the kept after run leaves the window where it is, and its history holds only its own four status edits.
- **The before set** was taken with `window.focus`. It shows no such disturbance: outside the label regions, its canvas and panels match the clean after run pixel for pixel.

## No two labels overlap (geometry, not pixels)

`crates/ui-egui/src/canvas_labels_ui_tests.rs` takes the label areas the canvas publishes for hit testing, which include the icon. It asserts no two overlap, and that each sits within its frame's width just above it:
- Ridgeline at 10, 22, 50 and 100% (several centres each);
- a sweep of 40 zooms from 4% to about 330%;
- the scenario: the long title, every state, a right-to-left title, a 50-wide frame between two others, and a bar just above Home — Light, from 10% to 200%.

`canvas_labels_tests.rs` places 400 scattered labels of every priority, none overlapping. With the collision rule switched off, 7 of these tests fail.

## Contrast

Label inks follow what is behind the label: the page's colour, with any section fills over it. Text needs 4.5:1, icons (non-text) 3:1. `label_text_reads_on_both_canvases_and_both_default_pages`, `labels_inside_a_section_read_on_its_fill_not_the_page` and `every_status_icon_reads_on_both_canvases_and_both_default_pages` cover them.

| Behind the label | Title | Selected title | Component title | Ready | Changed | Completed |
|---|---|---|---|---|---|---|
| Light canvas #ECECEA | 7.89 | 4.59 | 5.34 | 4.59 | 4.47 | 7.89 |
| Default page #F5F5F5 | 8.56 | 4.98 | 5.79 | 4.98 | 4.85 | 8.56 |
| White section #FFFFFF (Ridgeline's Components) | 9.33 | 5.43 | 6.32 | 5.43 | 5.28 | 9.33 |
| Dark canvas #17181A | 7.63 | 5.70 | 7.35 | 6.12 | 8.12 | 7.63 |
| Dark page #1E1E1E (Ridgeline's page) | 7.16 | 5.35 | 6.90 | 5.75 | 7.62 | 7.16 |

Before, on Ridgeline's dark page in the light theme, titles were 1.79:1, and the component purple #9747FF was 3.7:1 there.

## The status round-trips unchanged

`crates/mcp/tests/dev_status.rs`:
- MCP `set_properties {devStatus}` and `get_node`; undo and redo; a change inside the frame making it Changed; save and open with Changed kept; Completed saved and reopened.
- The shipped example, saved before Stage E, opens with Home — Light Ready for dev.

No code under `crates/doc`, `crates/format`, `crates/engine` or `crates/mcp` changed in Stage E, so the `.figcraft` format is unchanged.
