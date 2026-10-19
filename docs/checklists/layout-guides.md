# Layout guides: Scrim's model on FigCraft's layout grids (Port 7)

The report: `docs/scrim-ports/layout-guides.md`. FigCraft's layout grids already cover Scrim's guides; these rows pin the mapping.

| Check | How | Covered by |
|---|---|---|
| Column and row tracks land where Scrim's do (all alignments, margins, gutters, offset frames, tracks past the frame) | Add columns to a frame (Design panel ▸ Layout grid) and compare with the numbers in `docs/scrim-transfer/cases/guides.json` | `crates/engine/tests/scrim_guides.rs` › `scrims_guides_map_onto_figcrafts_layout_grids` |
| Square grids draw Scrim's interior lines, plus the frame's own edges | Add a grid to a frame | the same test |
| Every line Scrim snaps to, FigCraft snaps to; hidden grids snap in neither | Drag a child of a frame with columns | `scrim_guides.rs` › `snap_lines_agree_inside_the_frame` |
