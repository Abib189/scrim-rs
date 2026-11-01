# Lost-work fixes from upstream (Fix AI stage 2b)

`docs/design/upstream-sync.md` › Ported from upstream. Each row is a bug that lost or corrupted a person's work, and the file in the damaged shape it must still open.

| Check | How | Covered by |
|---|---|---|
| Page ruler guides survive save and reopen | Add ruler guides to a page and a frame, save, close, reopen: the guides are there; add another, save and reopen again: the file opens with all of them | `crates/format/src/page_guides_tests.rs` › `page_guides_survive_save_and_reopen_twice`; `crates/doc/src/guides_tests.rs` |
| A file saved with the page's guides twice opens | Open a file whose page has the `guides` key twice (saved by an earlier build after a reopen): it opens with both lists, the file's bytes are unchanged, and its next save opens | `page_guides_tests.rs` › `a_file_saved_with_the_page_guides_key_twice_opens_with_its_guides_and_saves_a_valid_file`; `guides_tests.rs` › `a_page_written_with_the_key_twice_opens_with_both_lists` |
