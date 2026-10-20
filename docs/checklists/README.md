# Manual checklists

Every capability has a test and a row here (AGENTS.md › Scrim's rules 4). There is one table per area, in `<area>.md`.

**A row names:**
- what a person checks by hand in the built app;
- the automated test that covers it (crate, file and test name), or `manual only` with the reason.

| Check | How | Covered by |
|---|---|---|
| What must hold | The steps in the app | `crate/src/file.rs` › `test_name` |

**Areas:**
- [`fork.md`](fork.md): the fork's own guarantees (branding, file size, platform fixes).

The Scrim ports add their own areas (`agent-safety.md`, `fig-import.md`, `query.md`, `lint.md`, `guides.md`, `svg.md`, `export-fidelity.md`), and some port Scrim's tables from `docs/scrim-transfer/`.
- `layers.md`: Stage D, the Layers panel's collapse all and click to focus.
- `canvas-labels.md`: Stage E, frame, section and component labels on the canvas and the handoff status icons.
- `follow-agent.md`: Stage I, following an agent's work with the camera, the reveal and the working indicator.
