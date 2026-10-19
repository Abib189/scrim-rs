# Notice: the design rules in `crates/engine/src/cmd/lint/`

This directory's parent module includes design rules from **Impeccable** by Paul Bakaus (github.com/pbakaus/impeccable), licensed under the Apache License, Version 2.0. A copy of that licence is in `./LICENSE`, unchanged.

- **Upstream:** Impeccable v2.3.2, commit `1863a44b23bfcf28e5990c0068a0603995ab1e29`. Copyright 2025 Paul Bakaus.
- **Upstream NOTICE:** Impeccable has no NOTICE file at that commit (checked 19 Oct 2026), so there are no upstream attribution notices to carry (Apache-2.0 §4(d)).

## What is Impeccable's

- **The catalog** (`rules.rs`): the name and description of each of the 41 catalog rules, and their categories and domains.
- **The detection logic** of the 30 ported rules: `helpers.rs`, `checks.rs`, `rendered.rs`, `aggregate.rs` and `motion.rs`. This includes the overused-font and buzzword lists, and the thresholds.

## The chain of modifications (Apache-2.0 §4(b))

1. **Scrim** (github.com/Abib189/scrim, archived) ported the detector to TypeScript to run against its document model instead of a live DOM, and changed it:
   - redefined `gray-on-color`;
   - added the `expressive` profile relaxations;
   - ported rules over measured rects (`clipped-overflow-container` narrowed to text and images that are mostly clipped away);
   - ported the Motion rules over its motion model (`bounce-easing` also catches overshooting `cubic-bezier()` and non-monotonic `linear()`);
   - added its own rules: `text-occluded`, `root-overlap`, `size-duplicated` and `style-shorthand-conflict`.

   The browser, screenshot and visual engines were not ported.
2. **This fork** re-implemented Scrim's port in Rust from Scrim's specification (`docs/scrim-transfer/specs/lint*.md`). No TypeScript was copied. The behaviour on Scrim's documents is unchanged: Scrim's 88 recorded cases pass. The fork's further changes:
   - the rules read FigCraft's layers through a CSS view of each layer (`figcraft.rs`);
   - hidden layers are not linted;
   - profiles and ignores are stored in layer data.

No Impeccable branding is surfaced to users. "Impeccable" is not used as a product name (Apache-2.0 §6).
