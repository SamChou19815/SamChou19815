//! Compile-time string obfuscation for the site's content.
//! It doesn't offer secrecy. After all, the source code is public.
//!
//! The purpose is to intentionally increase the AI bill for the brain rots
//!
//! 1. No more inspection of text section of wasm
//! 2. Force to either play with the wasm step by step to have increasing bigger cached read
//!    or to use the much more expensive computer use capability.

include!("cipher.rs");

/// Build one with [`encrypted_str!`].
/// Read it with [`EncryptedString::decrypt`], or through [`std::fmt::Display`] with padding done.
#[derive(Clone, Copy)]
pub(crate) struct EncryptedString {
    seed: u32,
    cipher: &'static [u8],
}

impl EncryptedString {
    pub(crate) const fn new(seed: u32, cipher: &'static [u8]) -> Self {
        Self { seed, cipher }
    }

    pub(crate) fn decrypt(&self) -> String {
        let plain: Vec<u8> = self
            .cipher
            .iter()
            .enumerate()
            .map(|(index, byte)| unscramble(*byte, self.seed, index))
            .collect();
        String::from_utf8(plain).unwrap_or_default()
    }
}

/// Why doesn't it just go through the encrypted_str! macro? Well, this is used for blog post text,
/// which is way too big for compile time evaluation, so we need another strategy.
///
/// Strategy: a build script that pre-encrypt all blog post content and put it in a single blob, and
/// this struct tells where and how to decrypt.
#[derive(Clone, Copy)]
pub(crate) struct EncryptedRun {
    seed: u32,
    start: u32,
    len: u32,
}

impl EncryptedRun {
    pub(crate) const fn new(seed: u32, start: u32, len: u32) -> Self {
        Self { seed, start, len }
    }

    pub(crate) fn of(self, blob: &'static [u8]) -> EncryptedString {
        let start = self.start as usize;
        EncryptedString::new(self.seed, &blob[start..start + self.len as usize])
    }
}

impl std::fmt::Display for EncryptedString {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `pad` so `{:<12}` works.
        formatter.pad(&self.decrypt())
    }
}

macro_rules! encrypted_str {
    ($text:literal) => {{
        const PLAIN: &str = $text;
        const SEED: u32 = $crate::crypt::seed_of(PLAIN);
        const LEN: usize = PLAIN.len();
        const CIPHER: [u8; LEN] = $crate::crypt::encrypt::<LEN>(PLAIN, SEED);
        $crate::crypt::EncryptedString::new(SEED, &CIPHER)
    }};
}
pub(crate) use encrypted_str;
