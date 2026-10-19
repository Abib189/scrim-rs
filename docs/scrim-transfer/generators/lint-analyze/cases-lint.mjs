// Generates docs/transfer/cases/lint.json by running Scrim's real lint_design /
// ignore_lint / set_lint_profile tools. Run: node cases-lint.mjs <outPath>
import { writeFileSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { mk, run, RECT } from './lib.mjs';

const OUT = process.argv[2];
const W = '#ffffff';
const INK = '#111111';

/** A white 800×600 artboard (flex column, no spacing) holding `children`. */
const board = (children, extra = {}) => ({
  id: 'board',
  type: 'frame',
  name: 'Board',
  canvas: { x: 0, y: 0 },
  layout: { display: 'flex', flexDirection: 'column' },
  style: { background: W, width: 800, height: 600 },
  children,
  ...extra,
});
const txt = (id, text, style = {}, extra = {}) => ({ id, type: 'text', name: extra.name ?? id, text, style: { fontSize: 16, color: INK, fontFamily: 'Söhne', ...style }, ...extra });
const box = (id, style = {}, extra = {}) => ({ id, type: 'box', name: extra.name ?? id, style, ...extra });
const frame = (id, layout, style, children = [], extra = {}) => ({ id, type: 'frame', name: extra.name ?? id, layout, style, children, ...extra });
const one = (children, extra) => mk({ roots: [board(children, extra)] });
const BOARD_RECT = { board: RECT(0, 0, 800, 600) };

const cases = [];
const add = (c) => cases.push(c);

// ── model-only rules: positives and negatives ─────────────────────────────
add({ name: 'side-tab/positive', rules: ['side-tab'], polarity: 'positive', description: 'A 4px coloured left border with no other border (radius 0).',
  doc: one([box('card', { borderLeft: '4px solid #2563eb', width: 300, height: 80 })]) });
add({ name: 'side-tab/negative-uniform-border', rules: ['side-tab', 'border-accent-on-rounded'], polarity: 'negative', description: 'All four sides 4px: no side is ≥ 2× the others.',
  doc: one([box('card', { border: '4px solid #2563eb', width: 300, height: 80 })]) });
add({ name: 'side-tab/negative-neutral-colour', rules: ['side-tab'], polarity: 'negative', description: 'A thick left border in a neutral grey (channel spread < 30) never fires.',
  doc: one([box('card', { borderLeftWidth: 6, borderLeftColor: '#cccccc', width: 300, height: 80 })]) });
add({ name: 'side-tab/quirk-borderWidth-group-ignored', rules: ['side-tab', 'border-accent-on-rounded'], polarity: 'positive', description: 'QUIRK: the side test reads `border`, `borderX` and `borderXWidth`/`borderXColor` only. The group shorthands `borderWidth: 3` / `borderColor` are NOT read, so the other three sides count as 0px with no colour and a 4px left side on an evenly 3px-bordered card still fires (rounded, so border-accent-on-rounded). In CSS the left side is not an accent (4 < 2 × 3).',
  doc: one([box('card', { borderWidth: 3, borderColor: '#7c3aed', borderLeftWidth: 4, borderLeftColor: '#7c3aed', borderRadius: 16, width: 300, height: 80 })]) });
add({ name: 'border-accent-on-rounded/positive', rules: ['border-accent-on-rounded'], polarity: 'positive', description: 'The same accent on a rounded element reports border-accent-on-rounded instead of side-tab.',
  doc: one([box('card', { borderTop: '3px solid rgb(220, 38, 38)', borderRadius: 12, width: 300, height: 80 })]) });
add({ name: 'border-accent-on-rounded/negative-thin', rules: ['border-accent-on-rounded', 'side-tab'], polarity: 'negative', description: 'A 1px accent is under the 2px minimum.',
  doc: one([box('card', { borderLeft: '1px solid #2563eb', borderRadius: 12, width: 300, height: 80 })]) });

add({ name: 'overused-font/positive', rules: ['overused-font'], polarity: 'positive', description: 'The primary (first) family, unquoted and lower-cased, is in the overused list. Fires on ANY node type (here a frame and a text).',
  doc: one([frame('stack', { display: 'flex' }, { fontFamily: "'Geist Sans', system-ui" }, [txt('t', 'Hello', { fontFamily: '"Inter", sans-serif' })])]) });
add({ name: 'overused-font/negative', rules: ['overused-font'], polarity: 'negative', description: 'A family outside the list; an overused family in second place does not count.',
  doc: one([txt('t', 'Hello', { fontFamily: 'Söhne, Inter, sans-serif' })]) });

add({ name: 'single-font/positive', rules: ['single-font'], polarity: 'positive', description: 'Two text nodes, one family across the whole subtree. Attributed to the lint target (the root).',
  doc: one([txt('h', 'Pricing', { fontSize: 32 }), txt('p', 'Simple pricing for every team.')]) });
add({ name: 'single-font/negative', rules: ['single-font'], polarity: 'negative', description: 'Two families.',
  doc: one([txt('h', 'Pricing', { fontSize: 32, fontFamily: 'Canela' }), txt('p', 'Simple pricing for every team.')]) });

add({ name: 'flat-type-hierarchy/positive', rules: ['flat-type-hierarchy'], polarity: 'positive', description: 'Three text nodes, sizes 16/18/16: max/min = 1.125 < 1.25.',
  doc: one([txt('a', 'One', { fontFamily: 'Canela' }), txt('b', 'Two', { fontSize: 18, fontFamily: 'Söhne' }), txt('c', 'Three', { fontFamily: 'Plex' })]) });
add({ name: 'flat-type-hierarchy/negative', rules: ['flat-type-hierarchy'], polarity: 'negative', description: 'Sizes 16/32/16: ratio 2.',
  doc: one([txt('a', 'One', { fontFamily: 'Canela', fontSize: 32 }), txt('b', 'Two', { fontFamily: 'Söhne' }), txt('c', 'Three', { fontFamily: 'Plex' })]) });

add({ name: 'gradient-text/positive', rules: ['gradient-text'], polarity: 'positive', description: 'A: gradient background clipped to text. B: gradient backgroundImage with transparent text colour.',
  doc: one([
    txt('a', 'Metrics', { fontSize: 32, background: 'linear-gradient(90deg, #f0f, #0ff)', webkitBackgroundClip: 'text', color: 'transparent' }),
    txt('b', 'Growth', { fontSize: 32, fontFamily: 'Canela', backgroundImage: 'radial-gradient(#f0f, #0ff)', color: 'transparent' }),
  ]) });
add({ name: 'gradient-text/negative', rules: ['gradient-text'], polarity: 'negative', description: 'A gradient fill on a box, not clipped to text.',
  doc: one([box('hero', { background: 'linear-gradient(90deg, #ffd27a, #ff7a59)', width: 400, height: 200 })]) });

add({ name: 'repeating-stripes-gradient/positive', rules: ['repeating-stripes-gradient'], polarity: 'positive', description: 'repeating-linear-gradient in background.',
  doc: one([box('stripes', { background: 'repeating-linear-gradient(45deg, #111111 0 10px, #222222 10px 20px)', width: 200, height: 200 })]) });
add({ name: 'repeating-stripes-gradient/negative', rules: ['repeating-stripes-gradient'], polarity: 'negative', description: 'A plain linear gradient.',
  doc: one([box('fade', { backgroundImage: 'linear-gradient(#111111, #222222)', width: 200, height: 200 })]) });

add({ name: 'gpt-thin-border-wide-shadow/positive', rules: ['gpt-thin-border-wide-shadow'], polarity: 'positive', description: '1px border and a box-shadow whose third px number (blur) is ≥ 24.',
  doc: one([box('card', { border: '1px solid #e5e5e5', boxShadow: '0px 8px 32px rgba(0, 0, 0, 0.12)', width: 300, height: 120 })]) });
add({ name: 'gpt-thin-border-wide-shadow/negative-small-blur', rules: ['gpt-thin-border-wide-shadow'], polarity: 'negative', description: 'Blur 16 < 24.',
  doc: one([box('card', { border: '1px solid #e5e5e5', boxShadow: '0px 4px 16px rgba(0, 0, 0, 0.12)', width: 300, height: 120 })]) });
add({ name: 'gpt-thin-border-wide-shadow/quirk-unitless-zero', rules: ['gpt-thin-border-wide-shadow'], polarity: 'negative', description: 'QUIRK: only tokens ending in px are counted, so `0 8px 32px` has two px numbers and the blur reads as 0 — not flagged although the shadow is wide.',
  doc: one([box('card', { border: '1px solid #e5e5e5', boxShadow: '0 8px 32px rgba(0, 0, 0, 0.12)', width: 300, height: 120 })]) });

add({ name: 'cramped-padding/positive', rules: ['cramped-padding'], polarity: 'positive', description: 'A filled container with children and padding 6px (0 < p < 8).',
  doc: one([frame('chip', { display: 'flex', padding: 6 }, { background: '#f4f4f5' }, [txt('t', 'Beta')])]) });
add({ name: 'cramped-padding/negative', rules: ['cramped-padding'], polarity: 'negative', description: 'A: padding 12. B: padding 6 but no background or border. C: padding as a two-value shorthand (not a single px value, so not read).',
  doc: one([
    frame('a', { display: 'flex', padding: 12 }, { background: '#f4f4f5' }, [txt('ta', 'Beta', { fontSize: 32 })]),
    frame('b', { display: 'flex', padding: 6 }, {}, [txt('tb', 'Beta', { fontFamily: 'Canela' })]),
    frame('c', { display: 'flex', padding: '4px 6px' }, { background: '#f4f4f5' }, [txt('tc', 'Beta', { fontFamily: 'Plex' })]),
  ]) });

add({ name: 'nested-cards/positive', rules: ['nested-cards'], polarity: 'positive', description: 'A card (fill, radius ≥ 8, padding > 0, children) inside another card. Reported on the inner card.',
  doc: one([frame('outer', { display: 'flex', padding: 16 }, { background: '#f4f4f5', borderRadius: 12 }, [frame('inner', { display: 'flex', padding: 12 }, { background: W, borderRadius: 8 }, [txt('t', 'Nested')])])]) });
add({ name: 'nested-cards/negative', rules: ['nested-cards'], polarity: 'negative', description: 'Inner radius 4 < 8: not a card.',
  doc: one([frame('outer', { display: 'flex', padding: 16 }, { background: '#f4f4f5', borderRadius: 12 }, [frame('inner', { display: 'flex', padding: 12 }, { background: W, borderRadius: 4 }, [txt('t', 'Nested')])])]) });

add({ name: 'monotonous-spacing/positive', rules: ['monotonous-spacing'], polarity: 'positive', description: 'Four gap/padding values, all 16: dominant share 100% > 60% and ≤ 3 distinct values. Reported on the lint target.',
  doc: one([frame('f1', { display: 'flex', gap: 16, padding: 16 }, {}), frame('f2', { display: 'flex', gap: 16, padding: 16 }, {})]) });
add({ name: 'monotonous-spacing/negative', rules: ['monotonous-spacing'], polarity: 'negative', description: 'Values 8/16/24/32: no value over 60%.',
  doc: one([frame('f1', { display: 'flex', gap: 8, padding: 16 }, {}), frame('f2', { display: 'flex', gap: 24, padding: 32 }, {})]) });

add({ name: 'tiny-text/positive', rules: ['tiny-text'], polarity: 'positive', description: 'Own font size 11px < 12.',
  doc: one([txt('t', 'Terms apply', { fontSize: 11 })]) });
add({ name: 'tiny-text/negative', rules: ['tiny-text'], polarity: 'negative', description: 'A: 12px. B: 0.6rem — not a px value, so the model check skips it (QUIRK: rem/em sizes are never judged by the model rules).',
  doc: one([txt('a', 'Terms apply', { fontSize: 12 }), txt('b', 'Footnote', { fontSize: '0.6rem', fontFamily: 'Canela' })]) });

add({ name: 'oversized-h1/positive', rules: ['oversized-h1'], polarity: 'positive', description: 'Any text node with own font size ≥ 72px.',
  doc: one([txt('t', 'Big', { fontSize: 72 })]) });
add({ name: 'oversized-h1/negative', rules: ['oversized-h1'], polarity: 'negative', description: '71px.',
  doc: one([txt('t', 'Big', { fontSize: 71 })]) });

add({ name: 'extreme-negative-tracking/positive', rules: ['extreme-negative-tracking'], polarity: 'positive', description: "A: '-0.05em'. B: -3 (px) at 48px = -0.0625em.",
  doc: one([txt('a', 'Tight', { fontSize: 48, letterSpacing: '-0.05em' }), txt('b', 'Tighter', { fontSize: 48, letterSpacing: -3, fontFamily: 'Canela' })]) });
add({ name: 'extreme-negative-tracking/negative', rules: ['extreme-negative-tracking'], polarity: 'negative', description: '-0.02em.',
  doc: one([txt('t', 'Tight', { fontSize: 48, letterSpacing: '-0.02em' })]) });

add({ name: 'wide-tracking/positive', rules: ['wide-tracking'], polarity: 'positive', description: "Body text (< 24px) tracked ≥ 0.05em: A '0.08em' at 14px; B 2px at 14px = 0.143em.",
  doc: one([txt('a', 'Spaced out', { fontSize: 14, letterSpacing: '0.08em' }), txt('b', 'Spaced out', { fontSize: 14, letterSpacing: 2, fontFamily: 'Canela' })]) });
add({ name: 'wide-tracking/negative', rules: ['wide-tracking'], polarity: 'negative', description: 'Large text (32px ≥ 24) is exempt; 0.02em is under the threshold.',
  doc: one([txt('a', 'Spaced out', { fontSize: 32, letterSpacing: '0.08em' }), txt('b', 'Spaced', { fontSize: 14, letterSpacing: '0.02em', fontFamily: 'Canela' })]) });

add({ name: 'all-caps-body/positive', rules: ['all-caps-body'], polarity: 'positive', description: 'Own textTransform uppercase, text longer than 40 characters, body size.',
  doc: one([txt('t', 'This is a long paragraph set entirely in capitals', { fontSize: 14, textTransform: 'uppercase' })]) });
add({ name: 'all-caps-body/negative', rules: ['all-caps-body'], polarity: 'negative', description: 'A: 30 characters. B: typed in capitals without textTransform (the model rule reads the transform only).',
  doc: one([txt('a', 'Short uppercase label here ok', { fontSize: 14, textTransform: 'uppercase' }), txt('b', 'THIS IS A LONG PARAGRAPH TYPED IN CAPITALS BY HAND', { fontSize: 14, fontFamily: 'Canela' })]) });

add({ name: 'justified-text/positive', rules: ['justified-text'], polarity: 'positive', description: 'textAlign justify.',
  doc: one([txt('t', 'Justified body copy.', { textAlign: 'justify' })]) });
add({ name: 'justified-text/negative', rules: ['justified-text'], polarity: 'negative', description: 'textAlign left.',
  doc: one([txt('t', 'Left body copy.', { textAlign: 'left' })]) });

add({ name: 'low-contrast/positive', rules: ['low-contrast'], polarity: 'positive', description: 'Body text #8a8a8a on the white artboard (ratio < 4.5); background resolved from the nearest ancestor with a parseable solid colour.',
  doc: one([txt('t', 'Muted body copy', { color: '#8a8a8a' })]) });
add({ name: 'low-contrast/negative', rules: ['low-contrast'], polarity: 'negative', description: 'A: the same grey at 24px (large text, threshold 3:1). B: #111111 body text. C (QUIRK): text with no own colour inherits a light grey — the model check reads only the node\'s own colour, so nothing is judged.',
  doc: one([
    txt('a', 'Muted heading', { fontSize: 24, color: '#8a8a8a' }),
    txt('b', 'Ink body copy', { fontFamily: 'Canela' }),
    frame('wrap', { display: 'flex' }, { color: '#dddddd' }, [{ id: 'c', type: 'text', name: 'c', text: 'Inherited grey', style: { fontSize: 16, fontFamily: 'Plex' } }]),
  ]) });
add({ name: 'low-contrast/near-black-on-colour', rules: ['low-contrast', 'gray-on-color'], polarity: 'positive', description: 'Near-black text (luminance < 0.03) is not "gray", so gray-on-color does not fire; it falls through to low-contrast (#1a1a1a on #2563eb ≈ 3.4:1).',
  doc: one([frame('band', { display: 'flex', padding: 24 }, { background: '#2563eb' }, [txt('t', 'Dark on blue', { color: '#1a1a1a' })])]) });

add({ name: 'gray-on-color/positive', rules: ['gray-on-color'], polarity: 'positive', description: 'Mid grey (#888888) on a chromatic background (#2563eb). Reported instead of low-contrast.',
  doc: one([frame('band', { display: 'flex', padding: 24 }, { background: '#2563eb' }, [txt('t', 'Grey on blue', { color: '#888888' })])]) });
add({ name: 'gray-on-color/negative', rules: ['gray-on-color', 'low-contrast'], polarity: 'negative', description: 'White (near-white, luminance > 0.7) on the same blue.',
  doc: one([frame('band', { display: 'flex', padding: 24 }, { background: '#2563eb' }, [txt('t', 'White on blue', { color: W })])]) });

add({ name: 'marketing-buzzword/positive', rules: ['marketing-buzzword'], polarity: 'positive', description: 'Case-insensitive substring match; the FIRST phrase in list order is reported.',
  doc: one([txt('t', 'Seamlessly integrate and Supercharge your workflow')]) });
add({ name: 'marketing-buzzword/negative', rules: ['marketing-buzzword'], polarity: 'negative', description: 'Plain copy.',
  doc: one([txt('t', 'Invoices your accountant will like.')]) });

add({ name: 'em-dash-overuse/positive', rules: ['em-dash-overuse'], polarity: 'positive', description: 'Five dashes across the subtree text: four "—" and one "--" followed by a non-space.',
  doc: one([txt('a', 'One — two — three', { fontSize: 32 }), txt('b', 'four — five — six --seven', { fontFamily: 'Canela' })]) });
add({ name: 'em-dash-overuse/negative', rules: ['em-dash-overuse'], polarity: 'negative', description: 'Four em-dashes; "-- " followed by a space does not count.',
  doc: one([txt('a', 'One — two — three — four — five -- six')]) });

add({ name: 'numbered-section-markers/positive', rules: ['numbered-section-markers'], polarity: 'positive', description: 'Distinct markers 01, 02, 03 (two consecutive steps).',
  doc: one([txt('a', '01 Plan  02 Build  03 Ship')]) });
add({ name: 'numbered-section-markers/negative', rules: ['numbered-section-markers'], polarity: 'negative', description: '01, 05, 09: three markers but no consecutive pair. 13 is outside 01–12.',
  doc: one([txt('a', '01 Plan  05 Build  09 Ship  13 Review')]) });

// ── motion rules ──────────────────────────────────────────────────────────
add({ name: 'bounce-easing/positive', rules: ['bounce-easing'], polarity: 'positive', description: 'A: overshooting cubic-bezier in a transition shorthand (expanded to longhands). B: a hover state with a non-monotonic linear(). C: a shorthand that does not parse (unknown easing word) is kept as written and matched on "bounce".',
  doc: one([
    box('a', { transition: 'transform 300ms cubic-bezier(0.34, 1.56, 0.64, 1)', width: 100, height: 40 }),
    box('b', { width: 100, height: 40 }, { states: { hover: { transitionTimingFunction: 'linear(0, 1.2, 1)' } } }),
    box('c', { transition: 'transform 300ms ease-out-bounce', width: 100, height: 40 }),
  ]) });
add({ name: 'bounce-easing/negative', rules: ['bounce-easing'], polarity: 'negative', description: 'A standard cubic-bezier within [0, 1] and ease-out.',
  doc: one([box('a', { transition: 'transform 200ms cubic-bezier(0.2, 0, 0, 1)', width: 100, height: 40 }), box('b', { transitionProperty: 'opacity', transitionTimingFunction: 'ease-out', width: 100, height: 40 })]) });
add({ name: 'layout-transition/positive', rules: ['layout-transition'], polarity: 'positive', description: 'A: transition on width. B: a transition with no property expands to transition-property: all. C: padding-top inside the reduced-motion media block.',
  doc: one([
    box('a', { transition: 'width 200ms ease', width: 100, height: 40 }),
    box('b', { transition: '200ms ease-out', width: 100, height: 40 }),
    box('c', { width: 100, height: 40 }, { media: { '(prefers-reduced-motion: reduce)': { style: { transitionProperty: 'padding-top' } } } }),
  ]) });
add({ name: 'layout-transition/negative', rules: ['layout-transition'], polarity: 'negative', description: 'opacity and transform only.',
  doc: one([box('a', { transition: 'opacity 150ms ease-out, transform 150ms ease-out', width: 100, height: 40 })]) });
add({ name: 'image-hover-transform/positive', rules: ['image-hover-transform'], polarity: 'positive', description: 'An image whose hover state scales it and whose pressed (active) state rotates it.',
  doc: one([{ id: 'img', type: 'image', name: 'Photo', src: 'data:image/png;base64,AA==', style: { width: 200, height: 120 }, states: { hover: { transform: 'scale(1.05)' }, active: { rotate: '2deg' } } }]) });
add({ name: 'image-hover-transform/negative', rules: ['image-hover-transform'], polarity: 'negative', description: 'An image whose hover transform is none; a BOX that scales on hover (only images count).',
  doc: one([
    { id: 'img', type: 'image', name: 'Photo', src: 'data:image/png;base64,AA==', style: { width: 200, height: 120 }, states: { hover: { transform: 'none' } } },
    box('b', { width: 100, height: 40 }, { states: { hover: { scale: '1.05' } } }),
  ]) });

// ── document hygiene (informational in every profile) ────────────────────
add({ name: 'size-duplicated/positive', rules: ['size-duplicated'], polarity: 'positive', description: 'width in both layout and style: listed under informational, never counted.',
  doc: one([{ id: 'b', type: 'box', name: 'b', layout: { width: 120 }, style: { width: 120, height: 40 } }]) });
add({ name: 'size-duplicated/negative', rules: ['size-duplicated'], polarity: 'negative', description: 'width only in style, height only in layout.',
  doc: one([{ id: 'b', type: 'box', name: 'b', layout: { height: 40 }, style: { width: 120 } }]) });
add({ name: 'style-shorthand-conflict/positive', rules: ['style-shorthand-conflict'], polarity: 'positive', description: 'padding beside paddingTop, and border beside borderTopWidth (one finding per node, every pair in the detail).',
  doc: one([box('b', { padding: 20, paddingTop: 4, border: '1px solid #dddddd', borderTopWidth: 2, width: 120, height: 40 })]) });
add({ name: 'style-shorthand-conflict/negative', rules: ['style-shorthand-conflict'], polarity: 'negative', description: 'Only longhands of one group; a shorthand of a different group.',
  doc: one([box('b', { paddingTop: 4, paddingBottom: 8, borderRadius: 8, width: 120, height: 40 })]) });

// ── render-dependent rules (measured rects supplied) ─────────────────────
const para = 'Every line of this paragraph runs across the whole artboard so it is long enough to read as body copy, and then it keeps going well past any comfortable measure for a reader.';
add({ name: 'line-length/positive', rules: ['line-length'], polarity: 'positive', needsRects: true, description: `A ${para.length}-character paragraph measured one line tall (19.2px line = 16 × 1.2).`,
  doc: one([txt('p', para)]), rects: { ...BOARD_RECT, p: RECT(48, 48, 700, 19) } });
add({ name: 'line-length/negative', rules: ['line-length'], polarity: 'negative', needsRects: true, description: 'The same paragraph measured 96px tall: 5 lines, ~35 characters per line.',
  doc: one([txt('p', para)]), rects: { ...BOARD_RECT, p: RECT(48, 48, 600, 96) } });
add({ name: 'line-length/no-rect', rules: ['line-length'], polarity: 'negative', needsRects: true, description: 'No measured rect for the paragraph: render-dependent rules skip the node, never guess.',
  doc: one([txt('p', para)]), rects: { ...BOARD_RECT } });

add({ name: 'body-text-viewport-edge/positive', rules: ['body-text-viewport-edge'], polarity: 'positive', needsRects: true, description: 'A ≥ 40-character body paragraph 8px from the artboard\'s left edge.',
  doc: one([txt('p', 'A paragraph that sits flush against the left edge of the page.')]), rects: { ...BOARD_RECT, p: RECT(8, 48, 500, 96) } });
add({ name: 'body-text-viewport-edge/negative', rules: ['body-text-viewport-edge'], polarity: 'negative', needsRects: true, description: '24px from both edges; and a short (< 40 char) label at the edge.',
  doc: one([txt('p', 'A paragraph that sits a comfortable distance from the edge.'), txt('l', 'Label', { fontFamily: 'Canela', fontSize: 32 })]),
  rects: { ...BOARD_RECT, p: RECT(24, 48, 500, 96), l: RECT(0, 200, 60, 20) } });

add({ name: 'text-overflow/positive', rules: ['text-overflow'], polarity: 'positive', needsRects: true, description: 'An in-flow child whose rect spills its parent\'s right edge by 20px (any node type, not only text).',
  doc: one([frame('row', { display: 'flex' }, {}, [box('wide', { width: 420, height: 40 })])]), rects: { ...BOARD_RECT, row: RECT(48, 48, 400, 40), wide: RECT(48, 48, 420, 40) } });
add({ name: 'text-overflow/negative', rules: ['text-overflow'], polarity: 'negative', needsRects: true, description: 'An absolutely positioned child spilling (out of flow: exempt), and a 1px spill (threshold is > 1).',
  doc: one([frame('row', { display: 'flex' }, { position: 'relative' }, [box('abs', { position: 'absolute', width: 420, height: 40 }), box('edge', { width: 401, height: 40 })])]),
  rects: { ...BOARD_RECT, row: RECT(48, 48, 400, 40), abs: RECT(48, 48, 420, 40), edge: RECT(48, 48, 401, 40) } });

add({ name: 'clipped-overflow-container/positive', rules: ['clipped-overflow-container'], polarity: 'positive', needsRects: true, description: 'A text 25% visible inside an overflow:hidden frame; the clipping frame is named and related.',
  doc: one([frame('clip', { display: 'block' }, { position: 'relative', overflow: 'hidden', width: 200, height: 100 }, [txt('t', 'Cut off', { position: 'absolute' })])]),
  rects: { ...BOARD_RECT, clip: RECT(100, 100, 200, 100), t: RECT(250, 100, 200, 40) } });
add({ name: 'clipped-overflow-container/negative', rules: ['clipped-overflow-container'], polarity: 'negative', needsRects: true, description: 'A box shape 25% visible (shapes are never flagged) and a text 80% visible.',
  doc: one([frame('clip', { display: 'block' }, { position: 'relative', overflow: 'hidden', width: 200, height: 100 }, [box('shape', { position: 'absolute', background: '#f97316' }), txt('t', 'Mostly here', { position: 'absolute' })])]),
  rects: { ...BOARD_RECT, clip: RECT(100, 100, 200, 100), shape: RECT(250, 100, 200, 40), t: RECT(140, 150, 200, 40) } });

add({ name: 'hero-eyebrow-chip/positive', rules: ['hero-eyebrow-chip'], polarity: 'positive', needsRects: true, description: 'A 12px uppercase label ending 16px above a 56px headline it overlaps horizontally.',
  doc: one([txt('eyebrow', 'New release', { fontSize: 12, textTransform: 'uppercase', fontFamily: 'Plex' }), txt('h', 'Ship faster', { fontSize: 56, fontFamily: 'Canela' })]),
  rects: { ...BOARD_RECT, eyebrow: RECT(48, 40, 120, 16), h: RECT(48, 72, 500, 68) } });
add({ name: 'hero-eyebrow-chip/negative', rules: ['hero-eyebrow-chip'], polarity: 'negative', needsRects: true, description: 'The same label 64px above the headline (gap > 40).',
  doc: one([txt('eyebrow', 'New release', { fontSize: 12, textTransform: 'uppercase', fontFamily: 'Plex' }), txt('h', 'Ship faster', { fontSize: 56, fontFamily: 'Canela' })]),
  rects: { ...BOARD_RECT, eyebrow: RECT(48, 40, 120, 16), h: RECT(48, 120, 500, 68) } });
add({ name: 'hero-eyebrow-chip/pill', rules: ['hero-eyebrow-chip'], polarity: 'positive', needsRects: true, description: 'A pill: the label is the only child of a filled, fully rounded holder; the holder\'s rect is the one measured against the headline.',
  doc: one([frame('chip', { display: 'flex', padding: 8 }, { background: '#eef2ff', borderRadius: 999 }, [txt('label', 'Now in beta', { fontSize: 13, fontFamily: 'Plex' })]), txt('h', 'Ship faster', { fontSize: 64, fontFamily: 'Canela' })]),
  rects: { ...BOARD_RECT, chip: RECT(48, 40, 140, 32), label: RECT(56, 48, 124, 16), h: RECT(48, 96, 500, 76) } });

add({ name: 'text-occluded/covered', rules: ['text-occluded'], polarity: 'positive', needsRects: true, description: 'A positioned (absolute) later sibling with an opaque fill covers 50% of an in-flow text: it paints above.',
  doc: one([txt('title', 'Headline', { fontSize: 48, fontFamily: 'Canela' }), box('shape', { position: 'absolute', background: '#2563eb' })]),
  rects: { ...BOARD_RECT, title: RECT(48, 48, 400, 80), shape: RECT(48, 48, 200, 80) } });
add({ name: 'text-occluded/camouflage', rules: ['text-occluded'], polarity: 'positive', needsRects: true, description: 'An earlier in-flow sibling (paints below) filled in the text\'s own colour covers 20% of it: < 1.5:1 contrast, ≥ 10% share.',
  doc: one([box('moth', { background: '#14211a' }), txt('word', 'WATCH', { fontSize: 48, color: '#14211a', fontFamily: 'Canela' })]),
  rects: { ...BOARD_RECT, moth: RECT(48, 48, 80, 80), word: RECT(48, 48, 400, 80) } });
add({ name: 'text-occluded/negative', rules: ['text-occluded'], polarity: 'negative', needsRects: true, description: 'An earlier in-flow sibling below the text in a contrasting fill (no camouflage), and a positioned sibling above covering only 12.5%.',
  doc: one([box('under', { background: '#f97316' }), txt('title', 'Headline', { fontSize: 48, fontFamily: 'Canela' }), box('dot', { position: 'absolute', background: '#2563eb' })]),
  rects: { ...BOARD_RECT, under: RECT(48, 48, 400, 80), title: RECT(48, 48, 400, 80), dot: RECT(48, 48, 50, 80) } });

const twoRoots = (rects) => ({
  doc: mk({ roots: [
    { id: 'a', type: 'frame', name: 'Home', canvas: { x: 0, y: 0 }, layout: { display: 'flex' }, style: { background: W, width: 400, height: 300 } },
    { id: 'b', type: 'frame', name: 'Pricing', canvas: { x: 300, y: 200 }, layout: { display: 'flex' }, style: { background: W, width: 400, height: 300 } },
  ] }),
  rects,
});
add({ name: 'root-overlap/positive', rules: ['root-overlap'], polarity: 'positive', needsRects: true, description: 'Two root artboards whose rects intersect by 100×100. Whole-document runs only; attributed to the earlier root, naming the other.',
  ...twoRoots({ a: RECT(0, 0, 400, 300), b: RECT(300, 200, 400, 300) }) });
add({ name: 'root-overlap/negative-touching', rules: ['root-overlap'], polarity: 'negative', needsRects: true, description: 'Touching edges do not overlap.',
  ...twoRoots({ a: RECT(0, 0, 400, 300), b: RECT(400, 0, 400, 300) }) });
add({ name: 'root-overlap/subtree-run', rules: ['root-overlap'], polarity: 'negative', needsRects: true, description: 'A run with id (a subtree) never reports root-overlap.',
  ...twoRoots({ a: RECT(0, 0, 400, 300), b: RECT(300, 200, 400, 300) }), steps: [{ tool: 'lint_design', args: { id: 'a' } }] });

// ── unported catalog rules never fire ────────────────────────────────────
add({ name: 'unported-rules/never-reported', rules: ['italic-serif-display', 'tight-leading', 'ai-color-palette', 'cream-palette', 'dark-glow', 'broken-image', 'skipped-heading', 'icon-tile-stack', 'aphoristic-cadence', 'repeated-section-kickers', 'theater-slop-phrase'], polarity: 'negative',
  description: 'Shapes that upstream Impeccable would flag (italic serif display, line-height 1.0, purple gradient, cream ground, dark glow, empty image src) — the 11 unported rules never appear in any output.',
  doc: mk({ roots: [{ id: 'board', type: 'frame', name: 'Board', canvas: { x: 0, y: 0 }, layout: { display: 'flex', flexDirection: 'column' }, style: { background: '#0b0b10', width: 800, height: 600 }, children: [
    txt('h', 'A quiet revolution', { fontSize: 64, fontStyle: 'italic', fontFamily: 'Georgia', lineHeight: 1, color: '#f5f5f5' }),
    box('glow', { background: 'linear-gradient(135deg, #7c3aed, #06b6d4)', boxShadow: '0px 0px 40px rgba(124, 58, 237, 0.8)', width: 200, height: 60 }),
    { id: 'img', type: 'image', name: 'Placeholder', src: '', style: { width: 200, height: 120 } },
  ] }] }) });

// ── profiles ─────────────────────────────────────────────────────────────
const poster = (lint) => mk({ lint, roots: [
  { id: 'poster', type: 'frame', name: 'Poster', canvas: { x: 0, y: 0 }, layout: { display: 'flex', flexDirection: 'column', padding: 48 }, style: { background: '#d7f25a', width: 800, height: 1100, overflow: 'hidden', position: 'relative' }, children: [
    txt('moth', 'MOTH', { fontSize: 300, color: '#14211a', fontFamily: 'Anton' }),
    box('wing', { position: 'absolute', background: '#14211a' }),
    txt('faint', 'free entry', { fontSize: 14, color: '#e8f79a', fontFamily: 'Space Mono' }),
    box('rule', { borderBottom: '6px solid #e11d48', width: 700 }),
    txt('bleed', 'NIGHT', { fontSize: 120, color: '#14211a', fontFamily: 'Anton', position: 'absolute' }),
  ] },
] });
const posterRects = { poster: RECT(0, 0, 800, 1100), moth: RECT(48, 48, 700, 360), wing: RECT(48, 48, 700, 200), faint: RECT(48, 420, 100, 17), rule: RECT(48, 460, 700, 6), bleed: RECT(700, 900, 400, 144) };
add({ name: 'profiles/expressive-stored', rules: ['oversized-h1', 'side-tab', 'clipped-overflow-container', 'text-occluded', 'low-contrast'], polarity: 'mixed', needsRects: true,
  description: 'A poster stored as expressive: oversized-h1, side-tab and clipped-overflow-container are relaxed (relaxedByProfile), text-occluded is informational, low-contrast still counts (never relaxed).',
  doc: poster({ profiles: { poster: 'expressive' }, ignores: [] }), rects: posterRects });
add({ name: 'profiles/override-ui', rules: ['oversized-h1', 'side-tab', 'clipped-overflow-container', 'text-occluded'], polarity: 'positive', needsRects: true,
  description: 'The same expressive poster linted with profile: "ui" (an override for this run, nothing written): every finding is active.',
  doc: poster({ profiles: { poster: 'expressive' }, ignores: [] }), rects: posterRects, steps: [{ tool: 'lint_design', args: { profile: 'ui' } }] });
add({ name: 'profiles/override-expressive-subtree', rules: ['oversized-h1', 'low-contrast'], polarity: 'mixed', needsRects: true,
  description: 'A ui-profile poster linted by id with profile: "expressive".',
  doc: poster(undefined), rects: posterRects, steps: [{ tool: 'lint_design', args: { id: 'poster', profile: 'expressive' } }] });

add({ name: 'set_lint_profile/flow', rules: [], polarity: 'mixed', needsRects: true,
  description: 'set_lint_profile writes the profile (ui stored as absent), returns per-root counts computed WITHOUT measured rects (QUIRK: its counts can differ from lint_design\'s), and refuses non-roots and unknown ids.',
  doc: poster(undefined), rects: posterRects, steps: [
    { tool: 'set_lint_profile', args: { ids: ['poster', 'poster'], profile: 'expressive' } },
    { tool: 'lint_design', args: {} },
    { tool: 'set_lint_profile', args: { ids: ['poster'], profile: 'expressive' } },
    { tool: 'set_lint_profile', args: { ids: ['poster'], profile: 'ui' } },
    { tool: 'set_lint_profile', args: { ids: ['moth'], profile: 'expressive' } },
    { tool: 'set_lint_profile', args: { ids: ['nope'], profile: 'ui' } },
  ] });

// ── ignores ──────────────────────────────────────────────────────────────
const ignoreDoc = () => mk({ roots: [
  board([
    frame('hero', { display: 'flex', flexDirection: 'column' }, {}, [txt('h1', 'Big headline', { fontSize: 96, fontFamily: 'Inter' }), txt('sub', 'Muted subhead', { color: '#9a9a9a', fontFamily: 'Canela' })]),
    txt('footer', 'Footer in Inter', { fontFamily: 'Inter', fontSize: 32 }),
  ]),
] });
add({ name: 'ignore_lint/flow', rules: ['overused-font', 'oversized-h1', 'low-contrast'], polarity: 'mixed',
  description: 'Node-scoped ignore on an ancestor covers descendants only; a re-reason of the same (ruleId, nodeId) replaces the reason; a document-wide ignore covers every node; the reason is trimmed; ignoring contrast is NOT refused by the tool (its description says never to); remove deletes; removing an absent ignore is a no-op.',
  doc: ignoreDoc(), steps: [
    { tool: 'lint_design', args: {} },
    { tool: 'ignore_lint', args: { ruleId: 'overused-font', nodeId: 'hero', reason: '  brand face inside the hero  ' } },
    { tool: 'lint_design', args: {} },
    { tool: 'ignore_lint', args: { ruleId: 'overused-font', nodeId: 'hero', reason: 'licensed brand face' } },
    { tool: 'ignore_lint', args: { ruleId: 'oversized-h1', reason: 'campaign page' } },
    { tool: 'ignore_lint', args: { ruleId: 'low-contrast', nodeId: 'sub', reason: 'decorative' } },
    { tool: 'lint_design', args: {} },
    { tool: 'ignore_lint', args: { ruleId: 'overused-font', nodeId: 'hero', remove: true } },
    { tool: 'ignore_lint', args: { ruleId: 'tight-leading', remove: true } },
    { tool: 'lint_design', args: {} },
  ] });
add({ name: 'ignore_lint/errors', rules: [], polarity: 'negative',
  description: 'Refusals: unknown rule (lists every known rule id), missing or blank reason, unknown node. Nothing is written.',
  doc: ignoreDoc(), steps: [
    { tool: 'ignore_lint', args: { ruleId: 'no-such-rule', reason: 'x' } },
    { tool: 'ignore_lint', args: { ruleId: 'overused-font' } },
    { tool: 'ignore_lint', args: { ruleId: 'overused-font', reason: '   ' } },
    { tool: 'ignore_lint', args: { ruleId: 'overused-font', nodeId: 'ghost', reason: 'x' } },
    { tool: 'ignore_lint', args: { ruleId: 'italic-serif-display', reason: 'known but unported rules are accepted' } },
  ] });
add({ name: 'ignore_lint/related-node', rules: ['root-overlap'], polarity: 'mixed', needsRects: true,
  description: 'An ignore naming the OTHER root of an overlapping pair matches through relatedNodeIds.',
  ...twoRoots({ a: RECT(0, 0, 400, 300), b: RECT(300, 200, 400, 300) }), steps: [
    { tool: 'ignore_lint', args: { ruleId: 'root-overlap', nodeId: 'b', reason: 'staging overlap while comparing' } },
    { tool: 'lint_design', args: {} },
  ] });
add({ name: 'ignore_lint/stored-in-document', rules: ['overused-font'], polarity: 'mixed',
  description: 'Ignores and profiles read from ScrimDoc.lint as loaded (a second session inherits them).',
  doc: mk({ lint: { profiles: {}, ignores: [{ ruleId: 'overused-font', reason: 'brand face' }] }, roots: [board([txt('t', 'Hello', { fontFamily: 'Roboto' })])] }) });

// ── scope ────────────────────────────────────────────────────────────────
add({ name: 'scope/subtree-target', rules: ['single-font', 'nested-cards'], polarity: 'mixed',
  description: 'lint_design with a child id: only that subtree is linted, aggregates are attributed to the target, rootId and profile come from the artboard, and nested-cards still sees ancestors outside the subtree.',
  doc: one([frame('outer', { display: 'flex', padding: 16 }, { background: '#f4f4f5', borderRadius: 12 }, [frame('inner', { display: 'flex', flexDirection: 'column', padding: 12 }, { background: W, borderRadius: 8 }, [txt('a', 'One', { fontSize: 32 }), txt('b', 'Two')])]), txt('out', 'Outside', { fontFamily: 'Inter' })]),
  steps: [{ tool: 'lint_design', args: { id: 'inner' } }] });
add({ name: 'scope/not-found', rules: [], polarity: 'negative', description: 'An unknown id is refused.',
  doc: one([]), steps: [{ tool: 'lint_design', args: { id: 'ghost' } }] });
add({ name: 'scope/two-roots-clean', rules: [], polarity: 'negative', description: 'A clean document: zero violations; `profiles` lists only roots that produced a finding (here none).',
  doc: mk({ roots: [board([txt('h', 'Pricing', { fontSize: 32, fontFamily: 'Canela' }), txt('p', 'Simple pricing for every team.')]), { id: 'b2', type: 'frame', name: 'Second', canvas: { x: 1000, y: 0 }, layout: { display: 'flex' }, style: { background: W, width: 400, height: 300 } }] }) });

// ── run ──────────────────────────────────────────────────────────────────
const out = [];
for (const c of cases) {
  const steps = c.steps ?? [{ tool: 'lint_design' }];
  const r = await run(c.doc, steps, c.rects);
  out.push({ name: c.name, rules: c.rules, polarity: c.polarity, needsRects: c.needsRects === true, description: c.description, doc: c.doc, rects: c.rects ?? null, steps: r.steps, docAfter: JSON.stringify(r.docAfter) === JSON.stringify(c.doc) ? null : { lint: r.docAfter.lint ?? null } });
}
const commit = execSync(`git -C ${process.env.SCRIM_REPO ?? new URL('../../../../', import.meta.url).pathname} rev-parse --short HEAD`).toString().trim();
writeFileSync(OUT, JSON.stringify({ schema: 'scrim-transfer/lint-cases@1', generatedFrom: `Scrim ${commit} (packages/*/dist), via the MCP tools in-process`, spec: '../specs/lint.md', cases: out }, null, 2) + '\n');
console.log(`${out.length} cases`);
