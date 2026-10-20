//! Fonts bundled with the desktop app (`assets/fonts/`, SIL OFL 1.1): Inter, Source Serif 4 and
//! Geist, so files that use them render without installing anything. Registered under their family
//! names, after any craft-fonts build input and only for a family it doesn't already supply. The
//! web build embeds none (about 3.8 MB); the browser uses its fallback there.

use crate::fontdb::CraftFont;

macro_rules! font {
    ($family:expr, $style:expr, $path:expr) => {
        CraftFont { family: $family, style: $style, bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/fonts/", $path)) }
    };
}

/// The bundled faces (variable fonts: their named instances are the family's styles).
#[cfg(not(target_arch = "wasm32"))]
pub static BUNDLED_FONTS: &[CraftFont] = &[
    font!("Inter", "Regular", "inter/InterVariable.ttf"),
    font!("Inter", "Italic", "inter/InterVariable-Italic.ttf"),
    font!("Source Serif 4", "Regular", "source-serif-4/SourceSerif4-Variable.ttf"),
    font!("Source Serif 4", "Italic", "source-serif-4/SourceSerif4-Italic-Variable.ttf"),
    font!("Geist", "Regular", "geist/Geist-Variable.ttf"),
];

#[cfg(target_arch = "wasm32")]
pub static BUNDLED_FONTS: &[CraftFont] = &[];

/// The families Scrim bundles (empty in the browser).
pub fn bundled_families() -> Vec<&'static str> {
    let mut v: Vec<&str> = BUNDLED_FONTS.iter().map(|f| f.family).collect();
    v.dedup();
    v
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use crate::{family_styles, find_face, has_family};

    /// Each bundled family resolves by name with its variable font's styles, without any installed
    /// font (tests don't index the system's).
    #[test]
    fn the_bundled_families_resolve_with_their_styles() {
        for (family, styles) in [
            ("Inter", &["Regular", "SemiBold", "Bold", "Italic"][..]),
            ("Source Serif 4", &["Regular", "Bold", "Italic"][..]),
            ("Geist", &["Regular", "Medium", "Bold"][..]),
        ] {
            assert!(has_family(family), "{family} is bundled");
            let have = family_styles(family);
            for s in styles {
                assert!(have.iter().any(|h| h.eq_ignore_ascii_case(s)), "{family} {s} in {have:?}");
                let face = find_face(family, s).unwrap_or_else(|| panic!("{family} {s}"));
                assert_eq!(face.family, family);
            }
        }
        assert!(find_face("Inter", "Italic").is_some_and(|f| f.italic), "the italic file serves italics");
        // Files often spell the instance with a space.
        assert!(find_face("Inter", "Semi Bold").is_some_and(|f| !f.italic), "Semi Bold resolves");
        assert_eq!(super::bundled_families(), ["Inter", "Source Serif 4", "Geist"]);
    }
}
