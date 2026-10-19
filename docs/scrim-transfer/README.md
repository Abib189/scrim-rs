> **In scrim-rs:** this folder is a copy of `docs/transfer/` from Scrim at commit `8a4cf86` (github.com/Abib189/scrim).
> - **The fixtures** referenced as `tests/fixtures/fig/*.fig` are in `fixtures/fig/` here.
> - **`generators/`** needs a Scrim checkout (set `SCRIM_REPO`) and is kept only for provenance.
> - **Edit the kit in scrim-rs from now on;** Scrim is archived.

# Scrim → scrim-rs transfer kit

Everything Scrim learned, in a form the Rust fork can use without reading Scrim's TypeScript.

- **The fork:** github.com/Abib189/scrim-rs, a fork of FigCraft at `a0d1162`, MIT OR Apache-2.0. It keeps Rust, egui and FigCraft's renderer. The DOM canvas does not carry over.
- **This repo** (TypeScript, React, Electron) is archived after the transfer, not deleted.

**Rules of the kit:**
- Specs and cases only, never Scrim source. The fork re-implements in Rust.
- Every expected output in `cases/` came from **running** Scrim's code at commit `f2b1bdb` (or its parent), not from hand-typing.
- Third-party material keeps its own licence. See "Licences" below.

## Contents

| Path | What | Use it for |
|---|---|---|
| `decisions.md` | The four LOCKED decisions and hard rules 1–18, each marked CARRY, ADAPT (with the Rust form and its enforcing test) or DROP (with why) | The fork's CLAUDE.md / AGENTS.md and its xtask checks |
| `decisions-architecture.md` | Every other architectural principle (A1–A60), with the same verdicts, plus 12 open questions for the human | Design reviews |
| `specs/query-nodes.md`, `specs/query-nodes-errors.md` | The `query_nodes` grammar (EBNF), attributes (`[MAP]` marks those that need a FigCraft mapping), limits, MCP tool and CLI, and 36 verbatim error templates | Port 5 |
| `cases/query-nodes.json`, `cases/README-query-nodes.md` | 288 cases (244 evaluator, 19 MCP, 25 CLI) over 5 documents | Port 5 tests |
| `specs/lint.md`, `specs/lint-rules.md` | The design lint: 45 rule ids (34 reportable, 7 of them needing measured rects), profiles, ignores, 3 tools, verbatim messages | Port 6 |
| `specs/analyze-design.md` | Every `analyze_design` output field and its computation | Port 6 |
| `cases/lint.json`, `cases/analyze-design.json` | 88 lint cases (each reportable rule positive and negative) and 8 analyze cases | Port 6 tests |
| `specs/fig-import.md`, `specs/fig-import-mapping.md` (+ security) | The `.fig` importer: stages, mapping (exact, approximate, skipped), security caps with verbatim refusals, the report format and the numbers to beat | Port 4 |
| `cases/fig-import.json` | Per-fixture expectations and every security case | Port 4 tests |
| `fixtures.md` | Every reusable fixture, with path, size, provenance and what it exercises | All ports |
| `specs/svg-allowlist.md`, `cases/svg.json` | The inline-SVG allow-list and the imported-file policy (refuse, never sanitise); 178 cases, 11 of them documented **gaps** Scrim accepts but should refuse | Port 8 |
| `specs/layout-guides.md`, `cases/guides.json` | The guides model and geometry, with the mapping to Figma-style layout grids; 62 core cases plus inspector cases | Port 7 |
| `ui/tokens.json`, `ui/contrast.json`, `ui/README.md` | The UI design system: 318 DTCG tokens (dark and light), 105 contrast pairs per theme, and the egui mapping | Port 1 |
| `html-fidelity/` | `fidelity.mjs`, the standalone canvas-against-HTML harness (playwright-core only), with its README and the 20 standard artboards | Port 2 and its xtask |

## Known gaps the kit records rather than fixes

Scrim is being archived, so these are recorded for the fork rather than fixed in Scrim:
- **SVG files:** Scrim's imported-SVG check accepts some files that rule 18 says to refuse, for example an `xml-stylesheet` or DTD with an external href, entity-encoded `javascript:`, or `<foreignObject>` with an iframe. Showing them as `<img>` under a strict CSP makes them harmless in Scrim. The fork must refuse them (`cases/svg.json`, `"gap": true`).
- **query_nodes:** misleading error texts, and positions counted in UTF-16 (`specs/query-nodes.md` › quirks).
- **Lint:** rules read a node's own values, never inherited ones; the shadow-blur parse misses some shadows; there are JavaScript `toFixed` rounding hazards (`specs/lint.md` › quirks).
- **UI:** the light theme's modifier pill is 1.00:1 (unreadable), and a few secondary pairs fall under AA (`ui/README.md` §9).
- **The egui mapping** in `ui/README.md` uses egui 0.31/0.32 field names. The fork pins **egui 0.36** (`Cargo.toml`), so check the renames there.

## Licences

- **Scrim's own specs and cases:** contributed to the fork under its MIT OR Apache-2.0 terms.
- **The lint rules and their messages:** ported from Impeccable (Apache-2.0, Copyright 2025 Paul Bakaus; `packages/engine/design-rules/`). The fork must ship that licence text and a modification notice in each ported file, and carry the notice forward.
- **The fixtures:** synthetic, built by `packages/fig-import/src/__tests__/fixtures.ts`.
- **The 24.6 MB `.fig` sample:** belongs to the human. It is not in this repo and must never be committed.

## Regenerating the cases

The generators are in `generators/`. They import Scrim's own `packages/*/src` (or `dist`, after `pnpm build`) through repo-relative paths. To point them at another checkout, set `SCRIM_REPO`.

| Generator | Writes | Run |
|---|---|---|
| `gen-query-cases.ts` (+ `tools.ts`) | `cases/query-nodes.json` | `bun generators/gen-query-cases.ts` |
| `lint-analyze/cases-lint.mjs`, `cases-analyze.mjs`, `fill-rules.mjs` | `cases/lint.json`, `cases/analyze-design.json`, the messages in `specs/lint-rules.md` | `node …` (needs `dist`) |
| `gen-fixtures.ts`, `gen-security.ts`, `merge.ts` | `cases/fig-import.json` | `bun …`, in that order |
| `gen-svg.ts`, `gen-guides.ts` | `cases/svg.json`, `cases/guides.json` | `bun …` |
| `resolve.mjs`, `build.mjs`, `derive.ts` | `ui/tokens.json`, `ui/contrast.json` | needs `playwright-core` + Chromium |
| `makeref.mjs`, `figcraft-pairs.mjs` | test references and the FigCraft pairing recipe for `html-fidelity/` | — |

**Verified:** `gen-guides.ts`, run from `generators/`, reproduces `cases/guides.json` byte for byte. The others were run from their original location with the same code; only the paths were rewritten.
