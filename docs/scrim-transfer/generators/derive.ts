/** Appearance colours → chrome tokens. ONE pure function turns a chosen base
 * colour into the four tokens a role needs in each theme:
 *
 *  - mark:   icons, selected edges, the focus ring, coloured text on a surface;
 *  - fill:   primary buttons and badges;
 *  - onFill: the label on that fill (white or near-black, whichever passes);
 *  - tint:   a low-opacity wash (drop targets, bands).
 *
 * Contract (WCAG 2.x): the mark reads as text (4.5:1) on every text surface
 * (bg-1, bg-2, bg-3, the floating surface) and as a UI mark (3:1) on the
 * canvas field and the selected-row surface; the fill's label reads at 4.5:1
 * and the fill itself shows at 3:1 on the panel. When the base fails, its
 * LIGHTNESS moves (hue and saturation kept) until it passes, and the result
 * says what changed. A colour that cannot be made to pass, or a grey (it
 * would read as the interface, not a colour), is rejected.
 *
 * Chrome only: these tokens never reach a design node, an export or a
 * render-window screenshot. No imports — main (apps/desktop) validates with
 * the same function through `@scrim/studio/appearance`. */

export type AppearanceTheme = 'dark' | 'light';

export interface RoleTokens {
  mark: string;
  fill: string;
  onFill: string;
  tint: string;
}

export type Derived = { ok: true; tokens: RoleTokens; adjusted: string | null } | { ok: false; reason: string };

/** The neutral ramp the contract is checked against (theme.css --ui-bg-*). */
export const RAMP: Record<AppearanceTheme, { canvas: string; panel: string; field: string; raised: string; selected: string; float: string }> = {
  dark: { canvas: '#17181a', panel: '#111214', field: '#1b1c1f', raised: '#232428', selected: '#2c2d32', float: '#232428' },
  light: { canvas: '#ececea', panel: '#ffffff', field: '#f4f4f3', raised: '#ebebea', selected: '#e1e1df', float: '#ffffff' },
};

/** Label colours on a fill. */
export const ON_LIGHT = '#111214';
export const ON_DARK = '#ffffff';
export const TINT_ALPHA: Record<AppearanceTheme, number> = { dark: 0.16, light: 0.12 };

export const TEXT_AA = 4.5;
export const MARK_AA = 3;
/** Saturation under this is a grey: rejected. */
const MIN_SATURATION = 0.18;

// ── colour maths ───────────────────────────────────────────────────────────

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

export function parseHex(hex: string): Rgb | null {
  const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return null;
  const h = m[1]!.length === 3 ? [...m[1]!].map((c) => c + c).join('') : m[1]!;
  return { r: parseInt(h.slice(0, 2), 16), g: parseInt(h.slice(2, 4), 16), b: parseInt(h.slice(4, 6), 16) };
}

export function toHex({ r, g, b }: Rgb): string {
  const c = (v: number): string => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, '0');
  return `#${c(r)}${c(g)}${c(b)}`;
}

/** A normalised `#rrggbb`, or null for anything else. */
export function normalizeHex(hex: string): string | null {
  const rgb = parseHex(hex);
  return rgb ? toHex(rgb) : null;
}

const channel = (v: number): number => {
  const s = v / 255;
  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
};

export function luminance(c: Rgb): number {
  return 0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b);
}

export function contrast(a: string | Rgb, b: string | Rgb): number {
  const la = luminance(typeof a === 'string' ? parseHex(a)! : a);
  const lb = luminance(typeof b === 'string' ? parseHex(b)! : b);
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

export interface Hsl {
  h: number;
  s: number;
  l: number;
}

export function toHsl({ r, g, b }: Rgb): Hsl {
  const [R, G, B] = [r / 255, g / 255, b / 255];
  const max = Math.max(R, G, B);
  const min = Math.min(R, G, B);
  const l = (max + min) / 2;
  if (max === min) return { h: 0, s: 0, l };
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  const h = max === R ? (G - B) / d + (G < B ? 6 : 0) : max === G ? (B - R) / d + 2 : (R - G) / d + 4;
  return { h: h * 60, s, l };
}

export function fromHsl({ h, s, l }: Hsl): Rgb {
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const k = (t: number): number => {
    const u = t < 0 ? t + 1 : t > 1 ? t - 1 : t;
    if (u < 1 / 6) return p + (q - p) * 6 * u;
    if (u < 1 / 2) return q;
    if (u < 2 / 3) return p + (q - p) * (2 / 3 - u) * 6;
    return p;
  };
  const hh = h / 360;
  return { r: k(hh + 1 / 3) * 255, g: k(hh) * 255, b: k(hh - 1 / 3) * 255 };
}

/** `fg` over `bg` at `alpha`, composited. */
export function over(fg: Rgb, bg: Rgb, alpha: number): Rgb {
  return { r: fg.r * alpha + bg.r * (1 - alpha), g: fg.g * alpha + bg.g * (1 - alpha), b: fg.b * alpha + bg.b * (1 - alpha) };
}

// ── the contract ───────────────────────────────────────────────────────────

/** The smallest lightness move from `base` (towards light on a dark theme,
 * towards dark on a light one) for which `passes` holds; null when none does. */
function shiftLightness(base: Hsl, theme: AppearanceTheme, passes: (hex: string) => boolean): string | null {
  const start = toHex(fromHsl(base));
  if (passes(start)) return start;
  const dir = theme === 'dark' ? 1 : -1;
  for (let step = 1; step <= 200; step++) {
    const l = base.l + (dir * step) / 200;
    if (l < 0 || l > 1) break;
    const hex = toHex(fromHsl({ ...base, l }));
    if (passes(hex)) return hex;
  }
  return null;
}

/** Mark: 4.5:1 on the text surfaces, 3:1 on the canvas and the selected row. */
export function markPasses(hex: string, theme: AppearanceTheme): boolean {
  const r = RAMP[theme];
  return [r.panel, r.field, r.raised, r.float].every((s) => contrast(hex, s) >= TEXT_AA) && [r.canvas, r.selected].every((s) => contrast(hex, s) >= MARK_AA);
}

/** The better-reading label on a fill, when one reaches 4.5:1. */
export function labelFor(fill: string): string | null {
  const light = contrast(fill, ON_DARK);
  const dark = contrast(fill, ON_LIGHT);
  const best = light >= dark ? ON_DARK : ON_LIGHT;
  return Math.max(light, dark) >= TEXT_AA ? best : null;
}

function fillPasses(hex: string, theme: AppearanceTheme): boolean {
  return labelFor(hex) !== null && contrast(hex, RAMP[theme].panel) >= MARK_AA;
}

const tintOf = (mark: string, theme: AppearanceTheme): string => {
  const { r, g, b } = parseHex(mark)!;
  return `rgba(${r}, ${g}, ${b}, ${TINT_ALPHA[theme]})`;
};

/** Derive the role's tokens for `theme` from `base`. */
export function deriveRole(base: string, theme: AppearanceTheme): Derived {
  const rgb = parseHex(base);
  if (!rgb) return { ok: false, reason: 'Use a hex colour such as #ff5a1f.' };
  const hsl = toHsl(rgb);
  if (hsl.s < MIN_SATURATION || (hsl.l > 0.97 || hsl.l < 0.03)) return { ok: false, reason: 'Too close to grey to read as a colour: pick a more saturated one.' };
  const mark = shiftLightness(hsl, theme, (h) => markPasses(h, theme));
  // the fill tries the base first, then darker (white label) and lighter (dark label)
  const fill = fillPasses(toHex(rgb), theme) ? toHex(rgb) : (shiftLightness(hsl, 'light', (h) => fillPasses(h, theme)) ?? shiftLightness(hsl, 'dark', (h) => fillPasses(h, theme)));
  if (!mark || !fill) return { ok: false, reason: `Can't reach readable contrast on the ${theme} theme: pick another colour.` };
  const onFill = labelFor(fill)!;
  const changed = [mark !== toHex(rgb) ? (theme === 'dark' ? 'lightened' : 'darkened') : null, fill !== toHex(rgb) ? 'fill adjusted' : null].filter(Boolean);
  const adjusted = changed.length ? `${capital(changed.join(', '))} for contrast on the ${theme} theme.` : null;
  return { ok: true, tokens: { mark, fill, onFill, tint: tintOf(mark, theme) }, adjusted };
}

const capital = (s: string): string => s.charAt(0).toUpperCase() + s.slice(1);

/** Both themes at once; the first failure wins. `adjusted` names what moved. */
export function deriveBoth(base: string): { ok: true; dark: RoleTokens; light: RoleTokens; adjusted: string | null } | { ok: false; reason: string } {
  const dark = deriveRole(base, 'dark');
  if (!dark.ok) return dark;
  const light = deriveRole(base, 'light');
  if (!light.ok) return light;
  // the light theme darkens most saturated colours: say so only when the DARK
  // theme moved too, or when the light one moved far (a near-white base)
  const far = (a: string, b: string): boolean => Math.abs(toHsl(parseHex(a)!).l - toHsl(parseHex(b)!).l) > 0.3;
  const adjusted = dark.adjusted ?? (far(light.tokens.mark, base) ? light.adjusted : null);
  return { ok: true, dark: dark.tokens, light: light.tokens, adjusted };
}

// ── agent vs accent ────────────────────────────────────────────────────────

/** Two bases whose hue and lightness are both too close to tell apart. */
export function tooClose(a: string, b: string): boolean {
  const x = parseHex(a);
  const y = parseHex(b);
  if (!x || !y) return false;
  const p = toHsl(x);
  const q = toHsl(y);
  const dh = Math.min(Math.abs(p.h - q.h), 360 - Math.abs(p.h - q.h));
  return dh < 20 && Math.abs(p.l - q.l) < 0.2;
}
