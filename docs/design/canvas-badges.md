# Canvas labels and status badges (Stage E)

Two collisions in the owner's capture: the Ridgeline app (`examples/ridgeline-app.figcraft`), dark theme, 22% zoom.
1. A large green "Ready for dev" pill covers the middle frame's title ("Trail — Light" shows as "…ght").
2. A black "Components" pill sits on the purple "Landscape" component label.

This page answers the E0 questions from the code at `89640c0`, then gives the plan for E1 and E2.

## 1. The "Ready for dev" pill

**What draws it:** `canvas::status_badge` (`crates/ui-egui/src/canvas.rs`, 1530 lines, frozen at that length). It is called from `frame_labels::draw` (`crates/ui-egui/src/frame_labels.rs`, 222 lines):
- for a frame, at `(end of the title + 6, top edge − 9)`;
- for a section, 6 px right of the section's name pill.

`canvas::status_label` picks the text and fill. The badge is white `semibold(10)` text on a filled 4-px-radius pill, padded 5 × 2 px.

**Whose pill it is:** it isn't the middle frame's. It belongs to **Home — Light**, the left frame, marked Ready for dev in the example.
- At 22% zoom, Home — Light is 86 px wide on screen.
- Its title is cut to that width (`edge.len`), and the pill is drawn after the cut title, about 70 px further right.
- Trail — Light starts 104 px from Home's left edge, so the pill lands on Trail's title.
- Nothing stops a frame's label from running past its frame, or checks it against a neighbour.

**Where it came from:** FigCraft (the fork's upstream). It was added in `bf3a45d`, "M1: sections and frames: resize to fit, section colours, ready-for-dev status (#23)", before the fork. The old TypeScript Scrim has no handoff status at all.

**States:** `figcraft_doc::DevStatus` (`crates/doc/src/node.rs`).

| Stored | Shown as | Pill colour now |
|---|---|---|
| `none` (the default; not written to the file) | nothing | — |
| `readyForDev` | "Ready for dev" | `DEV_READY` #14AE5C |
| `readyForDev` with `dev_changed: true` (its contents changed after it was marked) | "Changed" | `DEV_CHANGED` #D97A06 |
| `completed` | "Completed" | `DEV_COMPLETED` #6E6E73 |

There is no "in progress" state. "Changed" is set by the engine when a ready layer's contents change; a handoff snapshot is kept (`crates/doc/src/handoff.rs`).

**How it is set:**
- **UI:** the right panel's status menu for frames and sections (`section_panel.rs`, which runs `node.set {devStatus}`). "Resolve" (`dev.resolveChange`) clears "Changed".
- **Commands:** `node.set {devStatus: none|readyForDev|completed}` (`cmd/props.rs`), and `dev.resolveChange` (`cmd/section.rs`).
- **MCP:** `set_properties` with `devStatus`, or `run_command`. `inspect.readyForDev` lists them.
- **Import:** the .fig import doesn't read it. The .fig export writes it (`format/src/fig/export.rs`).

**Where it is stored:** on the node, as `dev_status`, `dev_changed` and `dev_note` (serde names in `document.json`, e.g. `"dev_status": "readyForDev"`). Frames and sections only.

**Clicking it:** the pill isn't part of any label's hit area.
- The area a label publishes (`ToolState::labels`) ends at the title for a frame, and is the name pill for a section.
- So a click on the badge falls through to whatever is under it: the canvas, a frame, or another frame's label.
- In the owner's capture it selects **Trail — Light**, the label it covers.

## 2. The black "Components" pill

**What it is:** the **name label of a section**. "Components" is a `section` node (id 349, at (−1300, 20), 853 × 1096).
- It isn't a hover or selection label: it is drawn always, on every frame.
- `frame_labels::draw` draws a section's name as a pill **inside** its top-left corner, `semibold(11)` on `t.hover`.
- `t.hover` is #1B1C1F in the dark theme, which reads as black on the page. When the section is selected, the pill is filled with the accent.

**Why it sits on "Landscape":** Landscape is a component set at (40, 40) inside the section, and it is labelled too, because sections' children are labelled like top-level frames (`canvas::boards`).
- Its label sits 16–18 screen px above its top edge, at a fixed screen size, while the 40-unit offset shrinks with the zoom.
- The section's pill covers 0–21 px down from the section's top and about 90 px to the right of its corner.
- So the two overlap below roughly 97% zoom: whenever 40 × zoom is less than the pill's height plus the label's height.
- At 22%, Landscape's label is about 9 px inside the section's corner, right on the pill.

## 3. Every label and badge on the canvas around a frame or component

All of them are drawn on the canvas painter in screen pixels at a fixed size: none scales with the zoom. Only two rows have any collision handling: measurement labels, and annotation callouts among themselves. **There is no shared layout for frame, section and component labels.**

| What | Owner | Anchor | Offset | Width limit | Collision handling |
|---|---|---|---|---|---|
| Frame title | `frame_labels::draw` | the frame's top-left as seen (rotates with the frame) | text top 16 px above the edge; hit area 18 px above to the text's bottom | the frame's on-screen width, but never under 20 px (so it overflows narrow frames) | none |
| Component / component set title, with the purple diamond (12 px) | `frame_labels::draw` | same | diamond at x, name at x + 14 | same | none |
| Section name pill | `frame_labels::draw` | **inside** the section's top-left | at the corner, padded 8 × 4 (+16 for the hidden-contents eye) | the section's width − 16, 20…480 px | none |
| Dev status pill | `canvas::status_badge` via `frame_labels::draw` | after the title (frames) or the section pill | +6 px; vertically centred on the label | **none**: runs past the frame | none; not in the hit area |
| "Contents hidden" | `frame_labels::draw` | a hidden section's centre | centred | shown when the section is over 120 × 60 px | none |
| Selection size pill ("240 × 1520") | `canvas::transform_box` / `canvas::pill` | the selection box's bottom centre | 9 px below | none | measurement labels avoid it |
| Measurement labels (Alt) | `measure_overlay::draw` / `place_labels` | on the measured line | centred, then slid or beside it | — | **yes**: each other and the size pill |
| Slot labels | `canvas.rs` (slot outlines) | above a slot's top-left | h + 2 px above | hidden when the slot is narrower than the label | none |
| "Swap instance" pill (Alt-drop from Assets) | `canvas::swap_highlight` | above the instance's top-left | 4 px above | none | none |
| Guide offset tags | `guides.rs` | on a dragged guide | — | — | none |
| Padding and gap values | `handles::pink_label` | on the handle | centred | — | none |
| Comment pins | `comments.rs` (`paint_pin`) | the comment's anchor point | the bubble's tip on the point | — | none (pins sit over labels) |
| Dev Mode annotation callouts | `dev_mode::draw` | beside the annotated layer | — | — | among themselves: a later callout moves below |
| Remote cursors, name tags, follow banner | `live.rs` | the person's pointer or caret; the canvas top | — | — | none |
| Rulers | `canvas::rulers` | canvas edges | — | — | chrome, not labels |

`canvas::boards` decides which layers are labelled: the page's visible top-level layers, plus the visible children of sections whose contents show (sections nest). The focus view (`dev.focus`) labels only its frame, in its own bar. Each label's area (its four corners) goes to the engine as `ToolState::labels`; `tools::label_at` makes a press there select and drag the frame, and a double-click renames it.

**A third finding the brief didn't name: the label colours ignore the canvas they sit on.**
- The canvas behind the labels is the **page's background**, not the theme's. Ridgeline's page is #1E1E1E in both themes, and new pages get #F5F5F5 or #1E1E1E depending on the theme they were made in.
- Labels take the **UI theme's** ink. In the light theme, Ridgeline's titles are #44474C on #1E1E1E: 1.79:1.
- The purple component text (#9747FF) is under 4.5:1 on every one of these backgrounds (3.7–4.1:1).
- A selected label's accent (#FF5A1F) is 2.6:1 on the light canvas.

## 4. The old TypeScript Scrim (reference only; nothing copied)

`apps/studio/src/editor/artboardLabels.ts`: the "Name · width ‹tag›" line above each root frame. Its rules:
1. A label never runs past its frame's on-screen width.
2. It stops short (8 px) of the next label or frame that starts to its right in the same band. It is hidden when another frame covers its start, or another label starts at the same x; the earlier root keeps its label.
3. Under 48 px of room the label is hidden.
4. A trailing tag chip shows only when the whole line fits; otherwise the name is ellipsised.

It had no sections, no component labels and no handoff status. Labels were 10 px mono in one ink (#6E6E66).

## 5. The plan

### E1: the status becomes a small icon

**Size and place:**
- 14 × 14 screen px at every zoom, drawn in code (no image).
- In the title row, to the right of the title with a 4 px gap.
- The title is ellipsised first, so it ends before the icon. The icon stays inside the label's room, which never runs past the frame.

**One shape per state** (not colour alone), new primitives drawn in the new module:

| State | Shape | Colour |
|---|---|---|
| Ready for dev | a ring with a check inside | `success` |
| Changed | a ring half filled (left half) | `warning` |
| Completed | a solid disc with the check cut out | `text_secondary` (neutral) |

**Colours** come from the token set that matches the **page background** (see Decision 3), at least 3:1 on both themes' canvases and both default page colours. The tooltip and the AccessKit name are the full status ("Ready for dev", "Changed since it was marked ready for dev", "Completed").

**Clicking:** the icon becomes part of its own frame's label area, so a press selects and drags **its** frame, like the title (Decision 5).

**Hiding:**
- When the label's room can't hold the shortest title plus the icon, the icon goes first.
- When it can't hold the shortest title, both hide.
- The icon is never drawn without its title.

### E2: one layout step for canvas labels

**The module:** `canvas_labels.rs` lays out every board label of a frame each canvas frame, before any is drawn: frame titles, component titles, section titles and status icons. The pure part (rectangles in, rectangles out) is testable without egui.

**Section titles** move outside, above the section's top edge, in the same family as frame titles: regular 11, the label ink, the eye icon when contents are hidden, and no filled pill. They are always-on titles, not hover or selection labels. Inside the corner they collide with any child placed within about 40 px of the section's top at ordinary zooms, and they hid the section's own content.

**Rules:**
1. Each label's room is its frame's on-screen width, from its left edge.
2. **Priority**, highest first:
   1. section titles, outer before inner;
   2. selected labels;
   3. hovered labels;
   4. the rest, in layer order (bottom first, as the old app's "the earlier root keeps its label").
3. Labels are placed in that order. A label goes in the leftmost free stretch of its room: a placed label covering its start pushes it right, past that label plus 8 px. A placed label further right cuts its room 8 px before it.
4. In the room it gets, the title is ellipsised (`bidi::name_galley` keeps a right-to-left name's start), then the icon follows (E1).
5. A label hides when its room is under 24 px. A hidden label has no hit area.
6. **Rotated frames** keep today's placement along their turned edge, and their box counts as taken for upright labels.

**Measurement labels and annotation callouts** keep their own placement. They are transient, and the brief names title, component, section and status labels.

### E3 checks

- Real window captures before and after.
- A rectangle test: no two published label areas overlap, over Ridgeline at 10, 22, 50 and 100%, a long title, a narrow frame, an RTL title, each state, and a section with a child at its corner.
- A pixel diff: outside the label bands, nothing changes.
- A contrast test of every label ink and icon colour on both canvases and both default page colours.
- A round trip of the status through save and open, undo and redo, and MCP, with the format unchanged.

## 6. Where I disagree with the brief, or add to it (overrule any)

1. **The pill isn't drawn on the middle frame.** It is Home — Light's badge, spilling over. The root cause is that a frame's label (title plus badge) can run past its frame. E2's room rule fixes that for every label, not only badges.
2. **The states are none, Ready for dev, Changed and Completed;** there is no "in progress". Changed gets the half-filled ring the brief suggested for in progress, because it is the "not quite" state.
3. **The label inks follow the page background, not the UI theme.** "3:1 against both themes' canvas backgrounds" can't hold with theme inks: the canvas is the page's own colour, and Ridgeline's is dark in both themes.
   - The labels and icons take the dark ink set on a dark page and the light set on a light one: the set with the better contrast against the actual background.
   - The purple component title gets a darker purple on light pages and a lighter one on dark pages, so it reaches 4.5:1. The diamond keeps the component colour where that is at least 3:1.
   - A selected or hovered title uses the accent text colour of the matching set.
4. **The section title moves outside the section, not only restyled.** Restyled inside, it would still collide with children near the corner.
5. **The icon is clickable as part of its own frame's label.** Before, the pill wasn't clickable and a click fell through, in the capture to another frame's label. The brief says to keep the old click behaviour; falling through to a neighbour isn't worth keeping.
6. **Labels can now hide** when their frame is too narrow on screen (under 24 px of room). Before, a title never got narrower than 20 px and overflowed a narrow frame. A hidden label can't be clicked; the frame itself still can.
7. **The Dev Mode panel's list keeps its status pill** (`dev_mode.rs`). It is panel chrome with room for words, not the canvas.
8. **No zoom threshold of its own.** Labels hide by on-screen room, which covers "below a minimum zoom" and also narrow frames at high zoom.

## E1 as built

- **`dev_icon.rs`** (new): `State` (Ready, Changed, Completed) from the node, the three icons drawn in code, `Ink::for_page` (the theme set that reads on the page's background), the icon's tooltip and AccessKit node (`Image`, named "Ready for dev", "Changed" or "Completed"), and the Dev Mode panel's worded pill. That pill moved out of `canvas.rs`, which shrinks from 1530 to 1509 lines.
- **`frame_labels::draw`**: the title, then the icon 4 px after it, centred in the title row, 2 px above the frame's top edge. `fit` shares the frame's on-screen width between them: the title is cut first, then the icon goes, then the label (under 20 px).
  - The icon is part of the label's area, so pressing it selects and drags its frame.
  - A hidden label publishes no area.
- **The section title moved above the section** (no pill) in this step. The icon must sit outside the frame and right after the title, which a title inside the section can't give.
- **Not yet:** the shared layout between labels, and the label text inks (E2).
