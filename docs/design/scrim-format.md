# .scrim as the native format (Fix AG, 1 Nov 2026)

## Findings (before any change, at commit 84c163b)

### 1. Where `.figcraft` and the `figcraft` document type appear

The complete list, 633 lines, is in [`scrim-format/figcraft-mentions.txt`](scrim-format/figcraft-mentions.txt) (`file:line: text`). By count:
- 343 in tests;
- 109 in docs;
- 68 in the 16 locale catalogues;
- 15 in packaging;
- 7 in xtask;
- the rest in shipped code.

The binary names `figcraft` and `scrim`, the crate names, the plugin import module `"figcraft"`, the `figcraft://` invite scheme and the MCP config keys `figcraft`/`figcraft-app` (old client entries Scrim cleans up) also contain the word, but they aren't the document format. They don't change.

What names the format, by place:

| Place | Where | What it does today |
|---|---|---|
| The format crate | `crates/format/src/lib.rs` | `EXTENSION = "figcraft"`, `FORMAT = "figcraft"` (written into `document.json`), docs "the native `.figcraft` format". `thumbnail.rs` and `stored_name.rs` pick readers by the extension `figcraft`. `library.rs` names its archive a `.figcraft` archive (internal: a library's bytes on a server, never a file name) |
| Open | `crates/engine/src/cmd/file.rs` `open` / `load_with` | Picks by **extension**: `svg`, image extensions; everything else is tried as `.fig`, then REST API file JSON, then native. A file opened as native keeps its path only when its extension is `figcraft` or `json` |
| Save, Save as | `file.rs` `save`, `save_as`, `save_to` | `file.saveAs` adds `.figcraft` when the path has no extension. `file.save` refuses a stored `.fig` path and asks for Save as; any other path is written in place, whatever its extension (a `.json` file opened as native is overwritten with a zip) |
| Auto-save | `crates/ui-egui/src/lib.rs` `auto_save_tick` | Writes every dirty document **that has a path**, with no check of the path's extension |
| Versions and branches | inside the file (`versions/`, `branches/` in the zip) | No files of their own. `version.autosave` only adds an entry in memory |
| Recovery copies | `apps/figcraft/src/recovery.rs` | `Recovery/<pid>/<uid>.figcraft`; recovery lists only `*.figcraft` |
| Save dialog | `crates/ui-egui/src/dialogs.rs` `save_as_dialog`; `apps/figcraft/src/services.rs` `save_filters` | Default `"<name>.figcraft"`, filters "Scrim (.figcraft)" then ".fig (experimental)"; toasts "Use the .figcraft or .fig extension.", "A .figcraft file with this name already exists…" |
| Open dialog, drag and drop | `crates/ui-egui/src/lib.rs` `OPEN_EXTENSIONS` (`figcraft fig json svg png jpg jpeg webp gif tif tiff`), the Import dialog list at line 1078 | A hand-kept list, separate from what `file.open` can read |
| Home | `crates/ui-egui/src/home.rs` | "Open .figcraft, .svg or images" on the Open file card; "Drop a .figcraft, .fig, .json, .svg or image file." |
| Libraries | `assets_panel.rs` | "Add library" picks `figcraft` only; "No libraries yet. Add a .figcraft file…" |
| Locale catalogues | `crates/ui-egui/locales/*.toml` (16) | The five strings above in every language |
| MCP | `crates/mcp/src/tools.rs` | `open_file` "Open a .figcraft or .json document…"; `save_file` "Save the active document as a .figcraft file", `path` "Output .figcraft path…"; `howToOpen` example `design.figcraft` |
| CLI | `apps/figcraft-cli/src/files.rs`, `main.rs` | Reads and writes by the extension `figcraft`; help text "Files: .figcraft (native)…" |
| macOS | `packaging/macos/Info.plist.in`, `xtask/src/bundle_macos.rs` | One **imported** UTI, `ai.storyteller.figcraft.document` (FigCraft's), extension `figcraft`, MIME `application/x-figcraft`, described as "Scrim Document"; a document type "Scrim Document", role Editor, rank Owner, icon `Scrim`; SVG (Editor, Alternate) and PNG/JPEG (Viewer, Alternate). The bundle builder fills the template. No `.scrim` type |
| Apple events | `apps/figcraft/src/apple_events.rs` | Opens whatever Finder sends; no extension check |
| Linux | `packaging/linux/design.scrim.app.mime.xml`, `.desktop`, the Flatpak bundle | MIME `application/x-figcraft`, glob `*.figcraft` |
| Windows | `packaging/windows/figcraft.wxs` | Extension `.figcraft`, `application/x-figcraft`, Scrim its default app |
| Web | `apps/figcraft-web` | No file dialogs; files come by drag and drop through the same `OPEN_EXTENSIONS` |
| Examples | `examples/*.figcraft` (5 synthetic demos) | Read by tests and docs |
| Docs | `docs/cli.md`, `docs/mcp.md`, `docs/plugins.md`, `docs/file-format-parity.md`, `README.md`, `AGENTS.md`/`CLAUDE.md` and others | Examples with `design.figcraft` |

### 2. The container format (`.figcraft`, format 1)

- **A zip** (`PK\x03\x04`). Members:
  - `document.json`: the document; deflated at a fast level;
  - `images/<sha256>`: stored, one per image, shared by the document, versions and branches;
  - `thumbnail.png`: stored;
  - `versions/index.json` and `versions/<id>.json`;
  - `branches/index.json` and `branches/<id>/{base,head}.json` and `versions/`;
  - the published library's members when the file publishes one.
- **`document.json`** begins `{"format":"figcraft","version":1,"imageIndex":[…],"name":…,"pages":…,"nodes":…}`.
  - `format` is written but **never checked** on reading.
  - `version` is checked: a newer one is refused ("made by a newer version").
  - Bare JSON (the document object alone, no zip) also loads.
- **Magic:** none beyond the zip header. What identifies a native file is a zip whose `document.json` is a JSON object with `"format": "figcraft"`, or with `pages` and `nodes` and no `_class` (that is how a file from before the `format` key looks).
- **Caps:** members are read within caps (512 MB a member, `document.json` up to 16× its packed size, at most 4 GB; 100 versions), so a hostile zip can't unpack into gigabytes.

### 3. The old TypeScript-era `.scrim`

- **No such file is in this repository** (`git ls-files '*.scrim'` is empty). Stage D's "Fixture" was rebuilt by hand because scrim-rs couldn't open the person's old `.scrim` file (`docs/design/layers-collapse-and-focus.md`).
- **The format is described by the TypeScript repo's `packages/core/src/schema.ts`** and appears as data in `docs/scrim-transfer/cases/query-nodes.json › documents` (five synthetic documents, Scrim's own, MIT as declared in its packages).
- **What it looks like:** **plain JSON, pretty-printed** (`JSON.stringify(doc, null, 2)` plus a newline; the library keeps them as `documents/<uuid>.scrim`, up to 200 MB):
  ```json
  {"schemaVersion": 1, "id": "…", "name": "…", "rootIds": ["page", …],
   "nodes": {"page": {"id": "page", "type": "frame", "name": "Page", "parentId": null,
                      "childIds": […], "layout": {"display": "flex", "gap": 32, …},
                      "style": {"width": 1280, "background": "#ffffff", …},
                      "canvas": {"x": 0, "y": 0}}, …},
   "components": {…}, "tokens": {…}, "canvasBackground"?: "…", "lint"?: {…}}
  ```
  - A flat, id-keyed node map; the tree is in `parentId`/`childIds`.
  - **Node types:** `frame`, `box`, `text`, `image`, `instance`.
  - **`layout` and `style` are CSS properties** in camelCase: flex, grid, absolute positions and sizes as numbers or CSS strings like `"50%"` and `"fit-content"`.
  - Root frames carry canvas positions.
  - **Images are `src: "scrim-asset:<sha256>.<ext>"`**, pointing into the app's content-addressed asset store, **not inside the file**.
  - Text is a plain string with CSS font properties.
  - Instances refer to `components[<id>].rootId` with style and text overrides.
  - There are also hover and other state styles, a reduced-motion media block, inline SVG content, layout guides and lint settings.
- **Telling the two `.scrim` formats apart by contents:**
  - the new one is a **zip** (or the bare native JSON, with `pages`);
  - the old one is a JSON object with `schemaVersion` and `rootIds`, and no `pages`.
  - The two can't be confused.

## What Fix AG changes

### AG1. `.scrim` is the native format

- **Same container, new extension.**
  - `figcraft_format::EXTENSION` is `scrim`; `LEGACY_EXTENSION` is `figcraft`.
  - The zip, its members and the version scheme are unchanged, and `document.json` still says `"format": "figcraft"`, version 1.
  - So a `.scrim` and a `.figcraft` are byte-for-byte the same format, and each opens wherever the other does.
  - Nothing in the findings called for a change: the format already has a required key, so it can be sniffed.
- **What a file is comes from its contents** (`crates/format/src/sniff.rs`):
  - a zip whose `document.json` says `"format": "figcraft"` (or, from before that key, has `pages` and no other tool's marks);
  - or that JSON bare.
  - Only the first 64 KB of `document.json` is read, and the keys before the format are skipped over unparsed.
- **Writing `.scrim`:**
  - Save, Save as, auto-save, recovery copies (`Recovery/<pid>/<uid>.scrim`) and MCP `save_file` write `.scrim`.
  - Any other name gets `.scrim` added: "Poster" becomes "Poster.scrim", "poster.v2" becomes "poster.v2.scrim".
  - The Save dialog offers "Scrim (.scrim)" first, then ".fig (experimental)", and suggests "Untitled.scrim" style names.
  - The CLI writes `.scrim` (`run --out x.scrim`) and refuses `.figcraft`.
  - Versions and branches stay inside the file, as before.
- **Byte-stable:** a second save of an unchanged document writes the same bytes, and so does a reload and save (`importers_tests.rs`).

### AG2. Backward compatibility

- **`.figcraft` opens forever and is never written:**
  - Opening never writes, as in Fix AF.
  - The document keeps the `.figcraft` path, so its name, the recent files and "already open" work as before.
  - Save opens Save as with `<same name>.scrim` in the same folder. The `.figcraft` is never modified or deleted.
  - From that save on, the document is the `.scrim`.
  - A `.figcraft` path given to Save as, `file.save` or `save_file` is refused, and auto-save skips the document.
- **The two `.scrim` formats are told apart by their contents:**
  - A new-format `.scrim` opens normally.
  - An earlier Scrim file goes through the import registry (`crates/format/src/old_scrim/`):
    - frames and boxes, flex layout as auto layout, absolute positions, backgrounds, borders, radius, opacity, shadows, overflow;
    - text with its font, size, weight, style, colour, line height, letter spacing, alignment, case and decoration;
    - the canvas background.
    - It opens as a new unsaved document with a report. Images (not in the file), instances (copied), grid layout, CSS the model has no place for, states, reduced-motion rules, inline SVG, links and tokens are listed in the report, with their layers.
  - A file with a newer `schemaVersion`, or a damaged one, is refused with a localised message saying what it is and that nothing was changed. Nothing is guessed, and nothing is written.
- **Recovery copies** left by older builds (`.figcraft`) still come back.

### AG3. Operating system integration

- **macOS (`packaging/macos/Info.plist.in`):**
  - an **exported** UTI `design.scrim.document`, conforming to `public.data` and `public.content`, with extension `scrim` and MIME `application/x-scrim`;
  - a document type "Scrim Document": role Editor, rank Owner, icon `Scrim` (the brand mark);
  - `.figcraft` keeps its imported UTI and its document type as before (Editor, Owner), now named "FigCraft Document".
- **Double-click, Open With and Dock drops** arrive as before (`apple_events.rs`) and are read by contents.
- **Linux:** the MIME type `application/x-scrim` (`*.scrim`) is in the shared-mime-info file and the `.desktop` entry.
- **Windows:** `.scrim` is an extension of the "Scrim.Document" ProgId and in "Open with".
- **File dialogs** on every platform, and the web build's drops, take the registry's extensions.
- **The test:** `xtask/src/bundle_macos_tests.rs` parses the built plist with `plutil`.

### AG4. The import registry

- **The interface** (`crates/engine/src/importers/`): an `Importer` has
  - `id`;
  - `extensions`;
  - `listed` (how Home names it);
  - `native`;
  - `sniff(bytes, name) -> 0..=100`;
  - `import(bytes, options) -> Imported` (a document, its versions and branches, and `Vec<ImportWarning>`).
  - `ImportWarning` is `{severity: dropped | approximated | info, path: the layer's names from the page down, message}` (`crates/format/src/import_report.rs`).
- **The registry, in one place:**
  - `REGISTRY = [Scrim document, earlier Scrim, .fig, Sketch, REST API file JSON, SVG, images]`.
  - The surest sniff wins, and the file's name never decides.
  - Adding an importer is one entry; the UI reads the Open and Import dialogs' extensions, drag and drop, and Home's "Open .scrim, .fig, .sketch, .svg or images" from it.
- **The existing readers moved behind it unchanged:**
  - `.fig`, REST API file JSON, SVG and images give the same documents as before (`the_existing_importers_give_the_same_output_through_the_registry`).
  - Their sentence notes become warnings: "skipped", "left out" or "not supported" mean dropped; "approximat…" or "imported as" mean approximated; everything else is info.
- **`file.open`, `file.openRecent`, `file.import`, drops and MCP `open_file`** all go through it.
  - An import opens as a new unsaved document, and the source is never written.
  - `file.import` appends the pages of anything that isn't an SVG or an image.
- **Results** carry `warnings: [{severity, path, message}]`.
- **The import report** (`crates/ui-egui/src/import_report.rs`):
  - the counts by severity ("2 left out · 1 approximated"), each warning with its layer, and **Copy report**;
  - no warnings, no dialog;
  - English and Spanish.

### AG5. Sketch

`crates/format/src/sketch/` follows the official file format (`@sketch-hq/sketch-file-format` 6.5.0 JSON Schema, document versions 121–146; the module header lists the fields and how each is read). Its fixture is `crates/format/tests/fixtures/sketch/sample.sketch`, built by the tests, MIT OR Apache-2.0.

**What maps:**
- pages;
- artboards → frames;
- groups;
- boolean shape groups;
- rectangles with per-corner radius;
- ovals;
- polygons, stars and triangles;
- vector paths;
- text with its runs, font, size, weight, colour, alignment and auto-resize;
- solid, linear, radial and angular fills, and image fills;
- borders;
- shadows and inner shadows;
- blur;
- opacity and blend;
- images;
- masks;
- symbols → components, with their instances and overrides;
- export settings.

**What doesn't** is a warning with its layer, never a silent drop:
- prototyping, hotspots, grids, smart layout, resizing constraints (their bits aren't documented);
- nested symbol swaps, layer-style overrides;
- unknown classes.

### AG6. Research

`docs/design/import-formats.md`: Paper, Penpot, Affinity, Adobe XD, Photoshop, Illustrator, Framer, Lunacy and Canva, from official sources only.

## Decisions (overrule them)

- **Saving a document opened from a `.figcraft` writes a new `.scrim` beside it.**
  - Save opens Save as with `<name>.scrim` in the `.figcraft`'s folder.
  - After that save the document is the `.scrim`: its tab, recent file and next saves.
  - The `.figcraft` is never modified, renamed or deleted, and stays in the recent files.
- **The format's internal name stays `"figcraft"`** in `document.json`, so `.scrim` and `.figcraft` stay one format and older Scrim builds open a `.scrim` (renamed). Only the extension changed.
- **Scrim writes only `.scrim` in place.**
  - A Scrim document opened from any other name (a `.json` from FigCraft's days, a `.scrim` renamed `.txt`) is also saved through Save as. It used to be written in place: a `.json` was overwritten with a zip.
  - `.figcraft` and `.json` paths given to Save as are refused.
  - Any other name gets `.scrim` added.
- **The earlier Scrim's files are imported, not opened:**
  - they get no path (the next save asks where);
  - images aren't in those files, so they become grey placeholders, listed in the report;
  - instances become copies of their component.
- **Old notes' severities are inferred from their words** (the `.fig`, REST and SVG readers report sentences). Their text is unchanged.
- **The `.figcraft` document type keeps rank Owner** on macOS and Windows, as before, so double-clicking one still opens Scrim. Sketch and `.fig` files get no Finder document type: Sketch's own UTI isn't in its public documentation.
- **The import report shows only when there are warnings.** Before, a `.fig` or REST import without warnings showed a report saying so.

## Tests

- **`crates/format/src/sniff_tests.rs`:** a Scrim document by its contents (zip, bare JSON, from before the format key); the earlier Scrim; Sketch, `.fig`, REST JSON, images, text and garbage are not Scrim documents; the key scanner never panics.
- **`crates/format/src/old_scrim_tests.rs`:**
  - the TypeScript Scrim's own five synthetic documents (`docs/scrim-transfer/cases/query-nodes.json`) import;
  - CSS maps to auto layout, sizing, borders, shadows and text;
  - newer or damaged files are refused; loops, missing children and unknown types are reported.
- **`crates/format/src/sketch/tests/`:** 36 tests, one per feature, plus unsupported features, hostile files and the committed sample.
- **`crates/engine/src/importers_tests.rs`:**
  - save writes `.scrim`, round-trips and is byte-stable;
  - a `.figcraft` opens unchanged, Save asks for a `.scrim` beside it, auto-save skips it, and the original is untouched;
  - sniffing: a `.scrim` renamed `.txt`, a `.figcraft` renamed `.scrim`, the earlier Scrim's file, a newer or damaged one, garbage and an empty file;
  - every existing importer gives the same output through the registry;
  - the extension list and Home's names come from it.
- **`crates/ui-egui/src/save_as_tests.rs`:** the Save dialog's `.scrim` default and filters; Save on a `.figcraft` document.
- **`crates/ui-egui/src/scrim_format_ui_tests.rs`:** Home's text, drops, the Sketch report and none for a Scrim document, and the refusals in English and Spanish.
- **`crates/ui-egui/src/import_report_tests.rs`:** counts by severity, layer paths, Copy report, and no report without warnings.
- **`xtask/src/bundle_macos_tests.rs`:** the built plist's `.scrim` UTI and document type, read back by `plutil`.
- **`crates/mcp/tests/scrim_format.rs`:** `save_file` and `open_file` with `.scrim`; a `.figcraft` never written over; a Sketch import's warnings.
- **Existing tests** that saved to `.figcraft` paths now save to `.scrim`: Scrim refuses to write `.figcraft`.

## Live check (1 Nov 2026)

A real window: the debug app with a scratch `HOME` and `SCRIM_SETTINGS_DIR`, MCP on 7690 and control on 7979. Input went through the window's own control channel; the system pointer was never moved. The script, its output and the captures are in `scrim-format/` (`live.py`, `live-output.txt`, `captures.png`).
1. **A new poster saved with no extension** became `Poster.scrim`:
   - a zip with `document.json` (`"format": "figcraft"`) and `thumbnail.png`;
   - a second save wrote the same bytes.
2. **Reopened through MCP `open_file`:** "Poster", with its path, and no repairs.
3. **A `.figcraft` copy opened** as "Festival":
   - MCP `save_file` without a path was refused.
   - File ▸ Save opened the native Save panel suggesting `Festival.scrim` beside it. The suggestion was read from the app's log, which names the file only. The panel itself is outside the window's own captures.
   - The `.figcraft` kept its bytes.
4. **The Sketch sample, opened at launch,** became a new unsaved document. Its report counted "3 left out", each with its layer (`Poster / Poster / Call to action`, `Tap area`, `Hologram`), with Copy report. MCP `open_file` returned the same three warnings, and the sample kept its bytes.
5. **Home after** read "Open .scrim, .fig, .sketch, .svg or images" and listed the recent files under their names.

No panics; both runs quit with 0. Not checked live: a Finder double-click on an installed app (the bundle isn't installed); the plist the bundle carries was read with `plutil` (process notes).

**A gap found by the check, not fixed:** Home's recent-file card for a `.sketch` shows no picture. The preview reader reads `.scrim`, `.figcraft`, `.fig` and images, not Sketch's `previews/preview.png`.
