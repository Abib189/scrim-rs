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
- **`overused-font`**: category **slop**; domain **Typography**; name: Overused font
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** **any node** (not only text) where `primaryFont(D.fontFamily)` is in this list: `inter, roboto, open sans, lato, montserrat, arial, helvetica, fraunces, instrument sans, instrument serif, geist, geist sans, geist mono, mona sans, plus jakarta sans, space grotesk, recoleta`.
- **Detail:** the primary font, lower-cased (`inter`).
- **Cases:** `overused-font/positive`, `overused-font/negative`.
- **Message:** `Inter, Roboto, Fraunces, Geist, Plus Jakarta Sans, and Space Grotesk are used on so many sites they no longer feel distinctive. Each new wave of AI-generated UIs converges on the same handful of faces. Choose a face that gives your interface personality.`

### `single-font`
- **`single-font`**: category **slop**; domain **Typography**; name: Single font for everything
- **Needs:** model. **Profiles:** active in both.
- **Fires when:**
  - `F` = the set of `primaryFont(D.fontFamily)` over **every node** of the subtree (frames included), nulls dropped;
  - it fires when `|F| = 1` and the subtree has `≥ 2` text nodes (`isText`).
- **Attributed to:** the lint target.
- **Detail:** the one font.
- **Cases:** `single-font/positive`, `single-font/negative`, `scope/subtree-target`.
- **Message:** `Only one font family is used for the entire page. Pair a distinctive display font with a refined body font to create typographic hierarchy.`

### `flat-type-hierarchy`
- **`flat-type-hierarchy`**: category **slop**; domain **Typography**; name: Flat type hierarchy
- **Needs:** model. **Profiles:** active in both.
- **Fires when:**
  - `S` = `px(D.fontSize)` of each text node with a px size (own value only);
  - it fires when there are `≥ 3` text nodes, `|S| ≥ 2`, `min > 0`, and `max / min < 1.25`.
- **Attributed to:** the lint target.
- **Detail:** `<min>–<max>px` (en dash), e.g. `16–18px`.
- **Cases:** `flat-type-hierarchy/positive`, `/negative`.
- **Message:** `Font sizes are too close together — no clear visual hierarchy. Use fewer sizes with more contrast (aim for at least a 1.25 ratio between steps).`

### `oversized-h1`
- **`oversized-h1`**: category **slop**; domain **Typography**; name: Oversized hero headline
- **Needs:** model. **Profiles:** `expressive`: **off**.
- **Fires on:** any text node with `px(D.fontSize) ≥ 72`. There is no heading or length test.
- **Detail:** `<size>px`.
- **Cases:** `oversized-h1/positive`, `/negative`, `profiles/*`.
- **Message:** `A full-sentence headline set at display size ends up dominating the viewport, leaving no room for anything else above the fold. A punchy one- or two-word headline at that size is fine — the problem is a long headline blown up too large. Set long headlines smaller, or tighten the copy.`

### `extreme-negative-tracking`
- **`extreme-negative-tracking`**: category **slop**; domain **Typography**; name: Crushed letter spacing
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** a text node with `trackingEm(D) ≤ −0.04`.
- **Detail:** `<em.toFixed(3)>em`, e.g. `-0.050em`, `-0.063em` (from −3px at 48px).
- **Cases:** `extreme-negative-tracking/positive`, `/negative`.
- **Message:** `Letter-spacing pulled tighter than the point where characters keep their own shapes costs legibility. Tighten display type optically, not destructively.`

### `wide-tracking`
- **`wide-tracking`**: category **quality**; domain **Typography** (catalog skillSection: none; fallback domain); name: Wide letter spacing on body text
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** a text node with `trackingEm(D) ≥ 0.05` and (`px(D.fontSize)` is null or `< 24`).
  - 24 = WCAG large text, `18 × 96/72`.
- **Detail:** `<em.toFixed(3)>em`.
- **Cases:** `wide-tracking/positive`, `/negative`.
- **Message:** `Letter spacing above 0.05em on body text disrupts natural character groupings and slows reading. Reserve wide tracking for short uppercase labels only.`

### `tiny-text`
- **`tiny-text`**: category **quality**; domain **Typography** (catalog skillSection: none; fallback domain); name: Tiny body text
- **Needs:** model. **Profiles:** active in both; **never relaxed**.
- **Fires on:** a text node with `px(D.fontSize) < 12`.
- **Detail:** `<size>px`.
- **Cases:** `tiny-text/positive`, `/negative`.
- **Message:** `Body text below 12px is hard to read, especially on high-DPI screens. Use at least 14px for body content, 16px is ideal.`

### `all-caps-body`
- **`all-caps-body`**: category **quality**; domain **Typography**; name: All-caps body text
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** a text node where all three hold:
  - `lower(D.textTransform) = 'uppercase'`;
  - the text is longer than 40 UTF-16 units;
  - `px(D.fontSize)` is null or `< 24`.
- Text typed in capitals without the transform does not count.
- **Detail:** none.
- **Cases:** `all-caps-body/positive`, `/negative`.
- **Message:** `Long passages in uppercase are hard to read. We recognize words by shape (ascenders and descenders), which all-caps removes. Reserve uppercase for short labels and headings.`

### `justified-text`
- **`justified-text`**: category **quality**; domain **Typography** (catalog skillSection: none; fallback domain); name: Justified text
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** a text node with `lower(D.textAlign) = 'justify'`.
- **Detail:** none.
- **Cases:** `justified-text/positive`, `/negative`.
- **Message:** `Justified text without hyphenation creates uneven word spacing ("rivers of white"). Use text-align: left for body text, or enable hyphens: auto if you must justify.`

### `hero-eyebrow-chip`
- **`hero-eyebrow-chip`**: category **slop**; domain **Typography**; name: Hero eyebrow / pill chip
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
- **Message:** `A tiny uppercase letter-spaced label sitting immediately above an oversized hero headline — or the same shape rendered as a pill chip — is now the default AI SaaS hero. Drop the eyebrow, integrate the kicker into the headline, or run it as a navigation breadcrumb instead.`

## Color & Contrast

### `gradient-text`
- **`gradient-text`**: category **slop**; domain **Color & Contrast**; name: Gradient text
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** any node where `bg` contains `gradient` **and** (`clip` contains `text` **or** `lower(D.color) = 'transparent'`):
  - `bg` = `lower(D.background) + ' ' + lower(D.backgroundImage)`;
  - `clip` = `lower(D.backgroundClip) + ' ' + lower(D.webkitBackgroundClip)`.
- **Detail:** none.
- **Cases:** `gradient-text/positive`, `/negative`.
- **Message:** `Gradient text is decorative rather than meaningful — a common AI tell, especially on headings and metrics. Use solid colors for text.`

### `low-contrast` and `gray-on-color` (one check, at most one finding)
- **`low-contrast`**: category **quality**; domain **Color & Contrast** (catalog skillSection: none; fallback domain); name: Low contrast text
- **`gray-on-color`**: category **quality**; domain **Color & Contrast**; name: Gray text on colored background
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
- **Message (`low-contrast`):** `Text does not meet WCAG AA contrast requirements (4.5:1 for body, 3:1 for large text). Increase the contrast between text and background.`
- **Message (`gray-on-color`):** `Gray text looks washed out on colored backgrounds. Use a darker shade of the background color instead, or white/near-white for contrast.`

## Layout & Space

### `nested-cards`
- **`nested-cards`**: category **slop**; domain **Layout & Space**; name: Nested cards
- **Needs:** model. **Profiles:** active in both.
- **Card:** a node with:
  - `background` or `backgroundColor` declared (any value);
  - `px(D.borderRadius) ≥ 8`;
  - `px(D.padding) > 0` (the single-value `padding` key only);
  - at least one child.
- **Fires on:** each card in the subtree that has a card **anywhere among its ancestors**. The walk goes beyond the lint target.
- **Detail:** none.
- **Cases:** `nested-cards/positive`, `/negative`, `scope/subtree-target`.
- **Message:** `Cards inside cards create visual noise and excessive depth. Flatten the hierarchy — use spacing, typography, and dividers instead of nesting containers.`

### `monotonous-spacing`
- **`monotonous-spacing`**: category **slop**; domain **Layout & Space**; name: Monotonous spacing
- **Needs:** model. **Profiles:** active in both.
- **Fires when:**
  - the values are `round(px(D.gap))` and `round(px(D.padding))` of every subtree node, where `> 0`. Only the `gap` and `padding` keys, single values; no longhands, no margin;
  - there are `≥ 4` of them;
  - the most common value's share is `> 0.6`;
  - there are `≤ 3` distinct values.
- **Attributed to:** the lint target.
- **Detail:** `<dominant>px <round(share×100)>%`, e.g. `16px 100%`.
- **Cases:** `monotonous-spacing/positive`, `/negative`.
- **Message:** `The same spacing value used everywhere — no rhythm, no variation. Use tight groupings for related items and generous separations between sections.`

### `cramped-padding`
- **`cramped-padding`**: category **quality**; domain **Layout & Space**; name: Cramped padding
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** a node where:
  - it has children;
  - `background ?? backgroundColor ?? border` is declared;
  - `0 < px(D.padding) < 8`. Only the `padding` key, a single value.
- **Detail:** `<pad>px`.
- **Cases:** `cramped-padding/positive`, `/negative`.
- **Message:** `Text is too close to the edge of its container. Two shapes: (1) an element with its own text where the padding is too low for the font size, and (2) a wrapper with text-bearing children and near-zero padding against a visible boundary (border, outline, or non-transparent background) — children land flush against the boundary line. Add at least 8px (ideally 12–16px) of padding inside bordered, outlined, or colored containers.`

### `text-overflow`
- **`text-overflow`**: category **quality**; domain **Layout & Space**; name: Content overflowing its container
- **Needs:** **rects** (the node's and its parent's). **Profiles:** active in both.
- **Fires on:** **any node** with a parent and both rects, whose own `position` is not `absolute` or `fixed`, where `spill > 1`:
  - `spill = max(parent.x − rect.x, (rect.x+rect.w) − (parent.x+parent.w))`.
  - It is horizontal only, against the parent's border box.
- **Detail:** `spills <round(spill)>px past <parent.name>`.
- **Cases:** `text-overflow/positive`, `/negative`.
- **Message:** `Content renders wider than its container, spilling out or forcing a horizontal scrollbar. Let text wrap, constrain widths, or give the region a deliberate scroll affordance.`

### `line-length`
- **`line-length`**: category **quality**; domain **Layout & Space**; name: Line length too long
- **Needs:** **rects**. **Profiles:** active in both.
- **Fires on:** a text node with `fontSizePx < 24`, where:
  - `lines = max(1, round(rect.height / lineHeightPx))`;
  - `cpl = text.length / lines`;
  - it fires when `cpl > 80`.
- **Detail:** `~<round(cpl)> chars/line`.
- **Cases:** `line-length/positive`, `/negative`, `/no-rect`.
- **Message:** `Text lines wider than ~80 characters are hard to read. The eye loses its place tracking back to the start of the next line. Add a max-width (65ch to 75ch) to text containers.`

### `body-text-viewport-edge`
- **`body-text-viewport-edge`**: category **quality**; domain **Layout & Space** (catalog skillSection: none; fallback domain); name: Body text touching viewport edge
- **Needs:** **rects** (the node's and the artboard's). **Profiles:** active in both.
- **Fires on:** a text node, not the artboard itself, with:
  - `fontSizePx < 24`;
  - text length `≥ 40`;
  - `gap < 16`, where `gap = min(rect.x − board.x, board.right − rect.right)`.
- The artboard stands in for the viewport.
- **Detail:** `<max(0, round(gap))>px from the edge`.
- **Cases:** `body-text-viewport-edge/positive`, `/negative`.
- **Message:** `Body paragraphs render flush against the left or right viewport edge with no container providing horizontal padding. Wrap content in a container with at least 16px (ideally 24-32px) of horizontal padding, or apply max-width with mx-auto.`

### `clipped-overflow-container`
- **`clipped-overflow-container`**: category **quality**; domain **Layout & Space**; name: Positioned child clipped by overflow container
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
- **Message:** `A clipping container (overflow hidden or clip) wrapping an absolutely-positioned child cuts off tooltips, menus, and popovers that need to escape. Let the overflow be visible, or move the positioned layer out of the clip.`

### `text-occluded` (Scrim-native)
- **`text-occluded`**: category **quality**; domain **Layout & Space**; name: Text hidden by an overlapping shape
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
- **Message:** `Part of this text is covered by a shape painted over it, or sits on a shape filled in its own colour, so the words disappear. Move the shape, change its paint order, or recolour one of them.`

### `root-overlap` (Scrim-native)
- **`root-overlap`**: category **quality**; domain **Layout & Space**; name: Overlapping artboards
- **Needs:** **rects**, in whole-document runs only. **Profiles:** active in both; **never relaxed**.
- **Check:** `lint.md` §7.4.
- **Detail:** `overlaps "<other.name>" by <W>×<H>`.
- **Related:** `[other]`.
- **Cases:** `root-overlap/positive`, `/negative-touching`, `/subtree-run`, `ignore_lint/related-node`.
- **Message:** `Two artboards overlap on the canvas, so one hides part of the other. Move them apart (Tidy up, ⌥⌘T, or the tidy_roots tool) or place new roots relative to measured neighbours.`

### `size-duplicated` (Scrim-native)
- **`size-duplicated`**: category **quality**; domain **Layout & Space**; name: Size stored twice
- **Needs:** model. **Profiles:** **informational in every profile**.
- **Fires on:** any node holding any of `width, height, minWidth, minHeight, maxWidth, maxHeight, flexBasis` in **both** `layout` and `style`.
- **Detail:** those keys joined `, `, in that order.
- **Cases:** `size-duplicated/positive`, `/negative`.
- **Message:** `This layer declares its size in both layout and style. Style is the one that renders, so the layout copy is dead and editing it does nothing. Any size edit (W / H, a resize, update_node) removes the stale copy.`

### `style-shorthand-conflict` (Scrim-native)
- **`style-shorthand-conflict`**: category **quality**; domain **Layout & Space**; name: Shorthand beside its longhands
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
- **Message:** `This layer stores a combined property next to a per-side value of the same group (padding and paddingTop, for example). CSS lets the later one win, so the canvas and export agree, but the value lives in two places. Editing the group in the inspector, or with update_node, rewrites it in one form.`

## Visual Details

### `side-tab` and `border-accent-on-rounded` (one check, at most one finding)
- **`side-tab`**: category **slop**; domain **Visual Details**; name: Side-tab accent border
- **`border-accent-on-rounded`**: category **slop**; domain **Visual Details**; name: Border accent on rounded element
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
- **Message (`side-tab`):** `Thick colored border on one side of a card — the most recognizable tell of AI-generated UIs. Use a subtler accent or remove it entirely.`
- **Message (`border-accent-on-rounded`):** `Thick accent border on a rounded card — the border clashes with the rounded corners. Remove the border or the border-radius.`

### `gpt-thin-border-wide-shadow`
- **`gpt-thin-border-wide-shadow`**: category **slop**; domain **Visual Details**; name: Hairline border with wide shadow
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** a node with `0.5 ≤ borderW ≤ 1.5` and `blur ≥ 24`:
  - `borderW = px(D.borderWidth) ?? widthOf(D.border)`, the shorthand parse above;
  - `blur` = the **third** match of `-?\d+(\.\d+)?px` in `lower(D.boxShadow)`, or 0 when there are fewer than 3.
- **Detail:** none.
- **Cases:** `gpt-thin-border-wide-shadow/positive`, `/negative-small-blur`, `/quirk-unitless-zero`.
- **Message:** `A hairline border paired with a wide, diffuse shadow is a recurring generated-UI signature. Commit to one — a defined edge or a soft elevation — rather than both at once.`

### `repeating-stripes-gradient`
- **`repeating-stripes-gradient`**: category **slop**; domain **Visual Details**; name: Repeating-gradient stripes
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** any node whose `bg` (as in `gradient-text`) contains `repeating-linear-gradient` or `repeating-radial-gradient`.
- **Detail:** none.
- **Cases:** `repeating-stripes-gradient/positive`, `/negative`.
- **Message:** `Repeating-gradient stripes used as surface decoration are a recurring generated-UI signature. Reach for a deliberate texture or leave the surface plain.`

## Copy

### `marketing-buzzword`
- **`marketing-buzzword`**: category **slop**; domain **Copy**; name: Marketing buzzword
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** a text node whose lower-cased text contains one of these phrases.
  - **The phrases, in order:** `streamline your, empower your, supercharge your, unleash your, unleash the power, leverage the power, built for the modern, trusted by leading, trusted by the world, best-in-class, industry-leading, world-class, enterprise-grade, next-generation, cutting-edge, transform your business, revolutionize, game-changer, game changing, mission-critical, best of breed, future-proof, future proof, seamless experience, seamlessly integrate, drive engagement, drive growth, drive results, harness the power`.
  - The **first phrase in list order** is reported, not the first in the text.
- **Detail:** `"<phrase>"`, with the quotes.
- **Cases:** `marketing-buzzword/positive`, `/negative`.
- **Message:** `Generic SaaS phrases (streamline / empower / supercharge / world-class / enterprise-grade / next-generation / cutting-edge / etc) are instant AI tells. Pick a specific verb and noun that says what the product literally does.`

### `em-dash-overuse`
- **`em-dash-overuse`**: category **slop**; domain **Copy**; name: Em-dash overuse
- **Needs:** model. **Profiles:** active in both.
- **Fires when:**
  - the text is every text node's text in the subtree, joined with two spaces;
  - `count` = matches of `—|--(?=\S)` (an em dash, or two hyphens followed by a non-space);
  - it fires when `count ≥ 5`.
- **Attributed to:** the lint target.
- **Detail:** `<count> em-dashes`.
- **Cases:** `em-dash-overuse/positive`, `/negative`.
- **Message:** `More than two em-dashes (— or --) in body copy is an AI cadence tell. Use commas, colons, periods, or parentheses instead.`

## Layout & Space (copy-based)

### `numbered-section-markers`
- **`numbered-section-markers`**: category **slop**; domain **Layout & Space**; name: Numbered section markers (01 / 02 / 03)
- **Needs:** model. **Profiles:** active in both.
- **Fires when:**
  - the markers are the distinct matches of `\b(0[1-9]|1[0-2])\b` in the joined subtree text, sorted as strings;
  - it fires with `≥ 3` markers of which `≥ 2` adjacent pairs are consecutive numbers.
- **Attributed to:** the lint target.
- **Detail:** the first 6 markers joined `, `.
- **Cases:** `numbered-section-markers/positive`, `/negative`.
- **Message:** `Numbered display markers as section labels (01, 02, 03) are the AI editorial scaffold one tier deeper than tracked eyebrow chips. If you find yourself reaching for them, choose a different section cadence.`

## Motion

These rules read **every declaration map** of a node:
- `layout` and `style` merged (style on top);
- each `states[<state>]` map;
- each `media[<query>].style`;
- each `media[<query>].states[<state>]`.

Detail prefixes: none for the base map, `<state>: ` for a state, `<query>: ` for a media style, `<query> <state>: ` for a media state. Duplicate entries are dropped, and the entries are joined `; `.

### `bounce-easing`
- **`bounce-easing`**: category **slop**; domain **Motion**; name: Bounce or elastic easing
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
- **Message:** `Bounce and elastic easing feel dated and tacky. Real objects decelerate smoothly — use exponential easing (ease-out-quart/quint/expo) instead.`

### `layout-transition`
- **`layout-transition`**: category **quality**; domain **Motion**; name: Layout property animation
- **Needs:** model. **Profiles:** active in both.
- **Check:**
  - per map, expand as above and take the comma items of `transitionProperty`, trimmed and lower-cased;
  - report `all`, or any layout property;
  - a transition shorthand with no property expands to `all`.
- **Layout properties:** `width height min-width min-height max-width max-height padding padding-top padding-right padding-bottom padding-left margin margin-top margin-right margin-bottom margin-left gap row-gap column-gap top right bottom left inset flex flex-grow flex-shrink flex-basis grid-template-columns grid-template-rows grid-template font-size line-height letter-spacing border-width border-top-width border-right-width border-bottom-width border-left-width`.
- **Detail entries:** the property, or `transition-property: all (includes layout properties)`, with the map prefix.
- **Cases:** `layout-transition/positive`, `/negative`.
- **Message:** `Animating width, height, padding, or margin causes layout thrash and janky performance. Use transform and opacity instead, or grid-template-rows for height animations.`

### `image-hover-transform`
- **`image-hover-transform`**: category **slop**; domain **Motion**; name: Image hover transform
- **Needs:** model. **Profiles:** active in both.
- **Fires on:** an **image** node whose `states.hover` or `states.active`, or the same state inside any media block, sets `transform`, `scale` or `rotate` to anything other than `none`.
- **Detail entries:** `hover: <key> <value>`, or `pressed: <key> <value>` for `active`, joined `; `.
- **Cases:** `image-hover-transform/positive`, `/negative`.
- **Message:** `Scaling or rotating an image on hover is a recurring generated-UI signature. Let imagery sit still, or use a subtler, purposeful interaction.`

## The 11 catalog rules that never fire

They are known to `ignore_lint` and carry the catalog's metadata, but no check exists (`lint.md` §2). Their names and messages, for completeness:

- **`ai-color-palette`**: category **slop**; domain **Color & Contrast**; name: AI color palette. Message: `Purple/violet gradients and cyan-on-dark are the most recognizable tells of AI-generated UIs. Choose a distinctive, intentional palette.`
- **`cream-palette`**: category **slop**; domain **Color & Contrast**; name: Cream / beige palette. Message: `A warm cream or beige page background has become the default "tasteful" AI surface, reached for by reflex. Choose a background that comes from a deliberate palette, not the safe warm off-white.`
- **`dark-glow`**: category **slop**; domain **Color & Contrast**; name: Dark mode with glowing accents. Message: `Dark backgrounds with colored box-shadow glows are the default "cool" look of AI-generated UIs. Use subtle, purposeful lighting instead — or skip the dark theme entirely.`
- **`icon-tile-stack`**: category **slop**; domain **Typography**; name: Icon tile stacked above heading. Message: `A small rounded-square icon container above a heading is the universal AI feature-card template — every generator outputs this exact shape. Try a side-by-side icon and heading, or let the icon sit in flow without its own container.`
- **`italic-serif-display`**: category **slop**; domain **Typography**; name: Italic serif display headline. Message: `Oversized italic serif (Fraunces, Recoleta, Playfair, Newsreader-italic) as the primary hero headline reads as taste in isolation but has become the universal AI-startup landing page hero. Set roman, or move to a non-serif display face. Editorial / magazine register may legitimately want this — judge by context.`
- **`repeated-section-kickers`**: category **slop**; domain **Typography**; name: Repeated section kicker labels. Message: `Repeating tiny uppercase tracked labels above section headings turns a brand page into AI editorial scaffolding. Replace them with stronger structure, artifacts, imagery, or a deliberate brand system.`
- **`aphoristic-cadence`**: category **slop**; domain **Copy**; name: Aphoristic-cadence copy. Message: `Three or more sections landing on a short rebuttal sentence ("X. No Y." / "X. Just Y.") or a manufactured-contrast aphorism ("Not a feature. A platform.") reads as AI cadence, not voice. Once is fine; the pattern is the tell.`
- **`broken-image`**: category **quality**; domain **Imagery**; name: Broken or placeholder image. Message: `<img> tags with empty src, missing src, or placeholder values ship as broken-image boxes. Use real images, generated assets, or remove the tag.`
- **`tight-leading`**: category **quality**; domain **Typography** (catalog skillSection: none; fallback domain); name: Tight line height. Message: `Line height below 1.3x the font size makes multi-line text hard to read. Use 1.5 to 1.7 for body text so lines have room to breathe.`
- **`skipped-heading`**: category **quality**; domain **Typography** (catalog skillSection: none; fallback domain); name: Skipped heading level. Message: `Heading levels should not skip (e.g. h1 then h3 with no h2). Screen readers use heading hierarchy for navigation. Skipping levels breaks the document outline.`
- **`theater-slop-phrase`**: category **slop**; domain **Copy**; name: Theater framing copy. Message: `Dismissing something as "theater" is a recurring generated-copy tic. Say plainly what the thing does or does not do.`
