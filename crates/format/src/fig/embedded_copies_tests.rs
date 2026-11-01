//! Image hashes naming one embedded blob share one copy of it (upstream FigCraft 3009d19f, audit
//! #1059 FMT-3): each used to get its own, so a small file pointing many hashes at a large blob
//! multiplied it in memory (100 hashes on a 250 MB blob were 25 GB).

use std::sync::Arc;

use super::embedded_copies;
use super::kiwi::Kv;

fn image(hash: u8, blob: u64) -> Kv<'static> {
    Kv::Object(vec![("image", Kv::Object(vec![("hash", Kv::Bytes(vec![hash; 20])), ("dataBlob", Kv::Uint(blob))]))])
}

#[test]
fn hashes_naming_one_blob_share_one_copy_of_it() {
    let big = vec![0x5A; 4096];
    let blob = |b: Vec<u8>| Kv::Object(vec![("bytes", Kv::Bytes(b))]);
    let msg = Kv::Object(vec![
        ("blobs", Kv::Array(vec![blob(big.clone()), blob(vec![7; 16])])),
        ("nodeChanges", Kv::Array((1..=50).map(|h| image(h, 0)).chain([image(99, 1), image(98, 2)]).collect())),
    ]);
    let copies = embedded_copies(&msg);
    assert_eq!(copies.len(), 51, "the hash naming no blob is left out");
    let first = &copies[&"01".repeat(20)];
    assert_eq!(**first, *big);
    assert_eq!(copies.values().filter(|c| Arc::ptr_eq(c, first)).count(), 50, "fifty hashes, one copy");
    assert_eq!(*copies[&"63".repeat(20)], [7; 16]);
}
