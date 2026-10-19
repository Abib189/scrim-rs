// Build docs/transfer/ui/tokens.json and contrast.json from resolved.json
// (headless-Chrome resolution of theme.css) and derive.ts (the accent rule).
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { deriveBoth, contrast as dContrast } from './derive.ts';

const OUT = '../../../docs/transfer/ui';
mkdirSync(OUT, { recursive: true });
const R = JSON.parse(readFileSync(new URL('./resolved.json', import.meta.url), 'utf8'));
const THEMES = ['dark', 'light'];

// ── colour maths ───────────────────────────────────────────────────────────
function parse(c) {
  c = c.trim();
  let m = /^#([0-9a-f]{6})([0-9a-f]{2})?$/i.exec(c);
  if (m) return { r: parseInt(m[1].slice(0, 2), 16), g: parseInt(m[1].slice(2, 4), 16), b: parseInt(m[1].slice(4, 6), 16), a: m[2] ? parseInt(m[2], 16) / 255 : 1 };
  m = /^rgba?\(\s*([\d.]+)[ ,]+([\d.]+)[ ,]+([\d.]+)(?:\s*[,/]\s*([\d.]+))?\s*\)$/.exec(c);
  if (m) return { r: +m[1], g: +m[2], b: +m[3], a: m[4] === undefined ? 1 : +m[4] };
  m = /^color\(srgb ([\d.]+) ([\d.]+) ([\d.]+)(?: \/ ([\d.]+))?\)$/.exec(c);
  if (m) return { r: +m[1] * 255, g: +m[2] * 255, b: +m[3] * 255, a: m[4] === undefined ? 1 : +m[4] };
  throw new Error(`unparsed colour ${c}`);
}
const h2 = (v) => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, '0');
function hex(c) {
  const a = Math.round(c.a * 1000) / 1000;
  return a >= 1 ? `#${h2(c.r)}${h2(c.g)}${h2(c.b)}` : `#${h2(c.r)}${h2(c.g)}${h2(c.b)}${h2(a * 255)}`;
}
const rgba = (c) => `rgba(${Math.round(c.r)}, ${Math.round(c.g)}, ${Math.round(c.b)}, ${Math.round(c.a * 1000) / 1000})`;
const over = (fg, bg) => ({ r: fg.r * fg.a + bg.r * (1 - fg.a), g: fg.g * fg.a + bg.g * (1 - fg.a), b: fg.b * fg.a + bg.b * (1 - fg.a), a: 1 });
const ch = (v) => { const s = v / 255; return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4; };
const lum = (c) => 0.2126 * ch(c.r) + 0.7152 * ch(c.g) + 0.0722 * ch(c.b);
const ratio = (a, b) => { const x = lum(a), y = lum(b); return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05); };
const withAlpha = (c, a) => ({ ...c, a: c.a * a });
const bright = (c, k) => ({ r: c.r * k, g: c.g * k, b: c.b * k, a: c.a });

// ── lookups ────────────────────────────────────────────────────────────────
const src = (theme, v) => R.source[theme][v] ?? R.source.dark[v] ?? R.source.base[v];
const col = (theme, v) => {
  const t = R[theme][v];
  if (!t || !t.color) throw new Error(`no colour ${v} (${theme})`);
  return parse(t.color);
};
const raw = (theme, v) => R[theme][v].computed;

let tokenCount = 0;
function ctok(theme, v, description, extra = {}) {
  tokenCount++;
  const c = col(theme, v);
  return { $value: hex(c), $description: description, $extensions: { scrim: { var: v, css: src(theme, v), ...(c.a < 1 ? { rgba: rgba(c) } : {}), ...extra } } };
}
/** a colour built from a token plus an alpha (overlay drawing) */
function atok(theme, v, alpha, description, css) {
  tokenCount++;
  const c = withAlpha(col(theme, v), alpha);
  return { $value: hex(c), $description: description, $extensions: { scrim: { css, from: v, alpha, rgba: rgba(c) } } };
}
function etok(theme, key, description) {
  tokenCount++;
  const c = parse(R.exprs[theme][key]);
  return { $value: hex(c), $description: description, $extensions: { scrim: { css: R.source.exprs[key], at: key.replace(/.*\((.*)\)/, '$1'), rgba: rgba(c) } } };
}
function lit(value, description, css, extra = {}) {
  tokenCount++;
  return { $value: value, $description: description, $extensions: { scrim: { css, ...extra } } };
}

// ── colour groups per theme ────────────────────────────────────────────────
function colors(theme) {
  const T = (v, d, x) => ctok(theme, v, d, x);
  const panel = col(theme, '--scrim-bg-1');
  const accentFill = col(theme, '--scrim-accent-fill');
  const dangerFill = col(theme, '--scrim-danger-fill');
  const disabledField = over(withAlpha(col(theme, '--scrim-bg-2'), 0.5), panel);
  const disabledFieldText = over(withAlpha(col(theme, '--scrim-text-off'), 0.5), disabledField);
  return {
    surface: {
      canvas: T('--scrim-bg-0', 'bg-0: the canvas field behind artboards; also the terminal host'),
      panel: T('--scrim-bg-1', 'bg-1: panels, top bar, shell, dialogs backdrop-side, Home'),
      field: T('--scrim-bg-2', 'bg-2: fields, buttons at rest, hovered rows, chips, the layers band'),
      raised: T('--scrim-bg-3', 'bg-3: pressed fields, hovered secondary buttons, the floating tool bar and its flyouts'),
      selected: T('--scrim-bg-4', 'bg-4: selected rows / segments / active icon buttons (neutral), keycaps, pressed secondary button, disabled primary'),
      float: T('--scrim-bg-float', 'menus, popovers, dialogs, the command palette, canvas hints and toasts'),
      floatHover: T('--scrim-float-hover', 'a highlighted row on a floating surface'),
      tooltip: T('--scrim-tooltip-bg', 'tooltips, the modifier pill, the canvas readout chips'),
      backdrop: T('--scrim-backdrop', 'behind a modal dialog or the command palette'),
      artboard: T('--scrim-artboard', 'default artboard paper (design world baseline, not chrome)'),
    },
    text: {
      primary: T('--scrim-text-1', 'text-1: names, values, active tab, menu rows'),
      secondary: T('--scrim-text-2', 'text-2: labels, section heads, icon buttons at rest, inactive segments'),
      tertiary: T('--scrim-text-3', 'text-3: meta, prefixes, units, shortcuts, placeholders, layer glyphs, carets'),
      disabled: T('--scrim-text-off', 'text-off: disabled text (WCAG 1.4.3 exempt); also code line numbers'),
      onTooltip: T('--scrim-tooltip-text', 'text on the tooltip surface'),
    },
    border: {
      hairline: T('--scrim-line-1', 'line-1: section and panel separators, indent guides, header rules'),
      strong: T('--scrim-line-2', 'line-2: floating-surface edge, field on hover / editing, menu separators, toolbar dividers'),
      swatchEdge: etok(theme, 'swatch-edge (theme.css:611)', 'inset edge of a colour swatch (overlay-rgb at 18%)'),
      swatchStackEdge: etok(theme, 'swatch-stack-edge (theme.css:626)', 'edge between stacked swatches (overlay-rgb at 20%)'),
      swatchButtonEdge: etok(theme, 'swatch-btn-edge (theme.css:2250)', 'inset edge of the Settings accent swatches (text-1 at 12%)'),
      hint: etok(theme, 'hint-border (theme.css:681)', 'edge of an inline hint box (text-2 at 22%)'),
      lintQuality: etok(theme, 'lint-quality-border (theme.css:891)', 'rule under a "quality" lint group (text-2 at 16%)'),
      overlayRgb: lit(raw(theme, '--scrim-overlay-rgb'), 'channel triple for hairline overlays on chrome: white in dark, black in light', src(theme, '--scrim-overlay-rgb'), { var: '--scrim-overlay-rgb', type: 'rgb-channels' }),
    },
    accent: {
      mark: T('--scrim-accent-mark', 'DERIVED. Selection outline, focus ring, selected-row edge, drop lines, snap guides, coloured text on a surface. 4.5:1 on text surfaces, 3:1 on canvas and selected row'),
      fill: T('--scrim-accent-fill', 'DERIVED. Primary button, checked checkbox, canvas measurement chips. 3:1 against the panel'),
      onFill: T('--scrim-accent-on-fill', 'DERIVED. Label on the accent fill: #ffffff or #111214, whichever reaches 4.5:1 (near-black for the orange default)'),
      tint: T('--scrim-accent-tint', 'DERIVED. Low-alpha wash: drop-inside row, text selection in the layer rename field (16% dark / 12% light of the MARK)'),
      resizeHandleActive: etok(theme, 'resize-handle-active (theme.css:425)', 'panel resize handle while hovered or dragged (accent mark at 50%)'),
    },
    agent: {
      mark: T('--scrim-agent-mark', 'DERIVED (same rule as the accent). The agent: canvas presence outline, dock status dot, agent-edited layer marker, code-tab tags'),
      fill: T('--scrim-agent-fill', 'DERIVED. Agent presence chip on the canvas'),
      onFill: T('--scrim-agent-on-fill', 'DERIVED. Text on the agent chip'),
      tint: T('--scrim-agent-tint', 'DERIVED. Agent chip background / border wash'),
    },
    feedback: {
      danger: T('--scrim-danger', 'destructive labels, inline errors, "closed" connection state'),
      dangerFill: T('--scrim-danger-fill', 'destructive confirm button'),
      onDanger: T('--scrim-on-danger', 'label on the danger fill'),
      dangerFillHover: (tokenCount++, { $value: hex(bright(dangerFill, 0.92)), $description: 'danger button hover: the fill under filter: brightness(0.92)', $extensions: { scrim: { css: 'background: var(--scrim-danger-fill); filter: brightness(0.92)', measured: R.filters[theme].dangerHover92 } } }),
      warning: T('--scrim-warning', 'lint findings only: count, dot, badge, the lint tool icon'),
      warningTint: T('--scrim-warning-tint', 'lint card / group wash'),
      onWarning: T('--scrim-on-warning', 'text on a warning badge or a failing contrast chip'),
      warningBorderChip: etok(theme, 'chip-slop-border (theme.css:446)', 'border of a lint chip (warning at 30%)'),
      warningBorderCard: etok(theme, 'lint-card-border (theme.css:697)', 'border of a lint card (warning at 25%)'),
      warningBorderGroup: etok(theme, 'lint-group-border (theme.css:879)', 'rule under a lint group head (warning at 18%)'),
    },
    state: {
      rowHover: T('--scrim-bg-2', 'row hover (layers, components, stream rows): bg-2', { alias: 'surface.field' }),
      rowPressed: T('--scrim-bg-3', 'row pressed: bg-3', { alias: 'surface.raised' }),
      rowSelected: T('--scrim-bg-4', 'selected row background: bg-4 (neutral), plus a 2 px inset accent edge on the LEFT for layer rows', { alias: 'surface.selected' }),
      rowSelectedEdge: T('--scrim-accent-mark', 'selected layer row: 2 px inset left edge', { alias: 'accent.mark', width: '2px' }),
      buttonHover: T('--scrim-bg-3', 'secondary button hover', { alias: 'surface.raised' }),
      buttonPressed: T('--scrim-bg-4', 'secondary button pressed', { alias: 'surface.selected' }),
      iconHover: T('--scrim-bg-2', 'icon button / tool hover background (text goes text-2 to text-1)', { alias: 'surface.field' }),
      iconPressed: T('--scrim-bg-3', 'icon button pressed', { alias: 'surface.raised' }),
      iconActive: T('--scrim-bg-4', 'icon button / tool / segment / chip toggle selected (aria-pressed)', { alias: 'surface.selected' }),
      fieldHoverBorder: T('--scrim-line-2', 'field 1 px border on hover and while editing (transparent at rest)', { alias: 'border.strong' }),
      floatRowHover: T('--scrim-float-hover', 'menu / palette / font row highlight', { alias: 'surface.floatHover' }),
      focusRing: T('--scrim-accent-mark', 'keyboard focus: 2 px outline, 1 px offset, never removed', { alias: 'accent.mark', width: '2px', offset: '1px' }),
      focusRingOnPrimary: T('--scrim-text-1', 'focus ring drawn on the accent fill (shows on any accent)', { alias: 'text.primary' }),
      primaryHover: (tokenCount++, { $value: hex(bright(accentFill, 0.92)), $description: 'primary button hover: accent fill under filter: brightness(0.92), computed per channel in sRGB', $extensions: { scrim: { css: 'background: var(--scrim-accent-fill); filter: brightness(0.92)', measuredScreenshot: R.filters[theme].accentHover92 } } }),
      primaryPressed: (tokenCount++, { $value: hex(bright(accentFill, 0.84)), $description: 'primary button pressed: brightness(0.84)', $extensions: { scrim: { css: 'background: var(--scrim-accent-fill); filter: brightness(0.84)', measuredScreenshot: R.filters[theme].accentPressed84 } } }),
      primaryDisabledBg: T('--scrim-bg-4', 'disabled primary button background', { alias: 'surface.selected' }),
      disabledText: T('--scrim-text-off', 'disabled text everywhere (buttons, menu rows, palette rows, arrange buttons)', { alias: 'text.disabled' }),
      disabledFieldBg: (tokenCount++, { $value: hex(disabledField), $description: 'a disabled field: bg-2 at opacity 0.5, composited on the panel', $extensions: { scrim: { css: '.scrim-field[data-disabled] { opacity: 0.5 } over var(--scrim-bg-1)' } } }),
      disabledFieldText: (tokenCount++, { $value: hex(disabledFieldText), $description: 'text in a disabled field: text-off at opacity 0.5 over the composited field', $extensions: { scrim: { css: 'color: var(--scrim-text-off); opacity: 0.5' } } }),
      mixed: T('--scrim-text-3', 'a "Mixed" value: the word in text-3, the field otherwise at default', { alias: 'text.tertiary' }),
    },
    canvasChrome: {
      selection: T('--scrim-accent-mark', 'selection outline, 1.5 px; multi-select union box; draft outline', { alias: 'accent.mark', lineWidth: 1.5 }),
      hover: atok(theme, '--scrim-accent-mark', 0.45, 'hover outline: 1 px at globalAlpha 0.45', 'accent mark, globalAlpha 0.45'),
      handleFill: T('--scrim-handle-fill', 'resize handle: 7 px square, white fill, 1.5 px accent stroke', { size: 7 }),
      handleStroke: T('--scrim-accent-mark', 'resize handle stroke', { alias: 'accent.mark', lineWidth: 1.5 }),
      marqueeFill: atok(theme, '--scrim-accent-mark', 0.08, 'marquee fill (alpha 0.08) with a 1.5 px accent stroke', 'accent mark, globalAlpha 0.08'),
      snapGuide: atok(theme, '--scrim-accent-mark', 0.75, 'snap guide: 1 px dashed, 4 px dash / 4 px gap, alpha 0.75', 'accent mark, globalAlpha 0.75'),
      spacingTick: T('--scrim-accent-mark', 'spacing bar with 6 px end caps, 1 px', { alias: 'accent.mark' }),
      chipFill: T('--scrim-accent-fill', 'distance chip block (14 px tall, 11 px Plex Sans, 5 px side padding)', { alias: 'accent.fill' }),
      chipText: T('--scrim-accent-on-fill', 'distance chip label', { alias: 'accent.onFill' }),
      dropContainer: atok(theme, '--scrim-accent-mark', 0.6, 'receiving container outline while dragging (1 px, alpha 0.6)', 'accent mark, globalAlpha 0.6'),
      dropGhostFill: atok(theme, '--scrim-accent-mark', 0.1, 'drag ghost fill (alpha 0.1; edge at 0.9)', 'accent mark, globalAlpha 0.1'),
      dropLine: T('--scrim-accent-mark', 'insertion line, 2 px', { alias: 'accent.mark', width: 2 }),
      agentOutline: T('--scrim-agent-mark', 'agent presence: 1 px outline on the node it is writing, fading', { alias: 'agent.mark' }),
      agentChipFill: T('--scrim-agent-fill', 'agent chip under the node (15 px tall)', { alias: 'agent.fill' }),
      agentChipText: T('--scrim-agent-on-fill', 'agent chip text', { alias: 'agent.onFill' }),
      measure: T('--scrim-measure', 'THE exception to one accent: distance lines, bands (alpha 0.16) and chips while measuring (10 px Plex Mono)'),
      onMeasure: T('--scrim-on-measure', 'text on a measurement chip'),
      readout: T('--scrim-tooltip-bg', 'position / size readout chip', { alias: 'surface.tooltip' }),
      readoutText: T('--scrim-tooltip-text', 'readout chip text', { alias: 'text.onTooltip' }),
      artboardLabel: T('--scrim-text-3', 'artboard name label above a root frame (11 px)', { alias: 'text.tertiary' }),
      canvasDot: T('--scrim-canvas-dot', '1 px dot grid on the canvas field; spacing adapts to zoom'),
      checkerA: lit(hex(parse(theme === 'dark' ? '#313338' : '#e1e1df')), 'transparency checkerboard, first square', src(theme, '--scrim-checker'), { var: '--scrim-checker' }),
      checkerB: lit(hex(parse(theme === 'dark' ? '#232428' : '#ffffff')), 'transparency checkerboard, second square', src(theme, '--scrim-checker'), { var: '--scrim-checker' }),
    },
    scrollbar: {
      thumb: T('--scrim-scrollbar-thumb', 'thin scrollbar thumb'),
      thumbHover: T('--scrim-scrollbar-thumb-hover', 'scrollbar thumb hovered'),
      track: T('--scrim-scrollbar-track', 'scrollbar track (transparent)'),
    },
    picker: {
      thumb: T('--scrim-picker-thumb', 'colour picker thumb (theme-independent)'),
      thumbRing: T('--scrim-picker-thumb-ring', 'colour picker thumb ring'),
    },
    terminal: Object.fromEntries([
      ['background', T('--scrim-bg-1', 'xterm background', { alias: 'surface.panel' })],
      ['foreground', T('--scrim-text-1', 'xterm foreground', { alias: 'text.primary' })],
      ['cursor', T('--scrim-accent-mark', 'xterm cursor', { alias: 'accent.mark' })],
      ['selection', T('--scrim-ansi-selection', 'xterm selection')],
      ...['black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white', 'bright-black', 'bright-red', 'bright-green', 'bright-yellow', 'bright-blue', 'bright-magenta', 'bright-cyan', 'bright-white'].map((n) => [n.replace(/-(\w)/, (_, c) => c.toUpperCase()), T(`--scrim-ansi-${n}`, `ANSI ${n}`)]),
    ]),
  };
}

const dim = (value, unit = 'px') => ({ value, unit });
function d(value, description, v, extra = {}) {
  tokenCount++;
  return { $value: dim(value), $description: description, $extensions: { scrim: { ...(v ? { var: v, css: R.source.base[v] } : {}), ...extra } } };
}
function n(value, description, extra = {}) {
  tokenCount++;
  return { $value: value, $description: description, ...(Object.keys(extra).length ? { $extensions: { scrim: extra } } : {}) };
}

function shadowTok(theme, v, layers, description) {
  tokenCount++;
  return { $value: layers, $description: description, $extensions: { scrim: { var: v, css: src(theme, v) } } };
}
const sh = (color, x, y, blur, spread, inset = false) => ({ color, offsetX: dim(x), offsetY: dim(y), blur: dim(blur), spread: dim(spread), ...(inset ? { inset: true } : {}) });

// ── accent presets ─────────────────────────────────────────────────────────
const PRESETS = [['orange', '#ff5a1f'], ['blue', '#4a86ff'], ['violet', '#8e7bff'], ['green', '#2fb36b'], ['pink', '#ec4f98'], ['teal', '#1fb5a6']];
const presets = Object.fromEntries(PRESETS.map(([id, base]) => {
  const r = deriveBoth(base);
  return [id, { base, dark: r.dark, light: r.light, adjusted: r.adjusted }];
}));

// ── the document ───────────────────────────────────────────────────────────
const tokens = {
  $description: 'Scrim editor chrome design system, resolved for transfer to an egui theme. Source of truth: apps/studio/src/theme.css (:root blocks) and apps/studio/src/appearance/derive.ts. Every colour is a concrete sRGB value; the original CSS is in $extensions.scrim.css. Chrome only: none of this ever styles a design node.',
  $extensions: {
    scrim: {
      sources: ['apps/studio/src/theme.css', 'apps/studio/src/appearance/derive.ts', 'apps/studio/src/appearance/presets.ts', 'apps/studio/src/editor/useOverlayDraw.ts', 'packages/engine/src/overlay.ts', 'packages/engine/src/measure/draw.ts', 'docs/design/ui-system.md'],
      resolvedWith: `theme.css loaded into headless Chrome ${R.chrome} (playwright-core) with html[data-theme=dark|light]; custom properties read with getComputedStyle; color-mix() and rgb(var() / a) resolved by assigning them to a probe element's color. Alpha-composited values (opacity, globalAlpha) and filter: brightness() are computed in sRGB by hand.`,
      generated: '2026-10-19',
      colorFormat: '$value is #rrggbb, or #rrggbbaa when translucent (straight, unpremultiplied alpha). $extensions.scrim.rgba repeats translucent ones as rgba().',
      dimensionFormat: 'DTCG 2025 dimension objects: {value, unit}. 1 px = 1 logical point (egui points).',
      defaultTheme: 'dark (also the fallback before a theme resolves); the preference is system | light | dark, system by default',
      accentNote: 'accent.* and agent.* colours below are the DEFAULTS (orange #ff5a1f, violet #8e7bff) already derived; any other base colour must be run through accentRule.',
    },
  },
  color: { $type: 'color', dark: colors('dark'), light: colors('light') },
  accentRule: {
    $description: 'How the accent (and agent) tokens are derived from ONE base colour per role. A port must reimplement derive.ts; the presets below are precomputed.',
    defaults: { accent: '#ff5a1f', agent: '#8e7bff' },
    roleTokens: { mark: 'icons, selected edges, focus ring, canvas selection, coloured text', fill: 'primary buttons, checked boxes, canvas chips', onFill: 'label on the fill', tint: 'low-alpha wash of the MARK' },
    contrast: { textAA: 4.5, markAA: 3, formula: 'WCAG 2.x relative luminance, sRGB' },
    ramp: { dark: { canvas: '#17181a', panel: '#111214', field: '#1b1c1f', raised: '#232428', selected: '#2c2d32', float: '#232428' }, light: { canvas: '#ececea', panel: '#ffffff', field: '#f4f4f3', raised: '#ebebea', selected: '#e1e1df', float: '#ffffff' } },
    steps: [
      'Parse #rgb / #rrggbb; anything else is refused ("Use a hex colour such as #ff5a1f.").',
      'Convert to HSL. Refuse when saturation < 0.18 or lightness > 0.97 or < 0.03 (reads as grey).',
      'MARK, per theme: must reach 4.5:1 on panel, field, raised and float, and 3:1 on canvas and selected. If the base fails, move LIGHTNESS only (hue and saturation kept) in steps of 0.005, up to 200 steps: lighter on dark, darker on light. First passing value wins; none passes means refused.',
      'FILL, per theme: the base itself if some label (#ffffff or #111214) reaches 4.5:1 on it AND the fill is 3:1 against the panel; else the nearest lighter HSL step that passes, else the nearest darker one.',
      'ON-FILL: whichever of #ffffff and #111214 has the higher ratio on the fill (ties go to white); it must be >= 4.5.',
      'TINT: the MARK rgb at alpha 0.16 (dark) / 0.12 (light).',
      'The agent colour is refused when it is "too close" to the accent: hue distance < 20 degrees AND lightness difference < 0.2 (and vice versa).',
      'Reported adjustment: "Lightened" / "Darkened" when the mark moved, "fill adjusted" when the fill moved; the light theme only reports when the dark theme moved too, or the light mark moved more than 0.3 in lightness.',
    ],
    presets: { $description: 'The six Settings presets, derived for both themes (role tokens identical for accent and agent).', ...presets },
  },
  fontFamily: {
    $type: 'fontFamily',
    sans: n(['IBM Plex Sans', 'system-ui', '-apple-system', 'sans-serif'], 'every label, value and prose in the chrome (weights 400 and 500 bundled; 600 ships but is unused)', { var: '--scrim-font-sans', css: R.source.base['--scrim-font-sans'] }),
    mono: n(['IBM Plex Mono', 'ui-monospace', 'SFMono-Regular', 'Menlo', 'monospace'], 'code only: the Code tab, the terminal, canvas measurement chips', { var: '--scrim-font-mono', css: R.source.base['--scrim-font-mono'] }),
  },
  fontSize: {
    $type: 'dimension',
    '11': d(11, 'section labels, meta, shortcuts, units, chips, badges, canvas labels', '--scrim-size-11'),
    '12': d(12, 'the body size: row labels, values, layer names, menu rows, buttons, tooltips', '--scrim-size-12'),
    '13': d(13, 'panel titles, dialog titles, inspector tabs, the selected node name, palette search', '--scrim-size-13'),
    canvasMeasure: d(10, 'measurement chip digits on the Canvas2D overlay (Plex Mono); the only 10 px text', null, { css: "10px 'IBM Plex Mono'", at: 'packages/engine/src/measure/draw.ts' }),
  },
  fontWeight: {
    $type: 'fontWeight',
    regular: n(400, 'everything by default', { var: '--scrim-weight-regular' }),
    medium: n(500, 'section labels, panel/dialog titles, selected node name, active tab, button labels, selected layer row, active segment', { var: '--scrim-weight-medium' }),
  },
  lineHeight: {
    $type: 'dimension',
    '16': d(16, 'with 11 and 12 px text', '--scrim-lh-16'),
    '18': d(18, 'with 13 px text; also dialog body copy', '--scrim-lh-18'),
    $extensions: { scrim: { exceptions: { codeView: 1.7, dialogBody: '18px', tooltipsWrapping: 1.35, hintsAndNotes: '1.35 – 1.5', lintCard: 1.4 } } },
  },
  letterSpacing: { $type: 'dimension', none: d(0, 'no letter-spacing anywhere; no uppercase; sentence case') },
  typography: {
    $type: 'typography',
    $description: 'Composite text styles. Every numeric value uses tabular numerals (font-variant-numeric: tabular-nums).',
    ...Object.fromEntries([
      ['body', 'sans', 12, 400, 16, 'default chrome text: row labels, values, layer names, menu rows, fields, tooltips (text.primary / text.secondary)'],
      ['bodyStrong', 'sans', 12, 500, 16, 'button labels, selected layer row, active segment, breadcrumb title, Home file names'],
      ['small', 'sans', 11, 400, 16, 'meta, units, shortcuts, chips, kind labels, notes (text.tertiary)'],
      ['label', 'sans', 11, 500, 16, 'section header labels, popover group heads, picker title (text.secondary, sentence case)'],
      ['title', 'sans', 13, 500, 18, 'panel titles, dialog titles, the selected node name, active inspector tab'],
      ['tab', 'sans', 13, 400, 18, 'inactive inspector tab (text.tertiary; hover text.secondary)'],
      ['code', 'mono', 12, 400, '1.7', 'the Code tab (text.tertiary base; tags agent.mark, strings accent.mark, attrs text.secondary, line numbers text.disabled)'],
      ['canvasLabel', 'sans', 11, 400, null, 'Canvas2D overlay text: artboard labels, distance chips, agent chip'],
      ['canvasMeasure', 'mono', 10, 400, null, 'measurement chip digits'],
    ].map(([k, fam, size, weight, lh, desc]) => {
      tokenCount++;
      return [k, { $value: { fontFamily: `{fontFamily.${fam}}`, fontSize: dim(size), fontWeight: weight, lineHeight: lh === null ? null : typeof lh === 'number' ? dim(lh) : Number(lh), letterSpacing: dim(0) }, $description: desc, $extensions: { scrim: { tabularNums: true } } }];
    })),
  },
  space: {
    $type: 'dimension',
    $description: '4 px grid; space-0 is the one optical 2 px step. Padding / margin / gap in chrome is only these (plus 0, 1, 2 px).',
    ...Object.fromEntries([[0, 2, 'optical step: segment track inset, kbd tweaks'], [1, 4, 'inside controls, icon-to-text, tree padding'], [2, 8, 'between paired fields, button side padding, menu inset'], [3, 12, 'panel side padding, section padding'], [4, 16, 'between groups, tab gap'], [5, 20, 'toolbar dividers, vertical rules'], [6, 24, 'Home gutters, dialog padding'], [8, 32, 'dialog margin from viewport']].map(([k, v, desc]) => [String(k), d(v, desc, `--scrim-space-${k}`)])),
  },
  size: {
    $type: 'dimension',
    row: d(28, 'THE row: every inspector, layer, menu, Settings and Document-panel row', '--scrim-row'),
    control: d(24, 'fields, buttons, segmented tracks, icon buttons, tooltips (min height), swatch buttons', '--scrim-control'),
    controlSmall: d(20, 'a segment inside its 24 px track; small buttons; chips', '--scrim-control-sm'),
    sectionHead: d(32, 'inspector section header, layer filter row, picker head, font search', '--scrim-section-head'),
    topbar: d(40, 'the top bar; also panel headers, inspector tabs, the node header, palette search', '--scrim-topbar'),
    toolbar: d(40, 'the floating tool bar thickness', '--scrim-toolbar'),
    tool: d(28, 'a tool button inside the tool bar', '--scrim-tool'),
    panelPad: d(12, 'left/right padding of every panel', '--scrim-panel-pad'),
    labelColumn: d(72, 'the inspector label column', '--scrim-label-col'),
    indent: d(16, 'one layer-tree level (guide line at 10 px)', '--scrim-indent'),
    rail: d(40, 'collapsed side rail', '--scrim-rail-w'),
    layersPanel: d(264, 'layers panel default width (min 200, max 420)', '--scrim-layers-w', { min: 200, max: 420 }),
    inspectorPanel: d(300, 'inspector width', '--scrim-inspector-w'),
    lintPanel: d(400, 'floating lint panel width', '--scrim-lint-w'),
    dock: d(340, 'agent dock default height', '--scrim-dock-h'),
    footer: d(24, 'panel footer', '--scrim-footer-h'),
    status: d(28, 'status strip', '--scrim-status-h'),
    homeSidebar: d(220, 'Home sidebar', '--scrim-home-side-w'),
    homeAside: d(280, 'Home details aside', '--scrim-home-aside-w'),
    homeTopbar: d(40, 'Home top bar', '--scrim-home-top-h'),
    homeThumb: d(150, 'Home card thumbnail height (cards keep 4:3)', '--scrim-home-thumb-h'),
    menuMinWidth: d(200, 'context menu min width', null, { css: '.scrim-menu { min-width: 200px }' }),
    dialog: d(400, 'dialog width (max 100vw - 32)', null, { css: '.scrim-dialog { width: 400px }' }),
    settingsDialog: d(480, 'Settings dialog width', null, { css: '.scrim-settings { width: 480px }' }),
    palette: d(560, 'command palette width, max-height 60vh', null, { css: '.scrim-palette { width: 560px }' }),
    picker: d(272, 'colour picker popover width', null, { css: '.scrim-picker { width: 272px }' }),
    checkbox: d(14, 'checkbox box (check glyph 10 px)', '--scrim-icon-14'),
    keycap: d(16, 'keycap height and min width', '--scrim-space-4'),
    swatch: d(13, 'inline colour swatch in a field', null, { css: '.scrim-swatch { width: 13px }' }),
    dot: d(4, 'status / chip dot (agent status dot is 8 px)', '--scrim-space-1'),
    canvasHandle: d(7, 'resize handle square, screen px, zoom-independent (hit slop 4 px)', null, { css: 'HANDLE_SIZE = 7', at: 'packages/engine/src/overlay.ts' }),
    canvasChipHeight: d(14, 'distance chip height (agent chip 15)', null, { at: 'packages/engine/src/overlay.ts' }),
  },
  icon: {
    $type: 'dimension',
    small: d(14, 'inside rows and fields; layer glyphs; carets; chevrons', '--scrim-icon-14'),
    medium: d(16, 'top bar, tool bar, section actions', '--scrim-icon-16'),
    stroke: d(1.25, 'outline icons on a 16 px grid, round caps and joins; state changes colour, never fill', null, { css: 'stroke-width 1.25 (ui-system.md › Icons; the chevron mask)' }),
  },
  radius: {
    $type: 'dimension',
    control: d(4, 'fields, buttons, rows, menu highlights, swatches, keycaps, checkboxes, chips', '--scrim-radius-control'),
    float: d(8, 'menus, popovers, dialogs, toasts, tooltips, the tool bar, Home cards', '--scrim-radius-float'),
    round: d(9999, 'presence avatars, status dots, the warning badge, the modifier pill only', '--scrim-radius-round'),
  },
  borderWidth: {
    $type: 'dimension',
    hairline: d(1, 'every separator and edge', '--scrim-hairline'),
    focusRing: d(2, 'focus ring width', '--scrim-ring'),
    focusRingOffset: d(1, 'focus ring offset (outside the control)', '--scrim-ring-offset'),
    selectedEdge: d(2, 'selected layer row left edge; drop insert line', '--scrim-ring'),
    canvasSelection: d(1.5, 'canvas selection, handles, draft (Canvas2D lineWidth)', null, { at: 'packages/engine/src/overlay.ts' }),
    canvasThin: d(1, 'canvas hover, guides, spacing ticks, agent outline, drop container', null, { at: 'packages/engine/src/overlay.ts' }),
  },
  shadow: {
    $type: 'shadow',
    dark: {
      float: shadowTok('dark', '--scrim-shadow-float', [sh('#313338', 0, 0, 0, 1), sh('#00000066', 0, 8, 24, 0)], 'the ONE shadow: floating surfaces only (menus, popovers, dialogs, tooltips, tool bar). First layer is a 1 px line-2 ring'),
      artboard: shadowTok('dark', '--scrim-artboard-shadow', [sh('#ffffff0f', 0, 0, 0, 1), sh('#000000b3', 0, 18, 40, -20)], 'root-frame artboard outline + drop (canvas)'),
    },
    light: {
      float: shadowTok('light', '--scrim-shadow-float', [sh('#d4d4d1', 0, 0, 0, 1), sh('#0000001a', 0, 8, 24, 0)], 'the ONE shadow: floating surfaces only'),
      artboard: shadowTok('light', '--scrim-artboard-shadow', [sh('#0000001a', 0, 1, 2, 0), sh('#00000059', 0, 18, 40, -24)], 'root-frame artboard shadow (canvas)'),
    },
    $extensions: { scrim: { rule: 'Docked panels are flat, separated by 1 px hairlines; no shadow, gradient, glow or blur anywhere in the chrome.' } },
  },
  focus: {
    ring: (tokenCount++, { $value: { width: dim(2), offset: dim(1), color: '{color.<theme>.accent.mark}', colorOnAccentFill: '{color.<theme>.text.primary}' }, $description: 'One focus ring for every keyboard-focused control (:focus-visible): 2 px accent outline, 1 px outside. Mouse focus on a field shows only the hover border. A field draws the ring on its wrapper, not the bare input. Never removed without a replacement.', $extensions: { scrim: { css: ':focus-visible { outline: var(--scrim-ring) solid var(--scrim-accent-mark); outline-offset: var(--scrim-ring-offset) }' } } }),
  },
  motion: {
    duration: (tokenCount++, { $type: 'duration', $value: dim(120, 'ms'), $description: 'the one duration: hover, press, colour, border, panel collapse, popover open, caret fade', $extensions: { scrim: { var: '--scrim-duration', css: '120ms', reducedMotion: '0ms (prefers-reduced-motion: reduce sets every chrome transition and animation to 0)' } } }),
    easing: (tokenCount++, { $type: 'cubicBezier', $value: [0.2, 0, 0, 1], $description: 'the one easing', $extensions: { scrim: { var: '--scrim-easing', css: 'cubic-bezier(0.2, 0, 0, 1)' } } }),
    $extensions: { scrim: { rules: ['No animation on canvas content (the design world is inert).', 'No infinite animations in chrome: a running agent is a static dot and a word.'], exceptions: { dockResize: '120ms cubic-bezier(0.22, 1, 0.36, 1)', revealFlash: 'scrim-reveal 1.6s ease-out (2 px accent ring fading out, controls a link points at)' } } },
  },
  opacity: {
    $type: 'number',
    disabledField: n(0.5, 'a disabled field'),
    canvasHover: n(0.45, 'canvas hover outline'),
    canvasMarquee: n(0.08, 'marquee fill'),
    canvasGuide: n(0.75, 'snap guides'),
    canvasDisabledHandles: n(0.35, 'handles when the group cannot resize'),
    canvasMeasureBand: n(0.16, 'measurement band fill'),
    tintDark: n(0.16, 'role tint alpha, dark'),
    tintLight: n(0.12, 'role tint alpha, light'),
  },
  zIndex: {
    $type: 'number',
    $description: 'Chrome stacking order (DOM z-index). Design content never takes part.',
    panelHeaderSticky: n(1, '.scrim-panel-header, tool caret'),
    resizeHandle: n(2, '.scrim-resize-handle'),
    dockResize: n(3, '.scrim-dock__resize'),
    snapCanvasLayer: n(8, 'theme.css:2140 (snapping layer)'),
    lintPanel: n(9, 'floating lint panel; canvas hint row; dock layers'),
    toolbar: n(10, 'the floating tool bar'),
    toolFlyout: n(11, 'a tool family flyout'),
    canvasHint: n(30, 'canvas hints and toasts'),
    home: n(50, 'Home over the hidden editor (home.css)'),
    popover: n(1000, 'menus, font / snap popovers, arrange tooltip'),
    modal: n(1100, 'dialog scrim, command palette scrim, modifier pill'),
  },
};

writeFileSync(`${OUT}/tokens.json`, JSON.stringify(tokens, null, 2) + '\n');

// ── contrast pairs ─────────────────────────────────────────────────────────
const SURF = { 'bg-0': 'surface.canvas', 'bg-1': 'surface.panel', 'bg-2': 'surface.field', 'bg-3': 'surface.raised', 'bg-4': 'surface.selected', 'bg-float': 'surface.float', 'float-hover': 'surface.floatHover', 'tooltip-bg': 'surface.tooltip' };
const SURFACES = ['bg-0', 'bg-1', 'bg-2', 'bg-3', 'bg-float', 'float-hover'];
const pairs = [];
const P = (fg, bg, kind, usage, source = 'themeContrast.test.ts') => pairs.push({ fg, bg, kind, usage, source });
const usageText = { 'text-1': 'primary text', 'text-2': 'secondary text / labels', 'text-3': 'tertiary text: meta, units, shortcuts' };
const surfUse = { 'bg-0': 'on the canvas field', 'bg-1': 'on panels', 'bg-2': 'on fields / hovered rows', 'bg-3': 'on raised / tool bar', 'bg-float': 'on menus and popovers', 'float-hover': 'on a highlighted menu row', 'bg-4': 'on a selected row' };
for (const fg of ['text-1', 'text-2', 'text-3']) for (const bg of SURFACES) P(fg, bg, 'text', `${usageText[fg]} ${surfUse[bg]}`);
for (const fg of ['text-1', 'text-2']) P(fg, 'bg-4', 'text', `${usageText[fg]} ${surfUse['bg-4']}`);
for (const fg of ['danger', 'warning', 'accent-mark', 'agent-mark']) for (const bg of ['bg-1', 'bg-2', 'bg-float']) P(fg, bg, 'text', `${fg} as text ${surfUse[bg]}`);
for (const fg of ['danger', 'warning', 'accent-mark', 'agent-mark']) for (const bg of [...SURFACES, 'bg-4']) P(fg, bg, 'mark', `${fg} as a mark (dot, edge, ring, icon) ${surfUse[bg]}`);
P('accent-on-fill', 'accent-fill', 'text', 'primary button label; checkbox check; canvas distance chip');
P('agent-on-fill', 'agent-fill', 'text', 'agent chip on the canvas');
P('on-danger', 'danger-fill', 'text', 'destructive button label');
P('on-warning', 'warning', 'text', 'lint badge count; failing-contrast chip in the picker');
P('tooltip-text', 'tooltip-bg', 'text', 'tooltips; canvas readout chips');
P('on-measure', 'measure', 'text', 'measurement chip digits (10 px)');
P('accent-fill', 'bg-1', 'mark', 'the primary button shape against the panel');
P('text-off', 'bg-1', 'disabled', 'disabled text on the panel (held to 3:1 by the test)');
P('text-1', 'bg-1', 'text', 'terminal foreground');
for (const c of ['red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'bright-red', 'bright-green', 'bright-yellow', 'bright-blue', 'bright-magenta', 'bright-cyan']) P(`ansi-${c}`, 'bg-1', 'text', `terminal ANSI ${c}`);
// observed in theme.css, not in the contrast test
const OBS = 'observed in theme.css (not in themeContrast.test.ts)';
P('text-3', 'bg-4', 'text', 'layer meta ("row · 28", 11 px) on the SELECTED layer row (theme.css .scrim-layer-row[data-selected] .scrim-layer-meta); ui-system.md says text-3 is never on bg-4 under 12 px', OBS);
P('agent-mark', 'bg-4', 'text', 'agent-edited marker (11 px) on a selected layer row', OBS);
P('accent-mark', 'bg-4', 'mark', 'selected layer row: 2 px accent edge on bg-4', OBS);
P('text-off', 'bg-2', 'disabled', 'disabled text on a field / secondary button', OBS);
P('text-off', 'bg-float', 'disabled', 'disabled menu / palette row', OBS);
P('text-off', 'bg-4', 'disabled', 'disabled primary button label (bg-4 background)', OBS);
P('text-off', 'bg-1', 'text', 'Code tab line numbers (.scrim-code-line__n): NOT disabled, so held to 4.5:1', OBS);
P('text-2', 'bg-4', 'text', 'keycap label (.scrim-kbd--key)', OBS);
P('text-1', 'tooltip-bg', 'text', 'modifier pill (.scrim-mod-pill) sets color: text-1 on tooltip-bg', OBS);
P('text-1', 'bg-1', 'mark', 'focus ring on the primary button: drawn in text-1, 1 px OUTSIDE the fill, so it sits on the panel (against the fill itself it is 2.64:1 dark / 6.01:1 light)', OBS);
P('accent-mark', 'artboard', 'mark', 'canvas selection outline on a white artboard (default paper)', OBS);
P('agent-mark', 'artboard', 'mark', 'agent outline on a white artboard', OBS);
P('text-3', 'bg-0', 'text', 'artboard name labels on the canvas field (11 px)', OBS);
P('accent-fill', 'bg-0', 'mark', 'canvas distance chip block on the canvas field', OBS);
P('measure', 'bg-0', 'mark', 'measurement lines on the canvas field', OBS);
P('measure', 'artboard', 'mark', 'measurement lines over a white artboard', OBS);
P('text-1', 'warning-tint@bg-1', 'text', 'lint card / group text on the warning wash over the panel', OBS);
P('warning', 'warning-tint@bg-1', 'text', 'lint chip / group count on the warning wash over the panel', OBS);
P('accent-mark', 'accent-tint@bg-1', 'mark', 'drop-inside row outline on the accent wash', OBS);
P('text-1', 'accent-tint@bg-1', 'text', 'layer name on a drop-inside row', OBS);
P('text-1', 'disabledField', 'disabled', 'disabled field text (opacity 0.5 on text-off over bg-2 at 0.5)', OBS);
P('accent-on-fill', 'primaryHover', 'text', 'primary button label while hovered (brightness 0.92)', OBS);
P('accent-on-fill', 'primaryPressed', 'text', 'primary button label while pressed (brightness 0.84)', OBS);
P('on-danger', 'dangerFillHover', 'text', 'destructive button label while hovered', OBS);

function resolvePair(theme, name) {
  const cs = tokens.color[theme];
  if (name === 'artboard') return col(theme, '--scrim-artboard');
  if (name === 'disabledField') return parse(cs.state.disabledFieldBg.$value);
  if (name === 'primaryHover') return parse(cs.state.primaryHover.$value);
  if (name === 'primaryPressed') return parse(cs.state.primaryPressed.$value);
  if (name === 'dangerFillHover') return parse(cs.feedback.dangerFillHover.$value);
  if (name.includes('@')) {
    const [a, b] = name.split('@');
    return over(col(theme, `--scrim-${a}`), col(theme, `--scrim-${b}`));
  }
  return col(theme, `--scrim-${name}`);
}
const MIN = { text: 4.5, mark: 3, disabled: 3 };
const out = { dark: [], light: [] };
for (const theme of THEMES) {
  for (const p of pairs) {
    const bg = resolvePair(theme, p.bg);
    let fg = resolvePair(theme, p.fg);
    if (p.fg === 'text-1' && p.bg === 'disabledField') fg = parse(tokens.color[theme].state.disabledFieldText.$value);
    if (fg.a < 1) fg = over(fg, bg);
    const r = ratio(fg, bg);
    const min = MIN[p.kind];
    const aa = p.kind === 'disabled' ? 'exempt' : r >= min ? 'pass' : 'fail';
    out[theme].push({ fg: p.fg === 'text-1' && p.bg === 'disabledField' ? 'disabledFieldText' : p.fg, bg: p.bg, fgHex: hex(fg), bgHex: hex(bg), ratio: Math.round(r * 100) / 100, kind: p.kind, min, aa, ...(p.kind === 'disabled' ? { meets3: r >= 3 } : {}), usage: p.usage, source: p.source });
  }
}
const presetChecks = {};
for (const [id, pr] of Object.entries(presets)) {
  presetChecks[id] = {};
  for (const theme of THEMES) {
    const t = pr[theme];
    const rp = tokens.accentRule.ramp[theme];
    presetChecks[id][theme] = {
      onFillOnFill: Math.round(dContrast(t.onFill, t.fill) * 100) / 100,
      markMinOnTextSurfaces: Math.round(Math.min(...['panel', 'field', 'raised', 'float'].map((s) => dContrast(t.mark, rp[s]))) * 100) / 100,
      markMinOnCanvasAndSelected: Math.round(Math.min(dContrast(t.mark, rp.canvas), dContrast(t.mark, rp.selected)) * 100) / 100,
      fillOnPanel: Math.round(dContrast(t.fill, rp.panel) * 100) / 100,
    };
  }
}
const failures = Object.fromEntries(THEMES.map((t) => [t, out[t].filter((p) => p.aa === 'fail').map((p) => `${p.fg} on ${p.bg}: ${p.ratio}:1 (needs ${p.min}) — ${p.usage}`)]));
const contrastDoc = {
  $description: 'Every text / mark / background pair the Scrim chrome uses, with its WCAG 2.x contrast ratio, in both themes, at the DEFAULT accent (orange) and agent (violet). Pairs marked themeContrast.test.ts are the ones the repo enforces; the rest were found by reading theme.css.',
  method: 'WCAG 2.x relative luminance on sRGB values resolved in headless Chrome (see tokens.json $extensions.scrim.resolvedWith). Translucent colours are composited over the stated background first.',
  floors: { text: '4.5:1 (every chrome text is under 18 px, so all of it is "small text")', mark: '3:1 (WCAG 1.4.11 non-text: dots, edges, rings, icons, the primary button shape)', disabled: 'exempt from 1.4.3; reported with meets3 (the repo holds text-off to 3:1 on the panel)' },
  tokenNames: 'fg / bg are theme.css token names without the --scrim- prefix; "a@b" is token a composited over token b; artboard = --scrim-artboard; primaryHover / primaryPressed / dangerFillHover / disabledField are the computed state colours in tokens.json.',
  failures,
  dark: out.dark,
  light: out.light,
  accentPresets: { $description: 'Each Settings preset after derive.ts, against the neutral ramp: the guarantees the rule enforces (onFill >= 4.5, mark >= 4.5 on text surfaces and >= 3 on canvas / selected, fill >= 3 on the panel).', ...presetChecks },
};
writeFileSync(`${OUT}/contrast.json`, JSON.stringify(contrastDoc, null, 2) + '\n');
console.log('tokens', tokenCount, 'pairs/theme', pairs.length);
console.log(JSON.stringify(failures, null, 2));
console.log(JSON.stringify(presetChecks));
