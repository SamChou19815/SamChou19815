// The cipher itself, with no dependencies of its own.
//
// Included textually by both `crypt.rs` and `build.rs`: the crate encrypts its
// own literals through `encrypted_str!`, while the build script encrypts the
// blog corpus, and the two must agree byte for byte. A build script cannot
// depend on the crate it builds, so sharing the source is what keeps one
// definition of the cipher rather than two that can drift.
//
// See `crypt.rs` for what this is for and what it is not.

/// The key the keystream is derived from.
const KEY: &[u8] = b"dev-sam";

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a over [`KEY`], folded once so every keystream byte does not pay for
/// it again.
const fn key_hash() -> u64 {
    let mut hash = FNV_OFFSET;
    let mut index = 0;
    while index < KEY.len() {
        hash = (hash ^ KEY[index] as u64).wrapping_mul(FNV_PRIME);
        index += 1;
    }
    hash
}

const KEY_HASH: u64 = key_hash();

/// The mask byte position `index` of a string seeded with `seed` is hidden
/// under: the key's hash carried on through the seed and the position.
///
/// The seed is what keeps two strings that start alike — the many
/// `https://github.com/SamChou19815/…` URLs — from sharing a ciphertext
/// prefix, which a position-only keystream would leave in plain sight.
const fn keystream(seed: u32, index: usize) -> u8 {
    let mut hash = KEY_HASH;
    let mut mixed = ((seed as u64) << 32) | (index as u64 & 0xffff_ffff);
    let mut byte = 0;
    while byte < 8 {
        hash = (hash ^ (mixed & 0xff)).wrapping_mul(FNV_PRIME);
        mixed >>= 8;
        byte += 1;
    }
    // Fold the whole word down to the byte that is actually used, so the low
    // bits of one multiplication are not the whole answer.
    (hash ^ (hash >> 24) ^ (hash >> 48)) as u8
}

/// One byte of ciphertext. The rotation after the mask moves a plaintext
/// byte's bits out from under it, so a run of ASCII does not stay a run of
/// bytes with the same high bits.
pub(crate) const fn scramble(byte: u8, seed: u32, index: usize) -> u8 {
    (byte ^ keystream(seed, index)).rotate_left(3)
}

/// The exact inverse of [`scramble`].
const fn unscramble(byte: u8, seed: u32, index: usize) -> u8 {
    byte.rotate_right(3) ^ keystream(seed, index)
}

/// A string's own seed: FNV-1a over its bytes, so no two distinct strings are
/// masked with the same keystream. Stored beside the ciphertext — it is a
/// diversifier, not a second secret.
pub const fn seed_of(text: &str) -> u32 {
    let bytes = text.as_bytes();
    let mut hash: u32 = 0x811c_9dc5;
    let mut index = 0;
    while index < bytes.len() {
        hash = (hash ^ bytes[index] as u32).wrapping_mul(0x0100_0193);
        index += 1;
    }
    hash
}

/// Encrypts `text` into `N` bytes, where `N` is the length of `text`. A `const
/// fn`, so the only place it ever runs is const evaluation.
///
/// Const evaluation is far slower than the compiled cipher and rustc denies a
/// long-running one outright, so this is for the literals a human writes.
/// Anything corpus-sized — the blog — is encrypted by `build.rs`, which runs
/// the same [`scramble`] compiled.
pub const fn encrypt<const N: usize>(text: &str, seed: u32) -> [u8; N] {
    let bytes = text.as_bytes();
    let mut cipher = [0u8; N];
    let mut index = 0;
    while index < N {
        cipher[index] = scramble(bytes[index], seed, index);
        index += 1;
    }
    cipher
}
