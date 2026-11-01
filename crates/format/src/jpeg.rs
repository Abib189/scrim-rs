//! JPEGs checked from their markers before any pixel is decoded (upstream FigCraft 9aac391e and
//! 0971d096, made one check here for every decoder in Scrim). A JPEG the decoder can't take
//! safely is refused like any unreadable image:
//!
//! - **Sampling.** A progressive frame whose later component is sampled more finely, across or
//!   down, than its first: zune-jpeg 0.5.15, the decoder behind `image`, asserts in its upsampler
//!   on it (zune-image#236). Its fix isn't released yet. In the browser a panic aborts the page.
//! - **Scans.** More than [`MAX_SCANS`] scans: each is another pass over the whole picture, so a
//!   small file could keep the decoder busy for minutes. Real files have about ten.
//!
//! Size and memory are bounded by each decoder's own limits (`image::Limits`), and a decode that
//! fails anyway is an image error, never a panic (`decode_thumbnail`, `figcraft_render::images`).

/// Most scans a JPEG may have.
pub const MAX_SCANS: usize = 256;

/// Why `bytes`, a JPEG, are refused before decoding; `None` for anything else, a JPEG the
/// decoders may read, or one too damaged to say (they refuse those themselves).
pub fn refusal(bytes: &[u8]) -> Option<String> {
    if !bytes.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut at = 2;
    let mut scans = 0usize;
    let mut framed = false;
    loop {
        if *bytes.get(at)? != 0xFF {
            return None;
        }
        let marker = *bytes.get(at + 1)?;
        match marker {
            // Fill bytes, and markers without a length.
            0xFF => at += 1,
            0x01 | 0xD0..=0xD8 => at += 2,
            0xD9 => return None,
            _ => {
                let len = usize::from(u16::from_be_bytes([*bytes.get(at + 2)?, *bytes.get(at + 3)?]));
                let end = at.checked_add(2)?.checked_add(len)?;
                let segment = bytes.get(at + 4..end)?;
                if is_frame(marker) && !framed {
                    framed = true;
                    if is_progressive(marker) && finer_than_first(segment) {
                        return Some("a progressive JPEG sampled in a way its decoder can't read".into());
                    }
                }
                at = end;
                if marker == 0xDA {
                    scans += 1;
                    if scans > MAX_SCANS {
                        return Some(format!("a JPEG with more than {MAX_SCANS} scans"));
                    }
                    at = after_scan(bytes, at)?;
                }
            }
        }
    }
}

/// A start-of-frame marker (not DHT C4, JPG C8 or DAC CC).
fn is_frame(m: u8) -> bool {
    matches!(m, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF)
}

fn is_progressive(m: u8) -> bool {
    matches!(m, 0xC2 | 0xC6 | 0xCA | 0xCE)
}

/// Whether a frame's `segment` (after its length: precision, height, width, the component count,
/// then id, sampling and table of each) samples a later component more finely than its first.
fn finer_than_first(segment: &[u8]) -> bool {
    let n = usize::from(segment.get(5).copied().unwrap_or(0));
    let sampling = |k: usize| segment.get(7 + 3 * k).map(|b| (b >> 4, b & 0x0F));
    let Some((h0, v0)) = sampling(0) else { return false };
    (1..n).any(|k| sampling(k).is_some_and(|(h, v)| h > h0 || v > v0))
}

/// Where the marker after a scan's entropy-coded data starts: a 0xFF there is followed by 0x00
/// (a stuffed byte), a restart marker or more 0xFF fill; anything else ends the data.
fn after_scan(bytes: &[u8], mut at: usize) -> Option<usize> {
    loop {
        let ff = bytes.get(at..)?.iter().position(|b| *b == 0xFF)?;
        at += ff;
        match *bytes.get(at + 1)? {
            0x00 | 0xD0..=0xD7 => at += 2,
            0xFF => at += 1,
            _ => return Some(at),
        }
    }
}
