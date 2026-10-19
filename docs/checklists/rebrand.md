# Rebrand: FigCraft → Scrim (Stage A)

The decisions: `docs/design/rebrand.md`.

| Check | How | Covered by |
|---|---|---|
| No interface string, in any language, names FigCraft or ArtCraft except the attribution line | Switch the language (Preferences ▸ Language) and look at Home, About, menus and the Share dialog | `crates/ui-egui/src/i18n_brand_tests.rs` › `no_catalog_string_names_the_old_product_outside_the_attribution` |
| About says "Scrim is based on FigCraft by the ArtCraft team" | Help ▸ About Scrim | `i18n_brand_tests.rs` › `the_attribution_line_is_there_and_credits_upstream` |
| The window title, Home and the help menu say Scrim | Open the app, open a file, click ? | Manual (screenshots in the A1 commit) |
