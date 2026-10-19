# Port 8: SVG, refused never sanitised (19 Oct 2026)

**Sources:**
- the spec, `docs/scrim-transfer/specs/svg-allowlist.md`;
- the cases, `docs/scrim-transfer/cases/svg.json` (178).

## The two layers, and what they map to here

Scrim had two SVG surfaces (spec §0):

| Scrim | Here |
|---|---|
| **Inline vector content:** structured JSON on a node, checked by a strict allow-list | **No counterpart.** This app stores vectors as vector networks, never as SVG data, so there is no SVG on a layer to allow-list. Agents create vectors from **path data** (`node.create {type: vector, path}`), which is checked against Scrim's path cases. |
| **Imported `.svg` files:** a deny-list text sniff, then shown only as an `<img>` (no script, no fetch) | **The SVG importer** (`crates/svg/src/import/`). It **parses** the file into layers, so the file layer matters more here than in Scrim. |

## What changed

1. **The gate** (`crates/svg/src/import/gate.rs`). It runs before any parsing, on every SVG import: both entry points (`cmd/file.rs` opening a file and `file.import`), which File ▸ Open, File ▸ Import, paste, drop and MCP's `open_file` all reach. An SVG embedded in an `<image>` data URI goes through the same gate inside the image resolver, because the outer text scan can't read base64. A refusal there refuses the whole import: `… (in an embedded SVG): …` (`an_embedded_svg_is_gated_too`).
   - **Scrim's six tests,** in Scrim's order, with Scrim's reasons: entities, script elements, `@import`, external `url()`, event handlers, external `href` / `src`.
   - **The gaps of Scrim's §6, closed:** Scrim showed files only as images, but this app parses them.
     - a stylesheet processing instruction;
     - a `SYSTEM` DTD;
     - schemes spelled with character references (`&#106;avascript:`);
     - SMIL writing a link (`to`, `from`, `by`, `values`);
     - relative references to other files (`other.svg#a`, `url(other.svg#g)`);
     - `foreignObject`;
     - a handler after a slash (`<svg/onload=…>`).

     Unbounded nesting was already bounded by the importer's own cap of 512.
   - **The message:** `<reason>: only self-contained SVGs are imported`. Nothing is imported.
   - **Before this port**, the importer parsed such files and dropped what it couldn't use. A script was ignored, a linked image was left unresolved, and the import looked fine. That is sanitising, which Scrim's rule 18 forbids.
2. **Path data:** numbers after `Z` are now refused (`cmd::create::parse_path_data`), as the SVG grammar and Scrim's parser refuse them. kurbo's parser accepted them.

## Against Scrim's cases

| Cases | Result |
|---|---|
| **`file`, unsafe** (23: scripts, plain and namespaced, upper case, inside a comment; handlers; https, http, `file:`, `//` and `javascript:` links; an iframe `src`; external `url()` in style; `@import`; internal, external and parameter entities; the Illustrator export) | **23 of 23 refused**, each with Scrim's reason (`scrim_cases_tests.rs`) |
| **`file`, gaps** Scrim accepted and reported (11) | **11 of 11 refused.** 10 by the gate; 50,000 nested groups by the importer's nesting cap |
| **`file`, safe** (12: icons, prolog and doctype, BOM, units, internal references, a style element, a `data:` image, SMIL) | **12 of 12 import** |
| **`file`, about images, not safety** (11: not SVG at all, over 25 MB, over 16,384 px a side) | **Not applicable.** Files that aren't SVG fail to parse, so they are refused here too. The size and pixel caps belong to Scrim's raster gate; this importer has its own budgets (64 MB, 100,000 layers, nesting 512). |
| **`inline`, accepted** (15; markup recorded for 12) | **The 12 vectors Scrim generated import here** (`scrims_generated_vectors_import`) |
| **`inline`, refused** (82) | **Not applicable:** they test Scrim's JSON vector data, which has no counterpart here |
| **`pathEdit`** (17) | **17 of 17 match** (`crates/engine/tests/scrim_paths.rs`): the 11 Scrim reads give the same shape (bounds within 0.02 after Scrim's rounding), and the 6 it refuses are refused. One of the 6 needed the fix above. |
| **`editRefusal`** (5) | **Not applicable:** this app's vector edit mode opens any vector, with its own rules |

## Decisions for the owner

1. **Illustrator exports are refused,** as in Scrim. They declare namespace entities, which the entity test catches. It's a known false positive (spec §6) and a real-world cost. The alternative: allow internal entities whose values hold no markup, and refuse only external and parameter entities.
2. **The standard SVG 1.1 `PUBLIC` doctype is accepted,** as in Scrim, although it names an http URL. Nothing ever fetches it. A `SYSTEM` DTD is refused.
3. **The two FigCraft tests that expected a linked image to import unresolved** now expect a refusal (`structure_tests.rs`, `budget_tests.rs`). Opening a file that links another file is refused rather than imported with a hole.

## Not done

| Item | Estimate |
|---|---|
| Check whether plugins or the collaboration server can bring SVG in by another path (every import found goes through `import_svg_with_notes`; not exhaustively audited) | 0.5 session |
| The decision on Illustrator entities | 0.5 session once decided |
