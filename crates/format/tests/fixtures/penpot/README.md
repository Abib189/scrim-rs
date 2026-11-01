# Penpot fixture

`sample.penpot` is a small Penpot export (`.penpot`, format v3: a zip of JSON members and
binary media) used by the Penpot import tests (`crates/format/src/penpot/tests/`).

## What it contains

One file ("Test file") with one page, "Poster" (background `#E8E8E8`), laid out as Penpot's
official documentation describes the format (manifest `version` 1, checked 2026-11-01):

- **Poster**: a 600 × 800 board holding
  - "Card": a rectangle with 24 px corners (`r1`–`r4`), a linear gradient fill and a shadow;
  - "Sun": a circle; "Wave": a path from SVG path data;
  - "Headline": a text layer;
  - "Badge": a group of a rounded rectangle with a stroke ("Pill") and a circle ("Dot");
  - "Photo": an image layer showing storage object `objects/0039433d-…e050.png` (an 8 × 8 PNG
    colour ramp);
  - "Call to action": an instance of the "Button" component (`componentId`), whose layer
    carries a `shapeRef`.
- **Button**: the main instance of the "Button" component (`files/…/components/`), a board with
  a rounded rectangle.
- A board thumbnail (`files/…/thumbnails/frame/…`) pointing at a 16 × 20 PNG storage object.

The text layer, the stroke, the shadow and the instance are what the documentation doesn't
describe, so the import reports them as warnings instead of guessing.

## How it was made

It is generated, not exported from Penpot: `penpot::tests::sample::sample()` builds every JSON
member and both PNGs in code and zips them with stored (uncompressed) entries and fixed
1980-01-01 timestamps, so the bytes never change. The test
`the_committed_sample_matches_the_builder` checks the committed file against the builder; after
changing the builder, rewrite it with

```sh
SCRIM_WRITE_PENPOT_FIXTURE=1 cargo test -p figcraft-format penpot
```

No one else's Penpot file was used or downloaded, and no Penpot source code was read.

## Licence

Made by the Scrim contributors, MIT OR Apache-2.0.

## `newer.penpot`

A manifest alone, `{"type": "penpot/export-files", "version": 9, "files": []}`, zipped with Python's `zipfile` (stored, 1980 timestamp). It stands for an export in a manifest version newer than the importer reads: the engine's registry test (`crates/engine/src/importers_tests.rs`) checks it is refused with a message saying so. Made by the Scrim contributors, MIT OR Apache-2.0.
