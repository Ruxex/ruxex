//! A crates.io placeholder for the Git-hosted Ruxex implementation.

const NOTICE: &str = "This is the crates.io placeholder for Ruxex. Use the Git dependency from https://github.com/Ruxex/ruxex instead.";

/// Explains how to depend on the actual Ruxex implementation.
///
/// This package intentionally contains no Ruxex implementation. Calling this
/// function prints the migration hint and returns the same message.
pub fn notice() -> &'static str {
    eprintln!("{NOTICE}");
    NOTICE
}
