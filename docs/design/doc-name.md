# One name for a document (Fix AF, 31 Oct 2026)

**Before:** a file opened from disk could go by two names. Home's recent-file card showed the file name ("zeichen-26"); the tab, the left panel's header, the window title, the Save as dialog, the export dialogs, `ui.inspect`, `document.inspect` and the MCP `open_file` result showed the name stored in the document ("Untitled").

The cause: every document carries a stored `name`, and nothing ever replaced the placeholder.
- `file.new` stores "Untitled" when no name is given. An agent's `file.new` followed by `save_file` (or Save as in the app) writes that placeholder into the file: saving never touched the name.
- Opening a file filled in the file name only when the stored name was *empty*, so "Untitled" stayed "Untitled".
- Only a rename (File ▸ Rename, the panel header, `file.rename`) ever set the name.
- Each place read the name in its own way:
  - Home's cards and list (`home_recents.rs`) took the file's stem;
  - Home's open files, the tab (`tabs.rs`), the header (`layers.rs`), the title (`windows.rs`), the Save as and export dialogs, `ui.inspect`, `document.inspect` and `file.open`'s summary took the stored name;
  - File ▸ Open recent and `file.openRecent` took the file name with its extension.

**Now:** one rule, `crates/engine/src/doc_name.rs` (`DocState::name`, `doc_name::of_file`), shown through `crates/ui-egui/src/doc_title.rs` with "Untitled" translated:
1. **A name someone gave** (a rename, `file.new {name}`, the name in a file from another tool) is used. A stored name of exactly "Untitled", empty or only spaces, counts as not set.
2. **Otherwise the file's name** without its extension.
3. **Otherwise "Untitled"**, and from the second unsaved document on, "Untitled 2", "Untitled 3". A tab keeps its number while it stays untitled; a new one takes the lowest number free.

Every place in the brief reads it:
- the tab, the left panel's header and the window title;
- Home: the open files, the recent-file cards and the list, its search and its sort by name;
- File ▸ Open recent and `file.openRecent`;
- the Save as dialog's suggested name, and the PDF, `.fig` and variables exports' suggested names;
- `file.open`'s summary (MCP `open_file`), `document.inspect` (MCP `inspect_document`, `open_file`'s `document`), `ui.inspect` (tabs, windows, Home's recent files), the status summary;
- the names derived from it: Duplicate file ("zeichen-26 (Copy)"), a version opened as a file, the branch banner, the Live publish form, the import banner and the AI chat list.

Details:
- **Save as** of a document never saved: its name becomes the new file's name at once, in the tab, the panel and Home, without reopening. Saved again under another name, it follows that file.
- **Renaming** in the panel (or File ▸ Rename, `file.rename`) sets the document's own name. It doesn't rename the file on disk, and the file isn't written until it is saved. Renaming a file in Finder doesn't rename the document.
- **Opening** reads the name every time. It never writes it: opening doesn't change a file's bytes.
- **Recent files that aren't open** go by the name stored in the file. Home's reader thread reads it with the file's dates, again only when the file has changed since. `figcraft_format::stored_name` reads only the first 64 KB of `document.json` (Scrim writes the name after the format, the version and the image index) or a `.fig` file's `meta.json`, skipping over the keys before the name without parsing them, so listing twenty files stays quick. An open file goes by its open document's name, an unsaved rename included.
- **Long names** end in "…" in the tab and the panel header, with the whole name in a tooltip.
- **Spanish:** "Untitled" is "Sin título", "Untitled 2" is "Sin título 2" (`es.toml`; `en.toml` has both keys).

## Decisions (overrule them)

- **Saving never writes the name.** A document still named with the placeholder keeps it in the file, so the file goes by its file name wherever it is saved, copied or moved. Only a rename writes a name. Saved after a rename, the file stores that name.
- **Only exactly "Untitled" is the placeholder.** "untitled", "Untitled 2" or "Sin título" stored in a file are names someone gave.
- **The number is per window session**, not stored. "Untitled 2" closed and reopened from Recover file is numbered again.
- **What agents are told is in English** (`file.open`, `document.inspect`, `file.openRecent`). `ui.inspect` reports what the window shows, so in Spanish an unsaved tab reads "Sin título" there. For a file opened from disk, both report the same name.
- **A REST API file JSON or `.fig` import keeps its own name** (the Figma file's name, else "Imported file" as before). These open as unsaved documents with no file of their own.
- **A file whose name comes after its first 64 KB** (a large REST API file JSON, which keeps its name after the whole document) is listed in the recent files under its file name, not its stored name: only the start of a file is read for its name.
- **The tooltip for a cut name** now applies to every inline name field that uses the shared field (`widgets::text_field_inline`): the file header, and any other name that ends in "…".
- **Publishing a library still uses the stored name** (`library.rs`): a published library's name belongs to the library and wasn't in the brief.

## Tests

- `crates/engine/src/doc_name_tests.rs` (9 tests):
  - the rule;
  - a file storing "Untitled" goes by its file name in `file.open`'s summary, `document.inspect` and the status, and opening leaves its bytes alone, also from Open recent in a fresh session;
  - a given name wins;
  - "Untitled", "Untitled 2", numbers kept and reused;
  - Save as names after the file and writes no name;
  - rename doesn't touch the file on disk, and undo restores;
  - Duplicate file's name;
  - the recent files list;
  - old JSON files and REST API imports still open.
- `crates/format/src/stored_name_tests.rs` (4 tests): reading the stored name from `.figcraft` (zip and bare JSON), JSON, `.fig`; nothing for other kinds or damaged files.
- `crates/ui-egui/src/doc_name_ui_tests.rs` (7 tests, through the window):
  - the tab, the header, the title, `ui.inspect` and Home for a file storing "Untitled" and for one with a given name;
  - "Untitled 2" and the Spanish strings;
  - Save as;
  - a rename typed in the header;
  - Home's search and sort by name;
  - long names cut with "…" and shown whole in the tab's and the header's tooltips.
- `home_recents_tests.rs`' fixture files now store the placeholder name, so they still go by their file names.

## Live check (31 Oct 2026)

A real window: the debug app with a scratch `HOME` and `SCRIM_SETTINGS_DIR`, MCP on 7690 and control on 7979. Text went through the window's own input (`ui.input`); the system pointer was never moved. The script, its output and the captures are in `doc-name/` (`live.py`, `live-output.txt`, `captures.png`).
1. **The file:** a poster made with `file.new` and saved without a name, as an agent makes one. It stores "Untitled".
2. **Home, the file closed:** its card reads "zeichen-26".
3. **Opened through MCP `open_file`:**
   - `opened.summary.name` and `document.name` are "zeichen-26";
   - so are the tab, the left panel's header and the window title ("zeichen-26 – Scrim");
   - the file's bytes are unchanged.
4. **Save as "Festival poster":**
   - the tab, the header, the title and Home read "Festival poster" at once;
   - the new file stores "Untitled", and zeichen-26 is untouched.
5. **File ▸ Rename, typed in the header** ("Spring festival poster, final, for print at A1 size"):
   - the tab and the header end in "…", and the title and Home read it whole;
   - the files on disk keep their names and bytes;
   - saved, the file stores the new name under its old file name.
6. No panics; the app quit with 0.
