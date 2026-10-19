# Rebrand: FigCraft → Scrim (Stage A)

The decisions: `docs/design/rebrand.md`.

| Check | How | Covered by |
|---|---|---|
| No interface string, in any language, names FigCraft or ArtCraft except the attribution line | Switch the language (Preferences ▸ Language) and look at Home, About, menus and the Share dialog | `crates/ui-egui/src/i18n_brand_tests.rs` › `no_catalog_string_names_the_old_product_outside_the_attribution` |
| About says "Scrim is based on FigCraft by the ArtCraft team" | Help ▸ About Scrim | `i18n_brand_tests.rs` › `the_attribution_line_is_there_and_credits_upstream` |
| The window title, Home and the help menu say Scrim | Open the app, open a file, click ? | Manual (screenshots in the A1 commit) |
| The app is Scrim to the system: `Scrim.app`, bundle id `design.scrim.app`, the Linux desktop entry and Flatpak id, deb and rpm `scrim`, installers `scrim-…` | Build a package (`packaging/*/package.sh`) and look at the name, the Dock or task bar, and the window class | `apps/figcraft/src/migrate_tests.rs` › `the_window_app_id_matches_the_packaging`; `packaging/check-app-specific.sh` (CI's packaging lint) |
| First start copies FigCraft's settings (preferences, recent files, recovery copies, plugins) into a new Scrim directory, never moving or deleting them; a fresh install and a second start copy nothing | Start Scrim on a machine with FigCraft's settings: the recent files and theme are there, and FigCraft still opens with its own | `migrate_tests.rs` › `an_upgrade_copies_figcrafts_settings_and_leaves_them_in_place`, `a_fresh_install_has_nothing_to_copy`, `with_both_directories_scrims_wins_and_nothing_is_copied` |
| The app icon and the in-app mark are Scrim's placeholder S, not FigCraft's chameleon | Look at the Dock, Home and About | `xtask/src/icon.rs` › `the_placeholder_is_drawn_from_code_and_names_the_app_id` |
