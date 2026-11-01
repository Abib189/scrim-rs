# Sketch fixture

`sample.sketch` is a small Sketch document used by the Sketch import tests
(`crates/format/src/sketch/tests/`).

## What it contains

Two pages, written to the published Sketch file format (JSON Schema of
`@sketch-hq/sketch-file-format` 6.5.0, document version 146):

- **Poster**: one 600 × 800 artboard ("Poster") holding
  - "Card": a rectangle with 24 px corners, a linear gradient fill, a 2 px inside border and a
    drop shadow;
  - "Sun": an oval; "Burst": an 8-point star; "Wave": a closed vector path with a curved point;
  - "Headline": a text layer with two runs (Inter Black and Inter Regular, two colours);
  - "Badge": a group of a rounded rectangle and a text layer;
  - "Photo": a bitmap layer showing `images/photo.png` (an 8 × 8 PNG colour ramp);
  - "Call to action": an instance of the "Button" symbol with a text override, and a prototype
    link;
  - things the import can't bring across, so its report has entries: a hotspot ("Tap area"), the
    prototype link on "Call to action" and a layer of an unknown `_class` ("Hologram").
- **Symbols**: the "Button" symbol master (a rounded rectangle and a text layer).

## How it was made

It is generated, not drawn in Sketch: `sketch::tests::sample::sample()` builds every JSON member
and the PNG in code and zips them with stored (uncompressed) entries and fixed 1980-01-01
timestamps, so the bytes never change. The test `the_committed_sample_matches_the_builder`
checks the committed file against the builder; after changing the builder, rewrite it with

```sh
SCRIM_WRITE_SKETCH_FIXTURE=1 cargo test -p figcraft-format sketch
```

No one else's Sketch file was used or downloaded.

## Licence

Made by the Scrim contributors, MIT OR Apache-2.0.
