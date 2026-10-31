//! The app icon: Scrim's mark (`docs/design/rebrand.md` › The app icon; the brand kit and its usage
//! guide are the owner's, outside the repository). Two identical pieces interlock into one shape, a
//! person and an agent on one surface: on the olive tile, the top piece in white and the bottom one
//! in ink, 58% of the tile wide and nudged up 1.5% for the optical centre. The pieces' outlines are
//! the owner's drawing, copied here unchanged.
//! `cargo xtask icon` writes `assets/app-icon/scrim/`: the SVG, scrim-<size>.png (macOS grid), the
//! hicolor theme (`design.scrim.app`) and scrim.ico, rasterised with the `resvg` CLI
//! (`cargo install resvg`) or `rsvg-convert`. The outputs are committed, so building and packaging
//! never need either. FigCraft's chameleon in `assets/app-icon/` stays in the repository, unshipped.

use std::path::Path;
use std::process::Command;

/// The app id the icons are named for.
pub const APP_ID: &str = "design.scrim.app";

/// Brand colours (the brand kit's usage guide).
const OLIVE: &str = "#7f7500";
const INK: &str = "#111214";

/// The mark's two pieces, each a transform and an outline in the drawing's own units (its box is
/// 150 × 141 at 4, 4): the ink piece, then the top piece.
const PIECES: [(&str, &str); 2] = [
    (
        "matrix(-1 0 0 -1 153.745 145)",
        "M50.153 0L139.745 0C145.268 0 149.745 4.477 149.745 10L149.745 35C149.745 40.523 145.268 45 139.745 45L59.871 45C54.316 45 49.826 49.526 49.871 55.081L50.071 79.831C50.116 85.42 45.573 89.961 39.983 89.92C31.461 89.859 19.324 89.785 10.612 89.788C4.663 89.79 0 84.81 0 78.862L0 10C0 4.477 4.477 0 10 0L50.153 0Z",
    ),
    (
        "matrix(1 0 0 1 4.254 4)",
        "M50.077 0L139.746 0C145.269 0 149.746 4.477 149.746 10L149.746 35C149.746 40.523 145.269 45 139.746 45L59.796 45C54.241 45 49.751 49.526 49.796 55.081L49.995 79.831C50.04 85.42 45.497 89.96 39.908 89.92C31.405 89.859 19.307 89.786 10.614 89.788C4.665 89.79 0 84.81 0 78.86L0 10C0 4.477 4.477 0 10 0L50.077 0Z",
    ),
];

/// The icon as SVG: the tile edge to edge (`view_box` pads it for macOS).
pub fn svg(view_box: &str) -> String {
    let [(ink_t, ink_d), (top_t, top_d)] = PIECES;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view_box}" width="512" height="512">
  <rect x="0" y="0" width="512" height="512" rx="112" fill="{OLIVE}"/>
  <g transform="translate(99.60 100.83) scale(1.9797)">
    <path transform="{ink_t}" d="{ink_d}" fill="{INK}"/>
    <path transform="{top_t}" d="{top_d}" fill="#ffffff"/>
  </g>
</svg>
"##
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
    fn the_mark_is_drawn_from_code_and_names_the_app_id() {
        let s = svg("0 0 512 512");
        // The olive tile, the ink piece and the white piece, both outlines present.
        assert!(s.contains(r##"fill="#7f7500""##) && s.contains(r##"fill="#111214""##) && s.contains(r##"fill="#ffffff""##), "{s}");
        assert_eq!(s.matches("<path ").count(), 2, "{s}");
        // Committed outputs match the generator.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let committed = std::fs::read_to_string(root.join("assets/app-icon/scrim/scrim.svg")).unwrap();
        assert_eq!(committed, s, "run `cargo xtask icon` after changing the mark");
        assert!(root.join(format!("assets/app-icon/scrim/hicolor/256x256/apps/{APP_ID}.png")).exists());
    }
}
