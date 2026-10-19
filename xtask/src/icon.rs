//! The app icon: Scrim's **placeholder** mark, drawn here in code (it awaits the owner's real one;
//! `docs/design/rebrand.md` › The placeholder icon). The same bold S the interface draws
//! (`crates/ui-egui/src/icons.rs` › `Icon::Logo`), in ink on the accent orange, on a rounded tile.
//! `cargo xtask icon` writes `assets/app-icon/scrim/`: the SVG, scrim-<size>.png (macOS grid), the
//! hicolor theme (`design.scrim.app`) and scrim.ico, rasterised with the `resvg` CLI
//! (`cargo install resvg`) or `rsvg-convert`. The outputs are committed, so building and packaging
//! never need either. FigCraft's chameleon in `assets/app-icon/` stays in the repository, unshipped.

use std::path::Path;
use std::process::Command;

/// The app id the icons are named for.
pub const APP_ID: &str = "design.scrim.app";

/// The 16-unit glyph scaled onto the 512 tile: x, y → 96 + 20·(x, y).
fn s_path() -> String {
    let (cx, r) = (256.0, 60.0);
    let pt = |cy: f64, deg: f64| (cx + r * deg.to_radians().cos(), cy + r * deg.to_radians().sin());
    // Top bowl: centre (8, 5) from −40° anticlockwise to the bottom; bottom bowl: centre (8, 11)
    // from the top clockwise to 140°.
    let (top, bottom) = (196.0, 316.0);
    let (a, b, c) = (pt(top, -40.0), pt(top, 90.0), pt(bottom, 140.0));
    format!("M {:.2} {:.2} A {r} {r} 0 1 0 {:.2} {:.2} A {r} {r} 0 1 1 {:.2} {:.2}", a.0, a.1, b.0, b.1, c.0, c.1)
}

/// The placeholder icon as SVG: the tile edge to edge (`view_box` pads it for macOS).
pub fn svg(view_box: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view_box}" width="512" height="512">
  <rect x="0" y="0" width="512" height="512" rx="112" fill="#ff5a1f"/>
  <path d="{}" fill="none" stroke="#111214" stroke-width="44" stroke-linecap="round"/>
</svg>
"##,
        s_path()
    )
}

fn render(svg: &Path, size: u32, out: &Path) -> bool {
    let s = size.to_string();
    let resvg = Command::new("resvg").args(["-w", &s, "-h", &s]).arg(svg).arg(out).status().is_ok_and(|st| st.success());
    let ok = resvg || Command::new("rsvg-convert").args(["-w", &s, "-h", &s, "-o"]).arg(out).arg(svg).status().is_ok_and(|st| st.success());
    if !ok {
        eprintln!("icon: rendering {} failed (install `resvg` or `rsvg-convert`)", out.display());
    }
    ok
}

pub fn generate(root: &Path) -> bool {
    let dir = root.join("assets/app-icon/scrim");
    let tmp = root.join("target/icon");
    let (tile, mac) = (dir.join("scrim.svg"), tmp.join("mac.svg"));
    let mut ok = std::fs::create_dir_all(&dir).is_ok()
        && std::fs::create_dir_all(&tmp).is_ok()
        && std::fs::write(&tile, svg("0 0 512 512")).is_ok()
        // macOS: the tile on Apple's 824/1024 icon grid (transparent margin).
        && std::fs::write(&mac, svg("-62 -62 636 636")).is_ok();
    if !ok {
        eprintln!("icon: cannot write {}", dir.display());
        return false;
    }
    for size in [16u32, 32, 48, 64, 128, 256, 512, 1024] {
        ok &= render(&mac, size, &dir.join(format!("scrim-{size}.png")));
        if size <= 512 {
            let hd = dir.join(format!("hicolor/{size}x{size}/apps"));
            let _ = std::fs::create_dir_all(&hd);
            ok &= render(&tile, size, &hd.join(format!("{APP_ID}.png")));
        }
    }
    // Windows .ico (PNG-compressed entries).
    let sizes = [16u32, 20, 24, 32, 40, 48, 64, 128, 256];
    let pngs: Vec<Vec<u8>> = sizes
        .iter()
        .filter_map(|s| {
            let png = tmp.join(format!("ico-{s}.png"));
            render(&tile, *s, &png).then(|| std::fs::read(png).ok()).flatten()
        })
        .collect();
    ok &= pngs.len() == sizes.len();
    let mut ico = vec![0, 0, 1, 0, pngs.len() as u8, 0];
    let mut offset = 6 + 16 * pngs.len() as u32;
    for (s, png) in sizes.iter().zip(&pngs) {
        let b = if *s >= 256 { 0 } else { *s as u8 };
        ico.extend_from_slice(&[b, b, 0, 0, 1, 0, 32, 0]);
        ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for png in &pngs {
        ico.extend_from_slice(png);
    }
    ok &= std::fs::write(dir.join("scrim.ico"), ico).is_ok();
    let sd = dir.join("hicolor/scalable/apps");
    let _ = std::fs::create_dir_all(&sd);
    ok &= std::fs::write(sd.join(format!("{APP_ID}.svg")), svg("0 0 512 512")).is_ok();
    eprintln!("icon: wrote {}", dir.display());
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_placeholder_is_drawn_from_code_and_names_the_app_id() {
        let s = svg("0 0 512 512");
        assert!(s.contains("#ff5a1f") && s.contains(" A 60 60 0 1 0 ") && s.contains(" A 60 60 0 1 1 "), "{s}");
        // Committed outputs match the generator.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let committed = std::fs::read_to_string(root.join("assets/app-icon/scrim/scrim.svg")).unwrap();
        assert_eq!(committed, s, "run `cargo xtask icon` after changing the mark");
        assert!(root.join(format!("assets/app-icon/scrim/hicolor/256x256/apps/{APP_ID}.png")).exists());
    }
}
