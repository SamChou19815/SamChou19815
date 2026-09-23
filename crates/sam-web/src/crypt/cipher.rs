// Zero dependency FNV-1a cipher. Not designed to be secure.
//
// The key is not in the source: build.rs rolls a random one per build, so decrypting means digging
// it out of the wasm rather than running this file.

const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Given a key, seed and position, generate xor cipher key.
const fn keystream(key: u64, seed: u32, index: usize) -> u8 {
    let mut hash = key;
    let mut mixed = ((seed as u64) << 32) | (index as u64 & 0xffff_ffff);
    let mut byte = 0;
    while byte < 8 {
        hash = (hash ^ (mixed & 0xff)).wrapping_mul(FNV_PRIME);
        mixed >>= 8;
        byte += 1;
    }
    // Mix in the high bits instead of just truncating.
    (hash ^ (hash >> 24) ^ (hash >> 48)) as u8
}

pub(crate) const fn scramble(byte: u8, key: u64, seed: u32, index: usize) -> u8 {
    // XOR cipher, with a simple rotation to avoid simple ASCII decoding
    (byte ^ keystream(key, seed, index)).rotate_left(3)
}

const fn unscramble(byte: u8, key: u64, seed: u32, index: usize) -> u8 {
    byte.rotate_right(3) ^ keystream(key, seed, index)
}

/// FNV-1a of the plaintext.
pub(crate) const fn seed_of(text: &str) -> u32 {
    let mut rest = text.as_bytes();
    let mut hash: u32 = 0x811c_9dc5;
    while let [byte, tail @ ..] = rest {
        hash = (hash ^ *byte as u32).wrapping_mul(0x0100_0193);
        rest = tail;
    }
    hash
}

/// Compile-time only. Too slow for anything big (blog posts go through build.rs instead).
/// Out of bounds fails the build, so the indexing never reaches the wasm.
#[allow(clippy::indexing_slicing)]
pub(crate) const fn encrypt<const N: usize>(text: &str, key: u64, seed: u32) -> [u8; N] {
    let bytes = text.as_bytes();
    let mut cipher = [0u8; N];
    let mut index = 0;
    while index < N {
        cipher[index] = scramble(bytes[index], key, seed, index);
        index += 1;
    }
    cipher
}
