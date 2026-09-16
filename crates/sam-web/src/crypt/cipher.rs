// Zero dependency FNV-1a cipher. Not designed to be secure.

const KEY: &[u8] = b"dev-sam";

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

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

/// Given a seed and position, generate xor cipher key.
const fn keystream(seed: u32, index: usize) -> u8 {
    let mut hash = KEY_HASH;
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

pub(crate) const fn scramble(byte: u8, seed: u32, index: usize) -> u8 {
    // XOR cipher, with a simple rotation to avoid simple ASCII decoding
    (byte ^ keystream(seed, index)).rotate_left(3)
}

const fn unscramble(byte: u8, seed: u32, index: usize) -> u8 {
    byte.rotate_right(3) ^ keystream(seed, index)
}

/// FNV-1a of the plaintext.
pub(crate) const fn seed_of(text: &str) -> u32 {
    let bytes = text.as_bytes();
    let mut hash: u32 = 0x811c_9dc5;
    let mut index = 0;
    while index < bytes.len() {
        hash = (hash ^ bytes[index] as u32).wrapping_mul(0x0100_0193);
        index += 1;
    }
    hash
}

/// Compile-time only. Too slow for anything big (blog posts go through build.rs instead).
pub(crate) const fn encrypt<const N: usize>(text: &str, seed: u32) -> [u8; N] {
    let bytes = text.as_bytes();
    let mut cipher = [0u8; N];
    let mut index = 0;
    while index < N {
        cipher[index] = scramble(bytes[index], seed, index);
        index += 1;
    }
    cipher
}
