//! Compile-time string obfuscation for the site's content.
//! It doesn't offer secrecy. After all, the source code is public.
//!
//! The purpose is to intentionally increase the AI bill for the brain rots
//!
//! 1. No more inspection of text section of wasm
//! 2. Force to either play with the wasm step by step to have increasing bigger cached read
//!    or to use the much more expensive computer use capability.

include!("cipher.rs");

/// A string that is stored encrypted and decrypted on use. Build one with
/// [`encrypted_str!`]; read it with [`EncryptedString::decrypt`], or straight
/// through [`std::fmt::Display`], which pads and aligns like a `&str` does.
#[derive(Clone, Copy)]
pub struct EncryptedString {
    seed: u32,
    cipher: &'static [u8],
}

impl EncryptedString {
    /// Wraps ciphertext produced by [`encrypt`] under the same `seed`.
    pub const fn new(seed: u32, cipher: &'static [u8]) -> Self {
        Self { seed, cipher }
    }

    /// The plaintext. The cipher is a byte-wise mask, so the bytes that come
    /// back out are exactly the UTF-8 that went in and the check never fails.
    pub fn decrypt(&self) -> String {
        let plain: Vec<u8> = self
            .cipher
            .iter()
            .enumerate()
            .map(|(index, byte)| unscramble(*byte, self.seed, index))
            .collect();
        String::from_utf8(plain).unwrap_or_default()
    }
}

/// Where one string sits inside a shared ciphertext blob — the form `build.rs`
/// generates, where the whole blog is one `include_bytes!` rather than a
/// `const` array per string.
///
/// Deliberately holds no reference to the blob, only offsets into it, and is
/// resolved by [`EncryptedRun::of`] at run time. A `static` table that pointed
/// into the blob would drag its bytes into that table's own const-evaluated
/// allocation, and the binary would carry the corpus twice — the same reason
/// `image::Baked` records an offset and a length rather than a slice.
#[derive(Clone, Copy)]
pub struct EncryptedRun {
    seed: u32,
    start: u32,
    len: u32,
}

impl EncryptedRun {
    pub const fn new(seed: u32, start: u32, len: u32) -> Self {
        Self { seed, start, len }
    }

    /// The string this run names, read out of the blob it was encrypted into.
    pub fn of(self, blob: &'static [u8]) -> EncryptedString {
        let start = self.start as usize;
        EncryptedString::new(self.seed, &blob[start..start + self.len as usize])
    }
}

impl std::fmt::Display for EncryptedString {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `pad` rather than `write_str`: the shell lays its columns out with
        // `{:<12}` and friends, which a `write_str` impl would silently ignore.
        formatter.pad(&self.decrypt())
    }
}

/// A string literal, as ciphertext.
///
/// The literal stays readable in the source while only the encrypted bytes
/// reach the binary: the seed, the length and every ciphertext byte are `const`
/// items, so the plaintext is consumed entirely by const evaluation and no
/// code at run time refers to it.
///
/// Const evaluation is slow enough that rustc refuses a long-running one, so
/// this is sized for the literals a human writes. The blog corpus goes the
/// other way round — encrypted by `build.rs` and read back through
/// [`EncryptedString::slice`].
#[macro_export]
macro_rules! encrypted_str {
    ($text:literal) => {{
        const PLAIN: &str = $text;
        const SEED: u32 = $crate::crypt::seed_of(PLAIN);
        const LEN: usize = PLAIN.len();
        const CIPHER: [u8; LEN] = $crate::crypt::encrypt::<LEN>(PLAIN, SEED);
        $crate::crypt::EncryptedString::new(SEED, &CIPHER)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        assert_eq!(encrypted_str!("").decrypt(), "");
        assert_eq!(encrypted_str!("samlang").decrypt(), "samlang");
        assert_eq!(
            encrypted_str!("https://github.com/SamChou19815").decrypt(),
            "https://github.com/SamChou19815"
        );
        // Multi-byte UTF-8 survives a byte-wise mask.
        assert_eq!(
            encrypted_str!("Copyright © Sam — 2026").decrypt(),
            "Copyright © Sam — 2026"
        );
    }

    #[test]
    fn ciphertext_hides_the_plaintext() {
        // A long ASCII string shares no byte with its own plaintext at the
        // same position, so `strings` finds nothing to latch onto.
        let plain = "Sam's programming language with full type-inference.";
        let seed = seed_of(plain);
        let cipher: Vec<u8> = plain
            .bytes()
            .enumerate()
            .map(|(index, byte)| scramble(byte, seed, index))
            .collect();
        assert!(cipher
            .iter()
            .zip(plain.bytes())
            .all(|(encrypted, plain)| *encrypted != plain));
    }

    #[test]
    fn a_shared_prefix_does_not_survive() {
        // Two URLs off the same host encrypt to bytes that diverge from the
        // very first one, because each carries its own seed.
        let first = encrypted_str!("https://developersam.com/resume.pdf");
        let second = encrypted_str!("https://developersam.com/blog");
        assert_ne!(first.cipher[0], second.cipher[0]);
    }

    #[test]
    fn display_pads_like_a_str() {
        assert_eq!(format!("[{:<10}]", encrypted_str!("Docs")), "[Docs      ]");
        assert_eq!(format!("[{:>6}]", encrypted_str!("Demo")), "[  Demo]");
    }

    /// The path every blog string takes: two strings encrypted separately and
    /// laid end to end, each read back out of the shared blob by offset.
    #[test]
    fn a_run_of_a_shared_blob_round_trips() {
        let seeds = (seed_of("one"), seed_of("two"));
        let encrypt = |text: &'static str, seed| {
            text.bytes()
                .enumerate()
                .map(move |(index, byte)| scramble(byte, seed, index))
        };
        let blob: &'static [u8] = encrypt("one", seeds.0)
            .chain(encrypt("two", seeds.1))
            .collect::<Vec<u8>>()
            .leak();
        assert_eq!(EncryptedRun::new(seeds.0, 0, 3).of(blob).decrypt(), "one");
        assert_eq!(EncryptedRun::new(seeds.1, 3, 3).of(blob).decrypt(), "two");
    }
}
