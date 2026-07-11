pub const SUPPORTED: &[&str] = &[
    "linux-x86_64",
    "linux-aarch64",
    "macos-x86_64",
    "macos-aarch64",
];

pub fn current() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}
