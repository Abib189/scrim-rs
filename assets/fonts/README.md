# Bundled fonts

Scrim embeds these families in the desktop app (`crates/text/src/bundled.rs`), so files that use them render correctly without installing anything. The web build doesn't embed them (about 3.8 MB); the browser uses its fallback there. Scrim never downloads fonts.

These are the one exception to "fonts live in craft-fonts, never in this repo" (AGENTS.md): the owner asked for them in Stage C (`docs/design/settings-and-connect.md`).

| Family | Files | Version | Upstream | Licence | SHA-256 |
|---|---|---|---|---|---|
| Inter | `inter/InterVariable.ttf`, `inter/InterVariable-Italic.ttf` | 4.001 (git-9221beed3) | [rsms/inter](https://github.com/rsms/inter), © 2016 The Inter Project Authors | SIL OFL 1.1, [`inter/OFL.txt`](inter/OFL.txt) | `4989b125…ccaf`, `d6f1f6a1…586f` |
| Source Serif 4 | `source-serif-4/SourceSerif4-Variable.ttf`, `source-serif-4/SourceSerif4-Italic-Variable.ttf` | 4.004 | [adobe-fonts/source-serif](https://github.com/adobe-fonts/source-serif), © 2014–2021 Adobe, Reserved Font Name "Source" | SIL OFL 1.1, [`source-serif-4/OFL.txt`](source-serif-4/OFL.txt) | `97b2d4da…ca0b`, `15fbc7e4…1f42` |
| Geist | `geist/Geist-Variable.ttf` | 1.401 | [vercel/geist-font](https://github.com/vercel/geist-font), © 2024 The Geist Project Authors | SIL OFL 1.1, [`geist/OFL.txt`](geist/OFL.txt) | `b1727819…005d` |

**Where the files came from** (20 Oct 2026, the owner's Mac; nothing downloaded):
- **Inter:** `~/Library/Fonts/InterVariable.ttf` and `InterVariable-Italic.ttf`, the variable fonts of the upstream 4.001 release (the version string in their `name` table). The licence is upstream's `OFL.txt`, from a copy shipped with Inter in another of the owner's projects.
- **Source Serif 4:** `~/Library/Fonts/SourceSerif4[opsz,wght].ttf` and `SourceSerif4-Italic[opsz,wght].ttf`, renamed here without brackets. The licence file combines the copyright line from the fonts' own `name` table (ID 0) with the standard OFL 1.1 text.
- **Geist:** the Google Fonts download of Geist (`Geist-VariableFont_wght.ttf`, with its `OFL.txt`), byte-identical to the copy installed in `~/Library/Fonts`.

The fonts are unmodified (same bytes; the SHA-256 above are of the files here). Under the OFL they may be bundled and redistributed with software, with their copyright and licence. A modified version could not keep a Reserved Font Name such as "Source".

**How they're used:**
- They're registered under their family names ("Inter", "Source Serif 4", "Geist"), with each variable font's named instances as the family's styles.
- The Missing fonts window never lists them.
- An installed copy of the same family is used alongside them.

To update one, replace the file and its row (version, SHA-256).
