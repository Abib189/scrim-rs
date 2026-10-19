//! The rule catalog: every rule's id, category, domain, name and message, in registry order
//! (`docs/scrim-transfer/specs/lint.md` §2, `lint-rules.md`).
//!
//! The names and messages of the 41 catalog rules are Impeccable's
//! (github.com/pbakaus/impeccable v2.3.2, commit 1863a44b23bfcf28e5990c0068a0603995ab1e29),
//! Copyright 2025 Paul Bakaus, licensed under the Apache License 2.0 (`design-rules/LICENSE`).
//! Modified (Apache-2.0 §4(b)): carried into Rust from Scrim's port, which redefined
//! `gray-on-color`, added profiles, ported the measured-rect and motion rules and dropped the
//! catalog's `severity`; see `design-rules/NOTICE.md`. The last four rules are Scrim's own.

/// `slop`: a tell of generated UI; `quality`: a defect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Slop,
    Quality,
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Slop => "slop",
            Category::Quality => "quality",
        }
    }
}

#[derive(Debug)]
pub struct Rule {
    pub id: &'static str,
    pub category: Category,
    pub domain: &'static str,
    pub name: &'static str,
    pub message: &'static str,
    /// Whether a check reports it: 11 catalog rules are known (`ignore_lint` takes them) but
    /// never reported.
    pub reported: bool,
}

/// The rule with this id. Every id the checks use is in the table (a test checks it).
pub fn rule(id: &str) -> &'static Rule {
    RULES.iter().find(|r| r.id == id).unwrap_or(&RULES[0])
}

/// Whether `ignore_lint` accepts `id`.
pub fn known(id: &str) -> bool {
    RULES.iter().any(|r| r.id == id)
}

/// The ids in registry order: the "Known rules" list.
pub fn ids() -> Vec<&'static str> {
    RULES.iter().map(|r| r.id).collect()
}

pub static RULES: &[Rule] = &[
    Rule {
        id: "side-tab",
        category: Category::Slop,
        domain: "Visual Details",
        name: "Side-tab accent border",
        message: "Thick colored border on one side of a card — the most recognizable tell of AI-generated UIs. Use a subtler accent or remove it entirely.",
        reported: true,
    },
    Rule {
        id: "border-accent-on-rounded",
        category: Category::Slop,
        domain: "Visual Details",
        name: "Border accent on rounded element",
        message: "Thick accent border on a rounded card — the border clashes with the rounded corners. Remove the border or the border-radius.",
        reported: true,
    },
    Rule {
        id: "overused-font",
        category: Category::Slop,
        domain: "Typography",
        name: "Overused font",
        message: "Inter, Roboto, Fraunces, Geist, Plus Jakarta Sans, and Space Grotesk are used on so many sites they no longer feel distinctive. Each new wave of AI-generated UIs converges on the same handful of faces. Choose a face that gives your interface personality.",
        reported: true,
    },
    Rule {
        id: "single-font",
        category: Category::Slop,
        domain: "Typography",
        name: "Single font for everything",
        message: "Only one font family is used for the entire page. Pair a distinctive display font with a refined body font to create typographic hierarchy.",
        reported: true,
    },
    Rule {
        id: "flat-type-hierarchy",
        category: Category::Slop,
        domain: "Typography",
        name: "Flat type hierarchy",
        message: "Font sizes are too close together — no clear visual hierarchy. Use fewer sizes with more contrast (aim for at least a 1.25 ratio between steps).",
        reported: true,
    },
    Rule {
        id: "gradient-text",
        category: Category::Slop,
        domain: "Color & Contrast",
        name: "Gradient text",
        message: "Gradient text is decorative rather than meaningful — a common AI tell, especially on headings and metrics. Use solid colors for text.",
        reported: true,
    },
    Rule {
        id: "ai-color-palette",
        category: Category::Slop,
        domain: "Color & Contrast",
        name: "AI color palette",
        message: "Purple/violet gradients and cyan-on-dark are the most recognizable tells of AI-generated UIs. Choose a distinctive, intentional palette.",
        reported: false,
    },
    Rule {
        id: "cream-palette",
        category: Category::Slop,
        domain: "Color & Contrast",
        name: "Cream / beige palette",
        message: "A warm cream or beige page background has become the default \"tasteful\" AI surface, reached for by reflex. Choose a background that comes from a deliberate palette, not the safe warm off-white.",
        reported: false,
    },
    Rule {
        id: "nested-cards",
        category: Category::Slop,
        domain: "Layout & Space",
        name: "Nested cards",
        message: "Cards inside cards create visual noise and excessive depth. Flatten the hierarchy — use spacing, typography, and dividers instead of nesting containers.",
        reported: true,
    },
    Rule {
        id: "monotonous-spacing",
        category: Category::Slop,
        domain: "Layout & Space",
        name: "Monotonous spacing",
        message: "The same spacing value used everywhere — no rhythm, no variation. Use tight groupings for related items and generous separations between sections.",
        reported: true,
    },
    Rule {
        id: "bounce-easing",
        category: Category::Slop,
        domain: "Motion",
        name: "Bounce or elastic easing",
        message: "Bounce and elastic easing feel dated and tacky. Real objects decelerate smoothly — use exponential easing (ease-out-quart/quint/expo) instead.",
        reported: true,
    },
    Rule {
        id: "dark-glow",
        category: Category::Slop,
        domain: "Color & Contrast",
        name: "Dark mode with glowing accents",
        message: "Dark backgrounds with colored box-shadow glows are the default \"cool\" look of AI-generated UIs. Use subtle, purposeful lighting instead — or skip the dark theme entirely.",
        reported: false,
    },
    Rule {
        id: "icon-tile-stack",
        category: Category::Slop,
        domain: "Typography",
        name: "Icon tile stacked above heading",
        message: "A small rounded-square icon container above a heading is the universal AI feature-card template — every generator outputs this exact shape. Try a side-by-side icon and heading, or let the icon sit in flow without its own container.",
        reported: false,
    },
    Rule {
        id: "italic-serif-display",
        category: Category::Slop,
        domain: "Typography",
        name: "Italic serif display headline",
        message: "Oversized italic serif (Fraunces, Recoleta, Playfair, Newsreader-italic) as the primary hero headline reads as taste in isolation but has become the universal AI-startup landing page hero. Set roman, or move to a non-serif display face. Editorial / magazine register may legitimately want this — judge by context.",
        reported: false,
    },
    Rule {
        id: "hero-eyebrow-chip",
        category: Category::Slop,
        domain: "Typography",
        name: "Hero eyebrow / pill chip",
        message: "A tiny uppercase letter-spaced label sitting immediately above an oversized hero headline — or the same shape rendered as a pill chip — is now the default AI SaaS hero. Drop the eyebrow, integrate the kicker into the headline, or run it as a navigation breadcrumb instead.",
        reported: true,
    },
    Rule {
        id: "repeated-section-kickers",
        category: Category::Slop,
        domain: "Typography",
        name: "Repeated section kicker labels",
        message: "Repeating tiny uppercase tracked labels above section headings turns a brand page into AI editorial scaffolding. Replace them with stronger structure, artifacts, imagery, or a deliberate brand system.",
        reported: false,
    },
    Rule {
        id: "numbered-section-markers",
        category: Category::Slop,
        domain: "Layout & Space",
        name: "Numbered section markers (01 / 02 / 03)",
        message: "Numbered display markers as section labels (01, 02, 03) are the AI editorial scaffold one tier deeper than tracked eyebrow chips. If you find yourself reaching for them, choose a different section cadence.",
        reported: true,
    },
    Rule {
        id: "em-dash-overuse",
        category: Category::Slop,
        domain: "Copy",
        name: "Em-dash overuse",
        message: "More than two em-dashes (— or --) in body copy is an AI cadence tell. Use commas, colons, periods, or parentheses instead.",
        reported: true,
    },
    Rule {
        id: "marketing-buzzword",
        category: Category::Slop,
        domain: "Copy",
        name: "Marketing buzzword",
        message: "Generic SaaS phrases (streamline / empower / supercharge / world-class / enterprise-grade / next-generation / cutting-edge / etc) are instant AI tells. Pick a specific verb and noun that says what the product literally does.",
        reported: true,
    },
    Rule {
        id: "aphoristic-cadence",
        category: Category::Slop,
        domain: "Copy",
        name: "Aphoristic-cadence copy",
        message: "Three or more sections landing on a short rebuttal sentence (\"X. No Y.\" / \"X. Just Y.\") or a manufactured-contrast aphorism (\"Not a feature. A platform.\") reads as AI cadence, not voice. Once is fine; the pattern is the tell.",
        reported: false,
    },
    Rule {
        id: "oversized-h1",
        category: Category::Slop,
        domain: "Typography",
        name: "Oversized hero headline",
        message: "A full-sentence headline set at display size ends up dominating the viewport, leaving no room for anything else above the fold. A punchy one- or two-word headline at that size is fine — the problem is a long headline blown up too large. Set long headlines smaller, or tighten the copy.",
        reported: true,
    },
    Rule {
        id: "extreme-negative-tracking",
        category: Category::Slop,
        domain: "Typography",
        name: "Crushed letter spacing",
        message: "Letter-spacing pulled tighter than the point where characters keep their own shapes costs legibility. Tighten display type optically, not destructively.",
        reported: true,
    },
    Rule {
        id: "broken-image",
        category: Category::Quality,
        domain: "Imagery",
        name: "Broken or placeholder image",
        message: "<img> tags with empty src, missing src, or placeholder values ship as broken-image boxes. Use real images, generated assets, or remove the tag.",
        reported: false,
    },
    Rule {
        id: "gray-on-color",
        category: Category::Quality,
        domain: "Color & Contrast",
        name: "Gray text on colored background",
        message: "Gray text looks washed out on colored backgrounds. Use a darker shade of the background color instead, or white/near-white for contrast.",
        reported: true,
    },
    Rule {
        id: "low-contrast",
        category: Category::Quality,
        domain: "Color & Contrast",
        name: "Low contrast text",
        message: "Text does not meet WCAG AA contrast requirements (4.5:1 for body, 3:1 for large text). Increase the contrast between text and background.",
        reported: true,
    },
    Rule {
        id: "layout-transition",
        category: Category::Quality,
        domain: "Motion",
        name: "Layout property animation",
        message: "Animating width, height, padding, or margin causes layout thrash and janky performance. Use transform and opacity instead, or grid-template-rows for height animations.",
        reported: true,
    },
    Rule {
        id: "line-length",
        category: Category::Quality,
        domain: "Layout & Space",
        name: "Line length too long",
        message: "Text lines wider than ~80 characters are hard to read. The eye loses its place tracking back to the start of the next line. Add a max-width (65ch to 75ch) to text containers.",
        reported: true,
    },
    Rule {
        id: "cramped-padding",
        category: Category::Quality,
        domain: "Layout & Space",
        name: "Cramped padding",
        message: "Text is too close to the edge of its container. Two shapes: (1) an element with its own text where the padding is too low for the font size, and (2) a wrapper with text-bearing children and near-zero padding against a visible boundary (border, outline, or non-transparent background) — children land flush against the boundary line. Add at least 8px (ideally 12–16px) of padding inside bordered, outlined, or colored containers.",
        reported: true,
    },
    Rule {
        id: "body-text-viewport-edge",
        category: Category::Quality,
        domain: "Layout & Space",
        name: "Body text touching viewport edge",
        message: "Body paragraphs render flush against the left or right viewport edge with no container providing horizontal padding. Wrap content in a container with at least 16px (ideally 24-32px) of horizontal padding, or apply max-width with mx-auto.",
        reported: true,
    },
    Rule {
        id: "tight-leading",
        category: Category::Quality,
        domain: "Typography",
        name: "Tight line height",
        message: "Line height below 1.3x the font size makes multi-line text hard to read. Use 1.5 to 1.7 for body text so lines have room to breathe.",
        reported: false,
    },
    Rule {
        id: "skipped-heading",
        category: Category::Quality,
        domain: "Typography",
        name: "Skipped heading level",
        message: "Heading levels should not skip (e.g. h1 then h3 with no h2). Screen readers use heading hierarchy for navigation. Skipping levels breaks the document outline.",
        reported: false,
    },
    Rule {
        id: "justified-text",
        category: Category::Quality,
        domain: "Typography",
        name: "Justified text",
        message: "Justified text without hyphenation creates uneven word spacing (\"rivers of white\"). Use text-align: left for body text, or enable hyphens: auto if you must justify.",
        reported: true,
    },
    Rule {
        id: "tiny-text",
        category: Category::Quality,
        domain: "Typography",
        name: "Tiny body text",
        message: "Body text below 12px is hard to read, especially on high-DPI screens. Use at least 14px for body content, 16px is ideal.",
        reported: true,
    },
    Rule {
        id: "all-caps-body",
        category: Category::Quality,
        domain: "Typography",
        name: "All-caps body text",
        message: "Long passages in uppercase are hard to read. We recognize words by shape (ascenders and descenders), which all-caps removes. Reserve uppercase for short labels and headings.",
        reported: true,
    },
    Rule {
        id: "wide-tracking",
        category: Category::Quality,
        domain: "Typography",
        name: "Wide letter spacing on body text",
        message: "Letter spacing above 0.05em on body text disrupts natural character groupings and slows reading. Reserve wide tracking for short uppercase labels only.",
        reported: true,
    },
    Rule {
        id: "text-overflow",
        category: Category::Quality,
        domain: "Layout & Space",
        name: "Content overflowing its container",
        message: "Content renders wider than its container, spilling out or forcing a horizontal scrollbar. Let text wrap, constrain widths, or give the region a deliberate scroll affordance.",
        reported: true,
    },
    Rule {
        id: "clipped-overflow-container",
        category: Category::Quality,
        domain: "Layout & Space",
        name: "Positioned child clipped by overflow container",
        message: "A clipping container (overflow hidden or clip) wrapping an absolutely-positioned child cuts off tooltips, menus, and popovers that need to escape. Let the overflow be visible, or move the positioned layer out of the clip.",
        reported: true,
    },
    Rule {
        id: "gpt-thin-border-wide-shadow",
        category: Category::Slop,
        domain: "Visual Details",
        name: "Hairline border with wide shadow",
        message: "A hairline border paired with a wide, diffuse shadow is a recurring generated-UI signature. Commit to one — a defined edge or a soft elevation — rather than both at once.",
        reported: true,
    },
    Rule {
        id: "repeating-stripes-gradient",
        category: Category::Slop,
        domain: "Visual Details",
        name: "Repeating-gradient stripes",
        message: "Repeating-gradient stripes used as surface decoration are a recurring generated-UI signature. Reach for a deliberate texture or leave the surface plain.",
        reported: true,
    },
    Rule {
        id: "theater-slop-phrase",
        category: Category::Slop,
        domain: "Copy",
        name: "Theater framing copy",
        message: "Dismissing something as \"theater\" is a recurring generated-copy tic. Say plainly what the thing does or does not do.",
        reported: false,
    },
    Rule {
        id: "image-hover-transform",
        category: Category::Slop,
        domain: "Motion",
        name: "Image hover transform",
        message: "Scaling or rotating an image on hover is a recurring generated-UI signature. Let imagery sit still, or use a subtler, purposeful interaction.",
        reported: true,
    },
    Rule {
        id: "root-overlap",
        category: Category::Quality,
        domain: "Layout & Space",
        name: "Overlapping artboards",
        message: "Two artboards overlap on the canvas, so one hides part of the other. Move them apart (Tidy up, ⌥⌘T, or the tidy_roots tool) or place new roots relative to measured neighbours.",
        reported: true,
    },
    Rule {
        id: "text-occluded",
        category: Category::Quality,
        domain: "Layout & Space",
        name: "Text hidden by an overlapping shape",
        message: "Part of this text is covered by a shape painted over it, or sits on a shape filled in its own colour, so the words disappear. Move the shape, change its paint order, or recolour one of them.",
        reported: true,
    },
    Rule {
        id: "size-duplicated",
        category: Category::Quality,
        domain: "Layout & Space",
        name: "Size stored twice",
        message: "This layer declares its size in both layout and style. Style is the one that renders, so the layout copy is dead and editing it does nothing. Any size edit (W / H, a resize, update_node) removes the stale copy.",
        reported: true,
    },
    Rule {
        id: "style-shorthand-conflict",
        category: Category::Quality,
        domain: "Layout & Space",
        name: "Shorthand beside its longhands",
        message: "This layer stores a combined property next to a per-side value of the same group (padding and paddingTop, for example). CSS lets the later one win, so the canvas and export agree, but the value lives in two places. Editing the group in the inspector, or with update_node, rewrites it in one form.",
        reported: true,
    },
];
