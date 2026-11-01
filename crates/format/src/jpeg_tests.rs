//! A crafted progressive JPEG can't crash the app (upstream FigCraft 9aac391e and 0971d096): such
//! files are refused from their header, and a decode that fails is an image error, never a panic.

use crate::jpeg::{MAX_SCANS, refusal};

/// A progressive 8 × 8 JPEG whose second component is sampled 2 × 2 under a 1 × 1 first one:
/// zune-jpeg 0.5.15 (behind `image`) asserts in its upsampler on it (zune-image#236). Built here,
/// byte by byte: one quantisation table, one DC Huffman table with a single code, and `scans` DC
/// scans whose six blocks are all zero.
pub(crate) fn finer_sampled_progressive(scans: usize) -> Vec<u8> {
    progressive(0x22, scans)
}

/// [`finer_sampled_progressive`] with Cb sampled `cb` (`0x11`: as the others).
fn progressive(cb: u8, scans: usize) -> Vec<u8> {
    let mut b = vec![0xFF, 0xD8];
    // DQT: table 0, all ones.
    b.extend([0xFF, 0xDB, 0x00, 0x43, 0x00]);
    b.extend([1u8; 64]);
    // SOF2 (progressive): 8 bits, 8 × 8, three components: Y 1×1, Cb 2×2, Cr 1×1, all table 0.
    b.extend([0xFF, 0xC2, 0x00, 0x11, 0x08, 0x00, 0x08, 0x00, 0x08, 0x03]);
    b.extend([0x01, 0x11, 0x00, 0x02, cb, 0x00, 0x03, 0x11, 0x00]);
    // DHT: DC table 0 with one code of length 1, for category 0.
    b.extend([0xFF, 0xC4, 0x00, 0x14, 0x00, 0x01]);
    b.extend([0u8; 15]);
    b.push(0x00);
    for _ in 0..scans {
        // SOS: all three components, DC only (Ss 0, Se 0, Ah 0, Al 0); six zero codes, padded.
        b.extend([0xFF, 0xDA, 0x00, 0x0C, 0x03, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00]);
        b.push(0x03);
    }
    b.extend([0xFF, 0xD9]);
    b
}

#[test]
fn a_crafted_progressive_jpeg_gives_no_thumbnail_and_no_panic() {
    let bytes = finer_sampled_progressive(1);
    let got = std::panic::catch_unwind(|| crate::thumbnail::decode_thumbnail(&bytes, 64));
    assert!(matches!(got, Ok(None)), "the thumbnail decode panicked or decoded: {:?}", got.map(|t| t.is_some()));
}

#[test]
fn a_crafted_progressive_jpeg_is_refused_as_an_image() {
    let mut d = figcraft_doc::Document::new("x");
    let e = crate::add_image(&mut d, finer_sampled_progressive(1)).expect_err("the image is refused");
    assert!(e.to_string().contains("JPEG"), "{e}");
    assert!(d.images.is_empty());
}

#[test]
fn the_jpeg_check_reads_headers_only_and_refuses_what_the_decoder_cant_take() {
    assert!(refusal(&finer_sampled_progressive(1)).is_some());
    // Too many scans: each is a pass over the whole picture.
    assert!(refusal(&progressive(0x11, MAX_SCANS + 1)).is_some_and(|r| r.contains("scans")));
    assert!(refusal(&progressive(0x11, MAX_SCANS)).is_none());
    // Not JPEGs, and a truncated one, are left to the decoders.
    assert!(refusal(b"\x89PNG\r\n\x1a\n").is_none());
    assert!(refusal(&[0xFF, 0xD8, 0xFF]).is_none());
    assert!(refusal(&[]).is_none());
    // A baseline frame of the same shape is the decoder's to read.
    let mut baseline = finer_sampled_progressive(1);
    let at = baseline.windows(2).position(|w| w == [0xFF, 0xC2]).unwrap();
    baseline[at + 1] = 0xC0;
    assert!(refusal(&baseline).is_none());
}
