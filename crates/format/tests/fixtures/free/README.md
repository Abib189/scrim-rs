# FREE fixture

`sample.free` is a small Lunacy FREE document used by the FREE import tests
(`crates/format/src/free/tests/`).

## What it contains

Two pages, written to the FREE format specification (the README of
<https://github.com/icons8/free> at commit `9b2fc846453f1917216a8d8202e2f31d48996115`, format
version 11):

- **Poster**: one 600 × 712 frame ("Poster art") with a vertical auto layout (fixed width,
  hugging height, 24 px spacing, 40 px padding) holding
  - "Hero": a rectangle with 24 px corners, a linear gradient fill and a drop shadow, filling the
    frame's width (`stretchWidth`);
  - "Heading": a nested frame with a horizontal auto layout that hugs its contents, holding
    "Sun" (an oval) and "Title" (a text layer with a styled range in another colour);
  - "Photo": a rectangle with an image fill showing `images/photo.png` (an 8 × 8 PNG colour
    ramp);
  - "Call to action": an instance of the "Button" component with a text override;
  - things the import can't bring across, so its report has entries: a variable binding on
    "Hero", a prototype interaction on "Call to action" and a connector ("Note arrow").
- **Components**: the "Button" component (a rounded frame with a horizontal auto layout and a
  text layer).
- `preview.webp`: a 30 × 36 lossless WebP preview.

## How it was made

It is generated, not drawn in Lunacy: `free::tests::sample::sample()` builds every JSON member,
the PNG and the WebP in code and zips them with stored (uncompressed) entries and fixed
1980-01-01 timestamps, so the bytes never change. The test `the_committed_sample_matches_the_builder`
checks the committed file against the builder; after changing the builder, rewrite it with

```sh
SCRIM_WRITE_FREE_FIXTURE=1 cargo test -p figcraft-format free
```

No one else's FREE file was used or downloaded.

## Licence

Made by the Scrim contributors, MIT OR Apache-2.0.
