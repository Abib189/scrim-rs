//! What an archive Scrim opens may unpack to (Fix AI-3). Every zip reader here (the `.scrim` and
//! `.figcraft` file, `.fig`, Sketch, Penpot and FREE) goes through these checks, so a small file
//! built to unpack into gigabytes is refused with a reason before it fills memory:
//!
//! - **Parts.** At most [`MAX_ENTRIES`] members (checked when the archive is opened).
//! - **Ratio.** A member may unpack to at most [`MAX_RATIO`] times its packed size past its first
//!   [`RATIO_FREE`] bytes. Deflate packs honest JSON ten to fifty times and pictures hardly at
//!   all, while a bomb packs about a thousand times. The ratio is checked against what the archive
//!   says and against the bytes actually read, so a lying header can't get past it.
//! - **Size.** Each member has its reader's own cap ([`crate::read_member`]'s `limit`), and an
//!   archive may say it unpacks to at most [`MAX_RATIO`] times its own size (from [`TOTAL_FREE`],
//!   up to [`MAX_TOTAL`]). Each reader's running budgets bound what it reads in all.

use std::fmt;

use crate::FormatError;

/// Most members an archive may have.
pub const MAX_ENTRIES: usize = 50_000;
/// Most a member may unpack to, per byte it packs to, past [`RATIO_FREE`].
pub const MAX_RATIO: u64 = 250;
/// What any member may unpack to, whatever its ratio.
pub const RATIO_FREE: u64 = 16 << 20;
/// What any archive may unpack to in all, whatever its size.
pub const TOTAL_FREE: u64 = 64 << 20;
/// Most an archive may unpack to in all.
#[cfg(not(target_pointer_width = "32"))]
pub const MAX_TOTAL: u64 = 8 << 30;
#[cfg(target_pointer_width = "32")]
pub const MAX_TOTAL: u64 = 2 << 30;

/// Why an archive was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unpack {
    /// It has `count` members, more than [`MAX_ENTRIES`].
    Entries { count: usize },
    /// Member `member` would unpack to more than [`MAX_RATIO`] times its packed size.
    Ratio { member: String },
    /// Member `member` would unpack to more than `limit` bytes, its reader's cap.
    Member { member: String, limit: u64 },
    /// The archive would unpack to more than `limit` bytes in all.
    Total { limit: u64 },
}

impl fmt::Display for Unpack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unpack::Entries { count } => write!(f, "it has {count} parts, more than the {MAX_ENTRIES} Scrim will unpack"),
            Unpack::Ratio { member } => {
                write!(
                    f,
                    "{member} would unpack to more than {MAX_RATIO} times its size, which no design file does (it may be damaged, or built to fill memory)"
                )
            }
            Unpack::Member { member, limit } => write!(f, "{member} would unpack to more than {}", size(*limit)),
            Unpack::Total { limit } => write!(f, "it would unpack to more than {} in all", size(*limit)),
        }
    }
}

/// `bytes` as people read a size: `512 KB`, `256 MB`, `4 GB`.
pub fn size(bytes: u64) -> String {
    match bytes {
        b if b >= 1 << 30 => format!("{} GB", b >> 30),
        b if b >= 1 << 20 => format!("{} MB", b >> 20),
        b => format!("{} KB", b.div_ceil(1 << 10)),
    }
}

/// The most a member that packs to `packed` bytes may unpack to.
pub(crate) fn ratio_limit(packed: u64) -> u64 {
    packed.saturating_mul(MAX_RATIO).max(RATIO_FREE)
}

/// Refuse an archive of `len` bytes with too many members, or that says it unpacks to more than
/// its size allows ([`MAX_RATIO`] times it, from [`TOTAL_FREE`] to [`MAX_TOTAL`]).
pub(crate) fn check<R: std::io::Read + std::io::Seek>(z: &mut zip::ZipArchive<R>, len: u64) -> Result<(), FormatError> {
    if z.len() > MAX_ENTRIES {
        return Err(FormatError::Unpack(Unpack::Entries { count: z.len() }));
    }
    let allowed = len.saturating_mul(MAX_RATIO).clamp(TOTAL_FREE, MAX_TOTAL);
    let mut said = 0u64;
    for i in 0..z.len() {
        if let Ok(f) = z.by_index_raw(i) {
            said = said.saturating_add(f.size());
        }
        if said > allowed {
            return Err(FormatError::Unpack(Unpack::Total { limit: allowed }));
        }
    }
    Ok(())
}

/// Why `bytes`, if they are an archive, would be refused for what they say they unpack to: too
/// many parts, too much in all, or a part past [`MAX_RATIO`]. Read from the archive's directory
/// only (nothing is unpacked), so a file no importer recognises can still be told apart from a zip
/// bomb.
pub fn refusal(bytes: &[u8]) -> Option<Unpack> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
    if let Err(FormatError::Unpack(u)) = check(&mut z, bytes.len() as u64) {
        return Some(u);
    }
    (0..z.len()).find_map(|i| {
        let f = z.by_index_raw(i).ok()?;
        (f.size() > ratio_limit(f.compressed_size())).then(|| Unpack::Ratio { member: crate::clip(f.name(), 120) })
    })
}
