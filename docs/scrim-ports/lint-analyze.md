# Port 6: the design lint and `analyze_design` (19 Oct 2026)

**What came over:**
- **Scrim's deterministic design lint:**
  - 34 reported rules: 30 ported from Impeccable's anti-pattern detector, plus 4 of Scrim's own;
  - 45 known rule ids;
  - profiles and reasoned ignores;
  - the tools `lint_design`, `set_lint_profile` and `ignore_lint`.
- **Scrim's `analyze_design`:** colours, type styles, spacing values and repeated shapes, measured against the document's tokens.

**Sources:**
- the specs, `docs/scrim-transfer/specs/lint.md`, `lint-rules.md` and `analyze-design.md`;
- the cases, `docs/scrim-transfer/cases/lint.json` and `analyze-design.json`.

**Where it lives:**
- **The lint:** `crates/engine/src/cmd/lint/`. Its engine commands are `lint.run`, `lint.setProfile` and `lint.ignore`.
- **The analysis:** `crates/engine/src/cmd/analyze/`, engine command `design.analyze`.
- **MCP:** the tools `lint_design`, `set_lint_profile`, `ignore_lint` and `analyze_design`.
- **CLI:** `figcraft-cli lint` and `analyze`.
- **Queries:** `query_nodes` now reads `@lint` and `@lintCount`.
- **The hint:** every content-writing MCP tool's description, and the server instructions, end with Scrim's "run lint_design before reporting done" hint (`lint.md` §6.4).

## Against Scrim's cases

Both engines take their document through a small trait. The tests replay Scrim's cases on Scrim's own documents through the same rules, profiles and output code that serve FigCraft.

| Cases | Result |
|---|---|
| **Lint** (`lint.json`, 88 cases, 107 tool calls, 23 with measured rects; every rule, profiles, ignores, the config after each step, every error message) | **88 of 88 identical** (`cmd/lint/scrim_cases_tests.rs`) |
| **`analyze_design`** (8 cases on 5 documents: tokens, ΔE matches, clusters, `var()`, SVG paints, inheritance, the 4 px grid, repeats and the implied-group drop) | **8 of 8 identical** (`cmd/analyze/scrim_cases_tests.rs`) |
| **`query_nodes`'s two linter-dependent cases** (Port 5 had skipped them) | Now **identical**, through this port's lint on Scrim's documents. `query_nodes` is now **244/244 core, 19/19 tool, 25/25 CLI** |

**What it took to match exactly:** JavaScript's `toFixed` (ties away from zero), `Math.round` and number printing; UTF-16 lengths; the declaration order of a layer's keys (`style-shorthand-conflict` reports pairs in that order); and Scrim's quirks, kept as specified (`lint.md` §10).

## On FigCraft's documents

| Spec | FigCraft |
|---|---|
| A layer's declarations (`D`) | **The CSS view** (`cmd/lint/figcraft.rs` › `css`), what the export would carry. Text fill → `color` (a gradient: `color: transparent` plus `backgroundClip: text`, so `gradient-text` fires); other fills → `backgroundColor`, `background` (gradients) or `backgroundImage`; strokes → `border` / `border<Side>`; corner radius; auto layout → `display`, `flexDirection`, `gap`, `padding`; clip → `overflow: hidden`; absolute in auto layout → `position: absolute`; shadows → `boxShadow`; text style → `fontFamily`, `fontSize`, `fontWeight`, `lineHeight`, `letterSpacing`, `textTransform`, `textAlign` |
| Measured rects | **The document's own boxes** (page-space bounds), so the 7 rules that need boxes always run. Scrim needed an editor's measurement. |
| Root artboards | Each page's top-level layers. `root-overlap` compares roots on the same page. |
| Profiles and ignores (`doc.lint`) | **Layer data:** a root's `scrim.lint.profile`, a layer's `scrim.lint.ignores`, and the document-wide ignores on the first page. Each write is one undo step, in the change feed with its author (Port 3), saved with the file and shared live. Deleting a layer takes its ignores with it, and undo brings them back. |
| Hidden layers | **Not linted** (FigCraft has hidden layers; Scrim has none) |
| Rules that can't fire on FigCraft documents | `size-duplicated`, `style-shorthand-conflict` (FigCraft stores each property once) and the three Motion rules (FigCraft has no CSS transitions; its prototype animations could map onto `bounce-easing`, but that isn't done). All five run, and are tested on Scrim's documents. |
| `analyze_design`'s tokens | **Colours:** colour variables (their collection's default mode) and paint styles with one solid paint. **Typography:** text styles. **Spacing:** number variables scoped to gaps, or, with no scopes, named for spacing, gaps, padding or gutters. `repeated` leaves out components, sets and instances. |

**Tests on FigCraft:**
- `cmd/lint/figcraft_tests.rs` (5): findings from the CSS view and boxes; hidden layers; an ignore stored and undone; an expressive profile that still counts contrast; `@lint` in queries.
- `cmd/analyze/figcraft_tests.rs` (2).
- `crates/mcp/tests/lint_design.rs` (3): the tools, the hint, a round trip, read-only analysis.
- `apps/figcraft-cli/tests/query.rs`: `lint` and `analyze`.

## On real files (read in place; numbers, not verdicts)

| File | Active findings |
|---|---|
| **The owner's 24.6 MB sample** (753 layers) | **118:** `oversized-h1` 31, `low-contrast` 27, `text-occluded` 20, `single-font` 19 (once per artboard), `text-overflow` 16, `side-tab` 2, `clipped-overflow-container` 2, `all-caps-body` 1. **Not checked against the canvas:** some `low-contrast` and `text-occluded` findings may come from the CSS view (text over an image or a gradient reads the nearest solid background) |
| stage-a / -b / -c / -d / lines fixtures | 1 / 2 / 11 / 0 / 0. Mostly `overused-font`, because the fixtures use Inter |

**The sample's analysis:**
- **Colours:** 23 distinct; 10 match a token; 1 near-duplicate cluster.
- **Type:** 7 styles across 176 text layers; 6 match a text style.
- **Spacing:** 3 values, all on the 4 px grid.
- **Repeats:** 6 repeated shapes; the largest is 5 × 7 layers.

**A finding about FigCraft itself:** FigCraft's default text font is Inter, which is on Impeccable's overused-font list. Every new text layer draws `overused-font` until the font is changed or the rule is ignored document-wide.

## Licence duties (Apache-2.0 §4, `lint.md` §11), done

| Duty | How |
|---|---|
| The licence | Impeccable's `LICENSE` verbatim, at `crates/engine/src/cmd/lint/design-rules/LICENSE` (the same bytes as Scrim's copy) |
| Notice of modification | Every file with a ported rule says so in its header. `design-rules/NOTICE.md` lists the chain of changes (Scrim's, then this fork's) |
| Keep the attribution | The copyright line, the upstream URL, version 2.3.2 and the commit hash are in `rules.rs`, `NOTICE.md`, the repository `NOTICE` and `ATTRIBUTION.md` |
| Upstream NOTICE | **None exists** at that commit (checked through GitHub's API: 404), so there is nothing to carry |
| Trademarks | "Impeccable" appears only in attributions and the `lint_design` description Scrim wrote ("vendored Impeccable anti-patterns"), never as branding |

## Not ported, with estimates

| Item | Estimate |
|---|---|
| The 11 catalog rules Scrim never reported (`broken-image`, `skipped-heading`, `icon-tile-stack`, and 8 that are portable but were deferred) | 1–2 sessions for the 8 portable ones |
| Map FigCraft's prototype easings onto `bounce-easing` (spring and bounce curves) | 0.5 session |
| An editor panel for findings (Scrim's studio had one) | 2–3 sessions; UI work, not in this port's scope |
| `get_design_guidelines` (Impeccable's seven domain reference docs) | 0.5 session (data plus a tool) |
| Check the sample's contrast and occlusion findings against the canvas | 0.5 session |
