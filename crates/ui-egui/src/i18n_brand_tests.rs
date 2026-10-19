//! The product is Scrim (`docs/design/rebrand.md`): no catalog string, in any language, names
//! FigCraft or ArtCraft except the attribution lines the licences and brand terms ask for.

use super::*;

/// The attribution lines, by English key: "Scrim is based on FigCraft by the ArtCraft team".
const ATTRIBUTION: &[&str] = &["Scrim is based on FigCraft by the ArtCraft team; it is not an ArtCraft product. Dual-licensed MIT OR Apache-2.0."];

const OLD_NAMES: &[&str] = &["FigCraft", "ArtCraft"];

fn texts(t: &Text) -> Vec<&'static str> {
    match t {
        Text::Plain(s) => vec![s],
        Text::Plural { one, few, many, other } => [*one, *few, *many, Some(*other)].into_iter().flatten().collect(),
    }
}

#[test]
fn no_catalog_string_names_the_old_product_outside_the_attribution() {
    let mut bad = Vec::new();
    for lang in LANGS {
        let file = format!("crates/ui-egui/locales/{}.toml", lang.code);
        for (key, t) in lang.ui.iter().chain(lang.commands).chain(lang.menu) {
            if ATTRIBUTION.contains(key) {
                continue;
            }
            for s in std::iter::once(*key).chain(texts(t)) {
                if OLD_NAMES.iter().any(|n| s.contains(n)) {
                    bad.push(format!("{file}: {s:?}"));
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "the old product name in a user-visible string (rename it to Scrim, or add an attribution line to ATTRIBUTION):\n  {}",
        bad.join("\n  ")
    );
}

#[test]
fn the_attribution_line_is_there_and_credits_upstream() {
    let en = LANGS.first().unwrap();
    for key in ATTRIBUTION {
        assert!(en.ui.iter().any(|(k, _)| k == key), "{key}");
        assert!(key.contains("based on FigCraft by the ArtCraft team"));
    }
}
