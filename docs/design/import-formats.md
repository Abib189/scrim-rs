# Importing other tools' files: research (Fix AG6, 1 Nov 2026)

## Scope

This note asks, for nine design tools, whether Scrim can read their files, and on what terms. It uses official sources only: the vendor's own documentation, specifications, licence and terms pages, official API docs and official repositories. Nothing here was reverse-engineered, and no one's design files were downloaded. Where an official source gives no answer, the note says "unknown". "Effort" means the work to write a Rust importer into Scrim's Figma-like model (frames, groups, shapes, vectors, text, fills, strokes, effects, images, components):
- **small**: little or no new parsing, because the route ends in a format Scrim already reads;
- **medium**: a new reader for a documented container (a zip of JSON, say) plus a mapping of the node model;
- **large**: a binary or page-description format whose structure does not map one-to-one onto layers, or one with undocumented parts.

All sources were checked on 1 November 2026. Scrim already opens SVG as editable layers ([`docs/file-format-parity.md`](../file-format-parity.md), Import table). It writes PDF but does not read it. That matters below, because most closed tools export SVG and PDF.

Three Adobe help pages (helpx.adobe.com) refused a direct fetch on the check date (HTTP 403). Their contents below come from the search index's copy of those official pages, and are marked so.

## Summary

| Tool | Documented format or export | Licence/terms | Effort | Recommendation |
|---|---|---|---|---|
| Paper (paper.design) | No documented file format. Exports PNG, JPG, WebP, AVIF, PDF, video, and SVG for SVG nodes only. A local MCP server returns JSX and computed CSS | ToS §4(iv) bars "benchmarking or the creation of a competing service". Whether a competing tool may read a user's file over MCP: unknown | Small for the SVG route (SVG nodes only). Large for PDF (needs a PDF importer). Medium for an MCP bridge | **Later**, through PDF import. No MCP bridge without Paper's written OK |
| Penpot (`.penpot`) | Yes. `.penpot` v3: a zip of JSON plus media, documented in the technical guide. Schema in the MPL-2.0 source | Code is MPL-2.0. The docs page states no licence of its own. Writing our own reader from the docs creates no MPL duty | Medium | **Next** |
| Affinity (`.af`, `.afdesign`, `.afphoto`, `.afpub`) | No public native format spec found (unknown). Exports PDF, SVG, PSD, PNG, JPEG, TIFF, WebP | Affinity Terms cl. 12 bar copying the software's "algorithms or code". Canva ToS §2(e)(ii) bars reverse engineering | Native: not possible from official sources. SVG route: small (works today) | **Not** native. Use SVG export now |
| Adobe XD (`.xd`) | No public format spec found (unknown). Exports PNG, SVG, JPG, PDF. XD is in maintenance mode | Adobe General Terms §17(B) bar reverse engineering | Native: not possible from official sources. SVG route: small (works today) | **Not** native. Use SVG export now |
| Adobe Photoshop (`.psd`) | Yes. Adobe Photoshop File Formats Specification (Nov 2019) | "Provided for 3rd parties to read and write the Photoshop native file format". No explicit licence grant beyond that. Copyright Adobe | Large | **Later** |
| Adobe Illustrator (`.ai`) | The native data has no public spec (unknown). A file saved with "Create PDF Compatible File" carries a PDF copy. PDF is ISO 32000-2, free from the PDF Association. Exports SVG | Adobe General Terms §17(B). ISO 32000-2 is free to obtain | SVG route: small (works today). The PDF inside `.ai`: large (a PDF importer) | **Later**, through a general PDF importer. Use SVG export now |
| Framer | No file format. No design export. Plugin API and Server API (beta) read and write the canvas | ToS v1.8 §2.5(f) bars using the Service "to develop a product or service similar or competitive to Framer's offerings" | Medium for an API bridge, but see terms | **Not** |
| Lunacy (Icons8) | Yes. `.free` format, documented in `icons8/free` (MIT). Lunacy also saves `.sketch` natively | MIT licence on the spec repository | Medium (close to Sketch). `.sketch` route: small once Sketch import lands | **Next**, after Sketch |
| Canva | No file format. Connect API exports PDF, JPG, PNG, GIF, PPTX, MP4, CSV and HTML. Users can download SVG on paid plans | Developer Terms §5(a)(iv) and §5(a)(viii) bar competing products and apps "primarily intended to export Designs from Canva into a third party design platform" | API importer: not allowed. SVG route: small (works today) | **Not** (no API importer). Users can download SVG themselves |

## Paper (paper.design)

**Format or export.** Paper's docs index has six pages: MCP, Snapshot, Tokens, Support, Paste and SVGs. None documents a native file format ([Docs](https://paper.design/docs)). The native format is **unknown**. Official routes out:
- **Exports.** The support FAQ says "Press ⌘⇧E to export images", and selections copy as Tailwind, React CSS or PNG ([Support](https://paper.design/docs/support)). The build log adds PDF export ("Export frames as PDFs", May 2026), combined PDF, WebP and AVIF, and video ([Build log](https://paper.design/build-log)).
- **The MCP server.** It runs locally at `http://127.0.0.1:29979/mcp` while Paper Desktop is open, and it "operates on the currently open file" ([MCP docs](https://paper.design/docs/mcp), [Support](https://paper.design/docs/support)). The server's own tool descriptions offer:
  - `get_jsx`, "the JSX code representation of a node and its descendants", with inline styles or Tailwind;
  - `get_computed_styles`, a map of node IDs to CSS properties;
  - `get_tree_summary`;
  - `export`, in AVIF, JPG, MP4, PDF, PNG, SVG and WebM. "SVG exports are only supported for SVG nodes", and "PDF exports require Paper Desktop".

  These descriptions come from the Paper MCP server as installed in this environment on 1 Nov 2026. They are not on a public web page.

**Terms.** The [Terms of Service](https://paper.design/legal/tos) were last updated on 11 October 2026:
- §4(iv) bars attempts to "disassemble, decompile, reverse compile, reverse engineer, or otherwise attempt to discover the source code", and to "use the Service for benchmarking or the creation of a competing service".
- §4(v) bars automated scraping or downloading of data from the Service's web pages.
- §3.2: "Company does not claim ownership of Your Content."
- The terms say nothing about MCP or about exports. Whether a competing design tool may read a user's open file through the MCP server is **unknown**. §4(iv) makes it risky without written permission.

**Effort.**
- SVG export: small. It works today, but only for SVG nodes, so frames, text and layout don't come across.
- PDF: large. Scrim has no PDF importer yet.
- An MCP bridge mapping JSX and computed CSS onto Scrim layers: medium. Paper is HTML and CSS, so flex layout maps well, but terms are unclear.

**Recommendation: later.** Paper's PDF export becomes useful once Scrim has a general PDF importer (see Illustrator). Don't build an MCP bridge unless Paper agrees in writing.

## Penpot (`.penpot`)

**Format.** Yes, documented.
- The current export is `.penpot` v3: "a ZIP archive with JSON metadata and binary media". The user guide says "your design data is never locked in a proprietary format" ([user guide: the .penpot file format](https://help.penpot.app/user-guide/export-import/penpot-file-format/)).
- The technical guide documents the layout ([technical guide: Penpot file format](https://help.penpot.app/technical-guide/developer/data-model/penpot-file-format/)):
  - `manifest.json`, with `type: "penpot/export-files"`, the Penpot version, files with their feature flags, and library `relations`;
  - one JSON file per shape under `files/{file-id}/pages/{page-id}/`;
  - `components/`, `colors/`, `typographies/` and one `tokens.json`;
  - media metadata pointing at binaries in `objects/`.
- The guide says "The authoritative schema definitions are in the Penpot source code", for example `common/src/app/common/types/shape.cljc` and `backend/src/app/binfile/v3.clj` ([repository](https://github.com/penpot/penpot)).
- Two oddities. The manifest's own `version` field reads `1` while the page calls the format v3. Feature flags such as `components/v2` and `design-tokens/v1` tell an importer what it must support.
- Older v1 binary files, and the old `.zip` export of SVG plus JSON, can only be exported from Penpot 2.3 or earlier ([user guide](https://help.penpot.app/user-guide/export-import/penpot-file-format/)). A reader can target v3 only.
- Penpot also exports PNG, JPEG, WebP, SVG and PDF per layer ([Exporting layers](https://help.penpot.app/user-guide/export-import/exporting-layers/)), so the SVG route works today too.

**Licence.**
- The repository is MPL-2.0: "This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0" ([repository README](https://github.com/penpot/penpot)).
- The documentation pages state no licence of their own. They are published from that repository.
- MPL-2.0 is copyleft per file. §3.3 says "You may create and distribute a Larger Work under terms of Your choice", provided the MPL files stay under the MPL ([MPL 2.0](https://www.mozilla.org/en-US/MPL/2.0/)). So a Rust reader we write ourselves from the documentation creates no MPL duty. Copying Penpot source files, or test files taken from them, into Scrim would make those files MPL-2.0. Keep fixtures generated in code.

**Effort: medium.** A zip of documented JSON maps closely onto Scrim's model: frames, groups, rects, circles, paths, text, booleans, fills, strokes, shadows, blur, images and components. The work is the field mapping. The schema is in Clojure, so reading it takes some care. Penpot's flex and grid layout match Scrim's auto layout and grid.

**Recommendation: implement next.** It is open, documented, has a growing user base, and its licence is clear.

## Affinity (`.af`, `.afdesign`, `.afphoto`, `.afpub`; Serif, now Canva)

**Format or export.** No official public specification was found, so the native format is **unknown**. Affinity now uses one `.af` format, and Canva also accepts the legacy `.afdesign`, `.afphoto` and `.afpub` ([Canva: upload formats](https://www.canva.com/help/upload-formats-requirements/)). Official routes out:
- "You can export vector and raster assets in PDF, SVG, PNG, JPEG, TIFF, and WebP", "by object, slice, or artboard". The same page warns that exports to PDF or SVG do "not preserve full editability" ([Affinity: graphic design software](https://www.affinity.studio/graphic-design-software)).
- Affinity Designer also exports PSD ([Affinity Designer help: supported file formats](https://affinity.help/designer2ipad/en-US.lproj/pages/Appendix/fileformat.html)).

**Terms.**
- [Affinity Terms](https://www.canva.com/policies/affinity-additional-terms/) cl. 12: "you have no right to make a copy of any of the Affinity Software, or of any algorithms or code" (with stated exceptions).
- Cl. 15: you "may use and develop your own content".
- The [Canva Terms of Use](https://www.canva.com/policies/terms-of-use/) §2(e)(ii) (effective 30 October 2026) bar attempts to "copy, replicate, decompile, reverse-engineer, attempt to derive the source code of" the Service.
- No clause on file formats or interoperability was found.

**Effort.**
- Native: not possible from official sources.
- SVG route: small. Scrim's SVG import already gives editable layers. Affinity's own warning about lost editability applies.
- PDF and PSD routes: see Illustrator and Photoshop.

**Recommendation: not native.** Point users to File ▸ Export ▸ SVG now. Affinity's PDF export comes for free once a PDF importer exists.

## Adobe XD (`.xd`)

**Format or export.** No official public file format specification was found, so the format is **unknown**.
- Adobe's developer site documents only the plugin APIs (XD API through UXP). It says "We are no longer accepting new plugins for Adobe XD in our Marketplace" ([Adobe XD developer docs](https://developer.adobe.com/xd/uxp/)).
- Adobe's help says XD is "currently in maintenance mode": Adobe is not shipping new features, but keeps fixing bugs and security issues ([XD help](https://helpx.adobe.com/support/xd.html), [XD FAQ](https://helpx.adobe.com/xd/desktop/introduction/faq.html); search-index copy, see Scope).
- XD exports PNG, SVG, JPG and PDF, per selection, per artboard or in batches. SVG has a choice of presentation attributes or internal CSS, with images embedded or linked ([Export design assets](https://helpx.adobe.com/xd/desktop/share-export-and-review/export-design-assets.html); search-index copy).

**Terms.** The [Adobe General Terms of Use](https://www.adobe.com/legal/terms.html) (effective 3 October 2025):
- §17(B) bars you from trying to "reverse engineer (including but not limited to monitoring or tracking the inputs and outputs" of the Services or Software.
- The only exception is for laws that "give you the right to decompile". Even then you must "first request such information from us".
- §4.2: "We do not claim any ownership rights to your Content."

**Effort.** Native: not possible from official sources. SVG route: small (works today).

**Recommendation: not native.** The product is in maintenance mode and the format is closed. Users should export SVG per artboard.

## Adobe Photoshop (`.psd`)

**Format.** Yes. The [Adobe Photoshop File Formats Specification](https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/) (November 2019) covers:
- PSD and PSB;
- layer records, channel data and blend modes;
- vector masks (`vmsk`, `vsms`);
- type layers (`TySh`) and effects (`lfx2`, older `lrFX`);
- placed and smart object layers (`SoLd`, `PlLd`, `SoLE`).

Much of the modern data (text, effects, smart objects) sits in "Descriptor" structures that the spec only outlines. The spec says it "does not have any information regarding the PSDC (Photoshop Cloud Document)", which is private.

**Terms.** Under "Audience", the spec says: "This document is provided for 3rd parties to read and write the Photoshop native file format." Its notice reads "Copyright © 1991-2019 Adobe Systems Incorporated. All rights reserved". It says the information is "furnished for informational use only, is subject to change without notice". There is no separate, explicit licence grant to implement it. The stated purpose is third-party reading and writing.

**Effort: large.** PSD is a raster format. Most layers are pixels, which would import as image layers in groups, and that part alone is medium. Editable text (`TySh` descriptors), shape layers (vector masks with fills), and effects are the large part. Smart objects and adjustment layers have no counterpart in Scrim's model.

**Recommendation: later.** It is worth doing once, as "layers as images in groups, text and shapes where the spec allows". Photoshop is not mainly a UI design tool, so it comes after Penpot and Lunacy. Affinity also exports PSD, which adds a little value.

## Adobe Illustrator (`.ai`)

**Format or export.** Adobe publishes no specification of Illustrator's native data, so the native format is **unknown**. Adobe's help explains "Create PDF Compatible File" ([Saving artwork](https://helpx.adobe.com/illustrator/using/saving-artwork.html); search-index copy):
- it "saves a PDF representation of the document in the Illustrator file";
- leaving it off reduces file size, because the file otherwise stores both PDF and PGF data ([Optimize native and PDF file sizes](https://helpx.adobe.com/illustrator/kb/optimize-native-pdf-file-sizes.html); search-index copy).

So an `.ai` file saved with that option holds a PDF, and PDF is an open standard. ISO 32000-2 (PDF 2.0) is "available at no cost" from the PDF Association, sponsored by Adobe, Apryse and Foxit ([PDF Association: ISO 32000-2](https://pdfa.org/resource/iso-32000-2/), [announcement](https://pdfa.org/announcing-no-cost-access-to-iso-32000-2-pdf-2-0/)). Illustrator also exports SVG through File ▸ Export ▸ Export As, with options for styling, fonts, images and decimal places ([Export artwork](https://helpx.adobe.com/illustrator/desktop/save-and-export/export-files-to-different-formats/export-artwork-from-illustrator.html); search-index copy).

**Terms.** Native data: Adobe General Terms §17(B), as under XD. The PDF path is ISO 32000-2, which is free to obtain from the PDF Association.

**Effort.**
- SVG route: small (works today).
- Reading the PDF inside `.ai`: large. It means a PDF importer: content streams to paths, text runs to text layers, images, transparency groups, and optional content (layers) to groups. PDF describes pages, not a layer tree, so frames, components and editable text structure don't survive.
- A file saved without PDF compatibility can't be read from official sources.

**Recommendation: later, through a general PDF importer.** That importer would also cover PDFs from Affinity, XD, Paper, Penpot and Canva. Until then, users export SVG.

## Framer

**Format or export.** Framer has no design file format.
- Framer's help says "Framer does not offer HTML export for self-hosting" ([Can I export my website to HTML…](https://www.framer.com/help/articles/can-i-export-my-website-to-html-and-self-host-it/)).
- A second help page says published sites are standard HTML, CSS and JavaScript that "you can download … at any time", and that CMS content exports to CSV or JSON through plugins ([Porting your data](https://www.framer.com/help/articles/porting-your-data-from-framer/)). The two pages disagree. Neither covers canvas or design export.
- The [Plugin API](https://www.framer.com/developers/nodes) reads and writes canvas nodes: `FrameNode`, `TextNode`, `SVGNode`, `CodeComponentNode`.
- The [Server API](https://www.framer.com/developers/server-api-reference) (beta, free during beta, per the [FAQ](https://www.framer.com/developers/server-api-faq)) "shares most methods" with the Plugin API and runs from outside Framer with a per-project key.
- Whether Framer lets users copy or export canvas layers as SVG is **unknown**. No official page says so.

**Terms.** The [Terms of Service](https://www.framer.com/legal/terms-of-service) are v1.8, effective 16 June 2026:
- §2.5(f): you may not "use the Service to develop a product or service similar or competitive to Framer's offerings";
- §2.5(a) bars reverse engineering and non-public APIs;
- §7.3: "Customer retains full ownership of Customer Content".

Whether a design tool may read users' projects through the Server API is **unknown**. §2.5(f) suggests not.

**Effort.** An API bridge would be medium, but it needs Node.js and Framer's npm package, an API key per project, and acceptable terms.

**Recommendation: not.** There is no file to read, and the only official route is an API whose terms bar competing products.

## Lunacy (Icons8)

**Format.** Yes, two routes.
- Lunacy's docs say "The default file format in Lunacy is .free", linking to the spec ([About Lunacy](https://lunacy.docs.icons8.com/about)).
- [`icons8/free`](https://github.com/icons8/free) describes FREE as "an open source format for vector graphic files". It is a zip holding:
  - `pages/` (JSON layer trees), `images/`, `fonts/`;
  - `shared/` (library components and styles);
  - `document.json`, `meta.json` and `preview.webp`.
- The README documents frames, groups, sections, shapes, paths, text, components, instances with overrides and variants, fills, borders, shadows, blur, auto layout, variables and prototyping. It ships C# schema types (`src/cs/Free.Schema`). The repository was last pushed on 28 October 2026.
- The same docs say Lunacy "works natively with .sketch files", and can open, edit and save them in that format ([About Lunacy](https://lunacy.docs.icons8.com/about)).

**Licence.** MIT ([LICENSE](https://github.com/icons8/free/blob/main/LICENSE): "Copyright (c) 2023 Icons8. Permission is hereby granted, free of charge…"). Lunacy itself "is completely free for personal and commercial use" ([About Lunacy](https://lunacy.docs.icons8.com/about)).

**Effort.**
- `.sketch` route: small once Scrim's Sketch import lands. Users Save as `.sketch` in Lunacy.
- Native `.free`: medium. It is a zip of JSON. FREE was designed as Sketch's successor, with shorter keys, nested frames, and richer overrides, so much of the Sketch mapping carries over.

**Recommendation: next, after Sketch.** The spec is MIT and the model is close to Scrim's. Until it ships, tell Lunacy users to save as `.sketch`.

## Canva

**Format or export.** Canva has no file format. The [Connect API](https://www.canva.dev/docs/connect/api-reference/exports/create-design-export-job/) export job supports `pdf`, `jpg`, `png`, `gif`, `pptx`, `mp4`, `csv`, `html_bundle` and `html_standalone`. The same page returns an error saying SVG export is currently unavailable. In the app, users can download a design as SVG on paid plans (Pro, Teams and others) ([Download or save designs](https://www.canva.com/help/download-or-purchase/)).

**Terms.** The [Canva Developer Terms](https://www.canva.com/policies/canva-developer-terms/) were last updated on 16 October 2026:
- §5(a)(ii) bars attempts to "reverse-engineer, attempt to derive the source code of" the APIs or Platform;
- §5(a)(iii) bars "competitive analysis";
- §5(a)(iv) bars "building or marketing a product or service that is competitive with Canva";
- §5(a)(viii) bars an App "primarily intended to export Designs from Canva into a third party design platform or system" without Canva's written consent;
- §5(a)(x) bars "data mining, extraction, or scraping".

The [MCP usage rules](https://www.canva.dev/docs/apps/mcp/prohibited-use/) repeat the export and reverse-engineering bans. For users, the [Terms of Use](https://www.canva.com/policies/terms-of-use/) §4(a) say "you own all right, title and interest in and to your User Content", and §15(c) lets you export designs "using the functionality of the Service". Designs that contain Canva's licensed elements stay under the [Content License Agreement](https://www.canva.com/policies/content-license-agreement/) (§4(c)).

**Effort.**
- API importer: not allowed.
- User-downloaded SVG: small (works today).
- User-downloaded PDF: needs the PDF importer (large).

**Recommendation: not.** Don't build anything that calls Canva's APIs. A user who downloads their own design as SVG can open it in Scrim today.

## Recommended order

For context: Sketch (`.sketch`) import is being implemented now from the official `sketch-hq/sketch-file-format` specification (the repository now redirects to [`sketch-hq/sketch-document`](https://github.com/sketch-hq/sketch-document), MIT).

1. **Penpot `.penpot` v3.** It is open and documented, and its licence is clear. It is also the main open-source tool whose users may move to Scrim.
2. **Lunacy `.free`.** The MIT spec is close to Sketch, so it reuses the Sketch mapping. Lunacy's `.sketch` saves work as soon as Sketch import does.
3. **Write up the SVG route in the Import dialog and help** for Illustrator, Affinity, XD, Canva (paid plans), Paper (SVG nodes) and Penpot. It needs no new parser, and it is the only route the closed tools document.
4. **A general PDF importer (later).** One importer would cover Illustrator files saved PDF-compatible, plus PDF exports from Affinity, XD, Paper, Penpot and Canva. ISO 32000-2 is free.
5. **PSD layered import (later).** The format is documented for third parties. Layers would come in as images in groups, with text and shapes where the spec allows. It matters less for UI work.

Not planned: native Affinity and XD files (no public format, and the terms bar reverse engineering); Framer (no file to read, and the terms bar competing use); a Canva API importer (barred by Developer Terms §5(a)(iv) and §5(a)(viii)); a Paper MCP bridge (unless Paper agrees in writing).

## Unknowns (as of 1 Nov 2026)

- Paper: the native file format, and whether a competing tool may read a user's open file through Paper's MCP server.
- Affinity: any public specification of `.af`, `.afdesign`, `.afphoto` or `.afpub`. None found in official sources.
- Adobe XD: any public specification of `.xd`. None found.
- Adobe Illustrator: any public specification of the native (PGF) data. None found. Files saved without PDF compatibility can't be read from official sources.
- Photoshop: an explicit licence grant beyond the spec's stated purpose. The spec is the November 2019 edition, and no newer edition was found.
- Framer: whether users can export canvas layers as SVG, and whether the Server API may be used by a third-party importer.
- Penpot: a licence on the documentation itself. Its repository is MPL-2.0. The manifest `version: 1` versus "v3" naming is not explained.
- Canva: when or whether SVG export returns to the Connect API.
