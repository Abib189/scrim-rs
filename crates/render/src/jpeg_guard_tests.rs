//! A crafted progressive JPEG draws nothing instead of crashing the renderer (upstream FigCraft
//! 9aac391e): the canvas, exports and PDF decode images here.

use figcraft_color::ColorProfile;

/// A progressive 8 × 8 JPEG with Cb sampled 2 × 2 under a 1 × 1 Y, which zune-jpeg 0.5.15 asserts
/// on (built as `figcraft_format`'s JPEG tests build it).
fn finer_sampled_progressive() -> Vec<u8> {
    let mut b = vec![0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x43, 0x00];
    b.extend([1u8; 64]);
    b.extend([0xFF, 0xC2, 0x00, 0x11, 0x08, 0x00, 0x08, 0x00, 0x08, 0x03, 0x01, 0x11, 0x00, 0x02, 0x22, 0x00, 0x03, 0x11, 0x00]);
    b.extend([0xFF, 0xC4, 0x00, 0x14, 0x00, 0x01]);
    b.extend([0u8; 16]);
    b.extend([0xFF, 0xDA, 0x00, 0x0C, 0x03, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x03, 0xFF, 0xD9]);
    b
}

#[test]
fn a_crafted_progressive_jpeg_decodes_to_nothing_without_a_panic() {
    let bytes = finer_sampled_progressive();
    let got = std::panic::catch_unwind(|| crate::images::decode(&bytes).is_some());
    assert!(matches!(got, Ok(false)), "decode panicked or gave pixels: {got:?}");
    let got = std::panic::catch_unwind(|| crate::images::decode_within(&bytes, ColorProfile::Srgb, 1024, 1 << 20).is_some());
    assert!(matches!(got, Ok(false)), "{got:?}");
}
