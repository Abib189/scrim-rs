# Spec: the lint rules, one by one

The companion to `lint.md`. It uses that file's helper definitions (§8) and run order (§7).

**Every rule section gives:**
- the **id**, **category**, **domain** and **name**;
- the **needs** line: whether the rule reads only the model or needs measured rects;
- the **profile** line: `ui` is always *active* unless the rule is informational in every profile;
- the check, with its thresholds and formulas;
- the node the finding is **attributed** to;
- the **detail** template;
- the **message**, verbatim. It is the `message` field of every finding. The catalog descriptions are © Paul Bakaus, Apache-2.0 (`lint.md` §11).

`D` is the node's effective declarations (`lint.md` §7.3). Each section names its cases in `../cases/lint.json`.

## Typography

### `overused-font`
{{meta:overused-font}}
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** **any node** (not only text) where `primaryFont(D.fontFamily)` is in this list: `inter, roboto, open sans, lato, montserrat, arial, helvetica, fraunces, instrument sans, instrument serif, geist, geist sans, geist mono, mona sans, plus jakarta sans, space grotesk, recoleta`.
- **Detail:** the primary font, lower-cased (`inter`).
- **Cases:** `overused-font/positive`, `overused-font/negative`.
- **Message:** {{msg:overused-font}}

### `single-font`
{{meta:single-font}}
- **Needs:** model. **Profiles:** active in both.
- **Fires when:**
  - `F` = the set of `primaryFont(D.fontFamily)` over **every node** of the subtree (frames included), nulls dropped;
  - it fires when `|F| = 1` and the subtree has `≥ 2` text nodes (`isText`).
- **Attributed to:** the lint target.
- **Detail:** the one font.
- **Cases:** `single-font/positive`, `single-font/negative`, `scope/subtree-target`.
- **Message:** {{msg:single-font}}

### `flat-type-hierarchy`
{{meta:flat-type-hierarchy}}
- **Needs:** model. **Profiles:** active in both.
- **Fires when:**
  - `S` = `px(D.fontSize)` of each text node with a px size (own value only);
  - it fires when there are `≥ 3` text nodes, `|S| ≥ 2`, `min > 0`, and `max / min < 1.25`.
- **Attributed to:** the lint target.
- **Detail:** `<min>–<max>px` (en dash), e.g. `16–18px`.
- **Cases:** `flat-type-hierarchy/positive`, `/negative`.
- **Message:** {{msg:flat-type-hierarchy}}

### `oversized-h1`
{{meta:oversized-h1}}
- **Needs:** model. **Profiles:** `expressive`: **off**.
- **Fires on:** any text node with `px(D.fontSize) ≥ 72`. There is no heading or length test.
- **Detail:** `<size>px`.
- **Cases:** `oversized-h1/positive`, `/negative`, `profiles/*`.
- **Message:** {{msg:oversized-h1}}

### `extreme-negative-tracking`
{{meta:extreme-negative-tracking}}
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** a text node with `trackingEm(D) ≤ −0.04`.
- **Detail:** `<em.toFixed(3)>em`, e.g. `-0.050em`, `-0.063em` (from −3px at 48px).
- **Cases:** `extreme-negative-tracking/positive`, `/negative`.
- **Message:** {{msg:extreme-negative-tracking}}

### `wide-tracking`
{{meta:wide-tracking}}
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** a text node with `trackingEm(D) ≥ 0.05` and (`px(D.fontSize)` is null or `< 24`).
  - 24 = WCAG large text, `18 × 96/72`.
- **Detail:** `<em.toFixed(3)>em`.
- **Cases:** `wide-tracking/positive`, `/negative`.
- **Message:** {{msg:wide-tracking}}

### `tiny-text`
{{meta:tiny-text}}
- **Needs:** model. **Profiles:** active in both; **never relaxed**.
- **Fires on:** a text node with `px(D.fontSize) < 12`.
- **Detail:** `<size>px`.
- **Cases:** `tiny-text/positive`, `/negative`.
- **Message:** {{msg:tiny-text}}

### `all-caps-body`
{{meta:all-caps-body}}
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** a text node where all three hold:
  - `lower(D.textTransform) = 'uppercase'`;
  - the text is longer than 40 UTF-16 units;
  - `px(D.fontSize)` is null or `< 24`.
- Text typed in capitals without the transform does not count.
- **Detail:** none.
- **Cases:** `all-caps-body/positive`, `/negative`.
- **Message:** {{msg:all-caps-body}}

### `justified-text`
{{meta:justified-text}}
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** a text node with `lower(D.textAlign) = 'justify'`.
- **Detail:** none.
- **Cases:** `justified-text/positive`, `/negative`.
- **Message:** {{msg:justified-text}}

### `hero-eyebrow-chip`
{{meta:hero-eyebrow-chip}}
- **Needs:** **rects**. It needs the artboard's rect and the node's. **Profiles:** active in both.
- **Candidate:** a text node with `fontSizePx ≤ 14` (inherited).
- **The holder:** the parent when the parent has exactly one child, else the node itself.
- **Label tests** (the candidate needs at least one):
  - `pill`: (the holder has an opaque fill, or declares `border` / `borderWidth`) **and** (`px(holder.borderRadius) ≥ 8`, or the radius contains `%`);
  - `tracked`: `trackingEm({ letterSpacing: inherited, fontSize: fontSizePx }) ≥ 0.05`;
  - `caps`: inherited `textTransform` is `uppercase`, or the text equals its upper-case form and contains `[A-Z]`.
- **Headline:**
  - `label` = the holder's rect, or the node's rect when the holder has none;
  - scan the subtree in pre-order for a text node `h ≠ node` with a rect, `fontSizePx(h) ≥ 48`, and `h.y − board.y ≤ 1000`;
  - it qualifies when `0 ≤ h.y − (label.y + label.height) ≤ 40` and the two overlap horizontally (strict);
  - the first qualifying headline wins.
- **Detail:** `pill above "<h.text first 18 units>…"`, or `eyebrow above "…"`. The `…` (U+2026) is added only when the text is longer than 18.
- **Related:** `[h.id]`.
- **Cases:** `hero-eyebrow-chip/positive`, `/negative`, `/pill`.
- **Message:** {{msg:hero-eyebrow-chip}}

## Color & Contrast

### `gradient-text`
{{meta:gradient-text}}
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** any node where `bg` contains `gradient` **and** (`clip` contains `text` **or** `lower(D.color) = 'transparent'`):
  - `bg` = `lower(D.background) + ' ' + lower(D.backgroundImage)`;
  - `clip` = `lower(D.backgroundClip) + ' ' + lower(D.webkitBackgroundClip)`.
- **Detail:** none.
- **Cases:** `gradient-text/positive`, `/negative`.
- **Message:** {{msg:gradient-text}}

### `low-contrast` and `gray-on-color` (one check, at most one finding)
{{meta:low-contrast}}
{{meta:gray-on-color}}
- **Needs:** model. **Profiles:** both are **never relaxed**.
- **Applies to:** a text node with:
  - `fg = parseColor(D.color)`, the **own** colour only;
  - `bgStr`: the first of the node's own, then each ancestor's (nearest first), `style.background ?? style.backgroundColor` that `parseColor` reads.
- Both must exist, else nothing is checked.
- **The check:** `ratio = contrastRatio(fg, bg)`; `large = px(D.fontSize) ≥ 24`.
  - If `isGrayColor(D.color)` **and** `hasChroma(bgStr)` → **`gray-on-color`**.
  - Else if `ratio < (large ? 3 : 4.5)` → **`low-contrast`**.
- **Detail:** `<ratio.toFixed(2)>:1`.
- **Cases:**
  - `low-contrast`: `low-contrast/positive`, `/negative`, `/near-black-on-colour`;
  - `gray-on-color`: `gray-on-color/positive`, `/negative`.
- **Message (`low-contrast`):** {{msg:low-contrast}}
- **Message (`gray-on-color`):** {{msg:gray-on-color}}

## Layout & Space

### `nested-cards`
{{meta:nested-cards}}
- **Needs:** model. **Profiles:** active in both.
- **Card:** a node with:
  - `background` or `backgroundColor` declared (any value);
  - `px(D.borderRadius) ≥ 8`;
  - `px(D.padding) > 0` (the single-value `padding` key only);
  - at least one child.
- **Fires on:** each card in the subtree that has a card **anywhere among its ancestors**. The walk goes beyond the lint target.
- **Detail:** none.
- **Cases:** `nested-cards/positive`, `/negative`, `scope/subtree-target`.
- **Message:** {{msg:nested-cards}}

### `monotonous-spacing`
{{meta:monotonous-spacing}}
- **Needs:** model. **Profiles:** active in both.
- **Fires when:**
  - the values are `round(px(D.gap))` and `round(px(D.padding))` of every subtree node, where `> 0`. Only the `gap` and `padding` keys, single values; no longhands, no margin;
  - there are `≥ 4` of them;
  - the most common value's share is `> 0.6`;
  - there are `≤ 3` distinct values.
- **Attributed to:** the lint target.
- **Detail:** `<dominant>px <round(share×100)>%`, e.g. `16px 100%`.
- **Cases:** `monotonous-spacing/positive`, `/negative`.
- **Message:** {{msg:monotonous-spacing}}

### `cramped-padding`
{{meta:cramped-padding}}
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** a node where:
  - it has children;
  - `background ?? backgroundColor ?? border` is declared;
  - `0 < px(D.padding) < 8`. Only the `padding` key, a single value.
- **Detail:** `<pad>px`.
- **Cases:** `cramped-padding/positive`, `/negative`.
- **Message:** {{msg:cramped-padding}}

### `text-overflow`
{{meta:text-overflow}}
- **Needs:** **rects** (the node's and its parent's). **Profiles:** active in both.
- **Fires on:** **any node** with a parent and both rects, whose own `position` is not `absolute` or `fixed`, where `spill > 1`:
  - `spill = max(parent.x − rect.x, (rect.x+rect.w) − (parent.x+parent.w))`.
  - It is horizontal only, against the parent's border box.
- **Detail:** `spills <round(spill)>px past <parent.name>`.
- **Cases:** `text-overflow/positive`, `/negative`.
- **Message:** {{msg:text-overflow}}

### `line-length`
{{meta:line-length}}
- **Needs:** **rects**. **Profiles:** active in both.
- **Fires on:** a text node with `fontSizePx < 24`, where:
  - `lines = max(1, round(rect.height / lineHeightPx))`;
  - `cpl = text.length / lines`;
  - it fires when `cpl > 80`.
- **Detail:** `~<round(cpl)> chars/line`.
- **Cases:** `line-length/positive`, `/negative`, `/no-rect`.
- **Message:** {{msg:line-length}}

### `body-text-viewport-edge`
{{meta:body-text-viewport-edge}}
- **Needs:** **rects** (the node's and the artboard's). **Profiles:** active in both.
- **Fires on:** a text node, not the artboard itself, with:
  - `fontSizePx < 24`;
  - text length `≥ 40`;
  - `gap < 16`, where `gap = min(rect.x − board.x, board.right − rect.right)`.
- The artboard stands in for the viewport.
- **Detail:** `<max(0, round(gap))>px from the edge`.
- **Cases:** `body-text-viewport-edge/positive`, `/negative`.
- **Message:** {{msg:body-text-viewport-edge}}

### `clipped-overflow-container`
{{meta:clipped-overflow-container}}
- **Needs:** **rects**. **Profiles:** `expressive`: **off**.
- **Fires on:** a **text or image** node (never box or frame) whose rect has `w > 0` and `h > 0`, when `share < 0.6`:
  - `share = visibleArea / rectArea`;
  - share is 0 when the node is clipped away entirely.
- **Visible rect:**
  - the node's rect intersected with the **padding box** of every ancestor that clips;
  - an ancestor clips per axis when its `overflow` (1 or 2 values: `x [y]`) or its `overflowX` / `overflowY` is `hidden`, `clip`, `auto` or `scroll`;
  - padding box = border box minus border widths. If `borderWidth` or any `border<Side>Width` is declared, the widths come from those alone: `borderWidth` holds 1–4 values, each `px` or 0, and `border<Side>Width` overrides its side; missing sides are 0. Otherwise every side takes the first px token of `border`. Finally, each side's `border<Side>` first px token overrides it;
  - ancestors without a rect are skipped.
- **Detail:** `<round(share×100)>% visible`, plus ` in <clip.name>`, where `clip` is the nearest ancestor that clips on any axis.
- **Related:** `[clip.id]` when there is one.
- **Cases:** `clipped-overflow-container/positive`, `/negative`, `profiles/*`.
- **Message:** {{msg:clipped-overflow-container}}

### `text-occluded` (Scrim-native)
{{meta:text-occluded}}
- **Needs:** **rects**. **Profiles:** `expressive`: **info**.
- **Applies to:** a text node with a rect.
  - `ink` = `parseColor(inherited color ?? '#000000')`.
  - Walk `a` = the node, then each ancestor, up to but not including the root. At each step, `p` = a's parent.
  - For every sibling `s ≠ a` in `p.childIds`, first decide whether `s` paints **above** `a` (paint rank below).
- **Gather the sibling's covering rects.** For each node `o` in `s`'s subtree (pre-order) with a rect that overlaps the text rect (strict):
  - `fill` = `o`'s **opaque fill**: `backgroundColor ?? background`, parsed, alpha `≥ 0.9`. An image counts as fill `#000000`.
  - If `s` paints above → add the rect to **over**.
  - Else, if `ink`, `o` is not an image, and `contrastRatio(ink, fill) < 1.5` → add it to **camo**.
- **Score.** For **over** (min share 0.25, word `covered by`), then **camo** (min share 0.1, word `lost against`):
  - share = **coverage** of the text rect by the list;
  - keep it if `share ≥ min` and it beats the best so far (strictly greater).
- **Paint rank** of a sibling at index `i`: `[layer, z, i]`.
  - `z = zIndex` (number, or via `px`);
  - `z < 0` → `[-1, z, i]`;
  - `z > 0` → `[2, z, i]`;
  - else `[positioned ? 1 : 0, 0, i]`, where positioned = `position` set and not `static`, or `transform` declared;
  - compare lexicographically, greater = above.
- **Coverage:** sample a 16 × 8 grid of cell centres, `x = r.x + (i+0.5)/16 × w`, `y = r.y + (j+0.5)/8 × h`. A point is covered when it lies in any list rect, half-open (`x ≥ c.x && x < c.right`, the same in y). Share = hits / 128.
- **Detail:** `<round(share×100)>% covered by <s.name>` or `… lost against <s.name>`.
- **Related:** `[s.id]`.
- **Cases:** `text-occluded/covered` (50%), `/camouflage` (19%: the grid sees 3 of 16 columns), `/negative`, `profiles/expressive-stored`.
- **Message:** {{msg:text-occluded}}

### `root-overlap` (Scrim-native)
{{meta:root-overlap}}
- **Needs:** **rects**, in whole-document runs only. **Profiles:** active in both; **never relaxed**.
- **Check:** `lint.md` §7.4.
- **Detail:** `overlaps "<other.name>" by <W>×<H>`.
- **Related:** `[other]`.
- **Cases:** `root-overlap/positive`, `/negative-touching`, `/subtree-run`, `ignore_lint/related-node`.
- **Message:** {{msg:root-overlap}}

### `size-duplicated` (Scrim-native)
{{meta:size-duplicated}}
- **Needs:** model. **Profiles:** **informational in every profile**.
- **Fires on:** any node holding any of `width, height, minWidth, minHeight, maxWidth, maxHeight, flexBasis` in **both** `layout` and `style`.
- **Detail:** those keys joined `, `, in that order.
- **Cases:** `size-duplicated/positive`, `/negative`.
- **Message:** {{msg:size-duplicated}}

### `style-shorthand-conflict` (Scrim-native)
{{meta:style-shorthand-conflict}}
- **Needs:** model. **Profiles:** **informational in every profile**.
- **The group keys:**
  - combined → longhands: `padding` → `padding{Top,Right,Bottom,Left}`; `margin` → the same four sides;
  - `borderRadius` → `border{TopLeft,TopRight,BottomRight,BottomLeft}Radius`;
  - `gap` → `rowGap, columnGap`;
  - `border` → all 12 `border<Side><Width|Style|Color>`;
  - `border<Width|Style|Color>` → the 4 sides of that part;
  - `border<Side>` → that side's 3 parts;
  - `transition` → `transition{Property,Duration,TimingFunction,Delay,Behavior}`;
  - `animation` → `animation{Name,Duration,TimingFunction,Delay,IterationCount,Direction,FillMode,PlayState}`.
- **The pairs:**
  - list the node's group keys in CSS order: `layout`'s keys in insertion order, then `style`'s. A key in both keeps its first position;
  - for each pair `(a, b)` with `a` before `b`, report it when at least one is a combined key and their longhand sets intersect.
- **Detail:** the pairs as `a + b`, joined `, `, e.g. `padding + paddingTop, border + borderTopWidth`.
- **Cases:** `style-shorthand-conflict/positive`, `/negative`, `side-tab/quirk-borderWidth-group-ignored`.
- **Message:** {{msg:style-shorthand-conflict}}

## Visual Details

### `side-tab` and `border-accent-on-rounded` (one check, at most one finding)
{{meta:side-tab}}
{{meta:border-accent-on-rounded}}
- **Needs:** model.
- **Profiles:**
  - `side-tab`: `expressive` **off**;
  - `border-accent-on-rounded`: active in both.
- **Side parse.** For each side in order Top, Right, Bottom, Left:
  - `width = px(D.border<Side>Width) ?? (sh.width || all.width)`;
  - `color = D.border<Side>Color ?? sh.color ?? all.color`;
  - `sh` = parse of `D.border<Side>`, and `all` = parse of `D.border`. **`borderWidth` / `borderColor` are not read.**
- **The shorthand parse:**
  - split on whitespace;
  - a token that `px` reads sets the width (the last one wins);
  - otherwise a token that `parseColor` reads, or one starting `#` or containing `rgb` or `hsl`, sets the colour (the last one wins).
- **The check:** for the first side with `width ≥ 2`, a colour, and `!isNeutralColor(colour)`:
  - `maxOther` = the max width of the other three;
  - if `maxOther ≤ 1` or `width ≥ 2 × maxOther` → report **`border-accent-on-rounded`** when `px(D.borderRadius) > 0`, else **`side-tab`**, and stop.
- **Detail:** `<width>px <colour token as written>`.
- **Cases:** `side-tab/positive`, `/negative-uniform-border`, `/negative-neutral-colour`, `/quirk-borderWidth-group-ignored`, `border-accent-on-rounded/positive`, `/negative-thin`.
- **Message (`side-tab`):** {{msg:side-tab}}
- **Message (`border-accent-on-rounded`):** {{msg:border-accent-on-rounded}}

### `gpt-thin-border-wide-shadow`
{{meta:gpt-thin-border-wide-shadow}}
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** a node with `0.5 ≤ borderW ≤ 1.5` and `blur ≥ 24`:
  - `borderW = px(D.borderWidth) ?? widthOf(D.border)`, the shorthand parse above;
  - `blur` = the **third** match of `-?\d+(\.\d+)?px` in `lower(D.boxShadow)`, or 0 when there are fewer than 3.
- **Detail:** none.
- **Cases:** `gpt-thin-border-wide-shadow/positive`, `/negative-small-blur`, `/quirk-unitless-zero`.
- **Message:** {{msg:gpt-thin-border-wide-shadow}}

### `repeating-stripes-gradient`
{{meta:repeating-stripes-gradient}}
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** any node whose `bg` (as in `gradient-text`) contains `repeating-linear-gradient` or `repeating-radial-gradient`.
- **Detail:** none.
- **Cases:** `repeating-stripes-gradient/positive`, `/negative`.
- **Message:** {{msg:repeating-stripes-gradient}}

## Copy

### `marketing-buzzword`
{{meta:marketing-buzzword}}
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** a text node whose lower-cased text contains one of these phrases.
  - **The phrases, in order:** `streamline your, empower your, supercharge your, unleash your, unleash the power, leverage the power, built for the modern, trusted by leading, trusted by the world, best-in-class, industry-leading, world-class, enterprise-grade, next-generation, cutting-edge, transform your business, revolutionize, game-changer, game changing, mission-critical, best of breed, future-proof, future proof, seamless experience, seamlessly integrate, drive engagement, drive growth, drive results, harness the power`.
  - The **first phrase in list order** is reported, not the first in the text.
- **Detail:** `"<phrase>"`, with the quotes.
- **Cases:** `marketing-buzzword/positive`, `/negative`.
- **Message:** {{msg:marketing-buzzword}}

### `em-dash-overuse`
{{meta:em-dash-overuse}}
- **Needs:** model. **Profiles:** active in both.
- **Fires when:**
  - the text is every text node's text in the subtree, joined with two spaces;
  - `count` = matches of `—|--(?=\S)` (an em dash, or two hyphens followed by a non-space);
  - it fires when `count ≥ 5`.
- **Attributed to:** the lint target.
- **Detail:** `<count> em-dashes`.
- **Cases:** `em-dash-overuse/positive`, `/negative`.
- **Message:** {{msg:em-dash-overuse}}

## Layout & Space (copy-based)

### `numbered-section-markers`
{{meta:numbered-section-markers}}
- **Needs:** model. **Profiles:** active in both.
- **Fires when:**
  - the markers are the distinct matches of `\b(0[1-9]|1[0-2])\b` in the joined subtree text, sorted as strings;
  - it fires with `≥ 3` markers of which `≥ 2` adjacent pairs are consecutive numbers.
- **Attributed to:** the lint target.
- **Detail:** the first 6 markers joined `, `.
- **Cases:** `numbered-section-markers/positive`, `/negative`.
- **Message:** {{msg:numbered-section-markers}}

## Motion

These rules read **every declaration map** of a node:
- `layout` and `style` merged (style on top);
- each `states[<state>]` map;
- each `media[<query>].style`;
- each `media[<query>].states[<state>]`.

Detail prefixes: none for the base map, `<state>: ` for a state, `<query>: ` for a media style, `<query> <state>: ` for a media state. Duplicate entries are dropped, and the entries are joined `; `.

### `bounce-easing`
{{meta:bounce-easing}}
- **Needs:** model. **Profiles:** active in both.
- **Easings of a map:**
  1. Expand the `transition` and `animation` shorthands into longhands. Items are comma-separated outside parentheses. A shorthand that cannot be parsed (an unknown token, `var()`) is kept as written.
  2. Take each comma item of `transitionTimingFunction` and `animationTimingFunction`.
  3. Add the raw `transition` / `animation` value when it survived expansion.
- **An easing bounces when any of these holds:**
  - it matches `/bounce|elastic/i`;
  - it is a `cubic-bezier(x1, y1, x2, y2)` with 4 finite numbers where `y1` or `y2` lies outside `[0, 1]`;
  - it is a `linear(…)` whose stop outputs (the first number of each comma stop) are all finite and some output is lower than the one before.
- **Detail:** the matching easings with their prefixes.
- **Cases:** `bounce-easing/positive`, `/negative`.
- **Message:** {{msg:bounce-easing}}

### `layout-transition`
{{meta:layout-transition}}
- **Needs:** model. **Profiles:** active in both.
- **Check:**
  - per map, expand as above and take the comma items of `transitionProperty`, trimmed and lower-cased;
  - report `all`, or any layout property;
  - a transition shorthand with no property expands to `all`.
- **Layout properties:** `width height min-width min-height max-width max-height padding padding-top padding-right padding-bottom padding-left margin margin-top margin-right margin-bottom margin-left gap row-gap column-gap top right bottom left inset flex flex-grow flex-shrink flex-basis grid-template-columns grid-template-rows grid-template font-size line-height letter-spacing border-width border-top-width border-right-width border-bottom-width border-left-width`.
- **Detail entries:** the property, or `transition-property: all (includes layout properties)`, with the map prefix.
- **Cases:** `layout-transition/positive`, `/negative`.
- **Message:** {{msg:layout-transition}}

### `image-hover-transform`
{{meta:image-hover-transform}}
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** an **image** node whose `states.hover` or `states.active`, or the same state inside any media block, sets `transform`, `scale` or `rotate` to anything other than `none`.
- **Detail entries:** `hover: <key> <value>`, or `pressed: <key> <value>` for `active`, joined `; `.
- **Cases:** `image-hover-transform/positive`, `/negative`.
- **Message:** {{msg:image-hover-transform}}

## The 11 catalog rules that never fire

They are known to `ignore_lint` and carry the catalog's metadata, but no check exists (`lint.md` §2). Their names and messages, for completeness:

{{unported}}
