//! Text encoding detection and conversion utilities for shell output.
//!
//! Non-UTF-8 command output is decoded with best-effort encoding detection before falling back to
//! lossy UTF-8 decoding.

use chardetng::EncodingDetector;
use encoding_rs::Encoding;
use encoding_rs::WINDOWS_1252;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct StreamOutput<T: Clone> {
    pub text: T,
    pub truncated_after_lines: Option<u32>,
}

impl StreamOutput<String> {
    pub fn new(text: String) -> Self {
        Self {
            text,
            truncated_after_lines: None,
        }
    }
}

impl StreamOutput<Vec<u8>> {
    pub fn from_utf8_lossy(&self) -> StreamOutput<String> {
        StreamOutput {
            text: bytes_to_string_smart(&self.text),
            truncated_after_lines: self.truncated_after_lines,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExecToolCallOutput {
    pub exit_code: i32,
    pub stdout: StreamOutput<String>,
    pub stderr: StreamOutput<String>,
    pub aggregated_output: StreamOutput<String>,
    pub duration: Duration,
    pub timed_out: bool,
}

impl Default for ExecToolCallOutput {
    fn default() -> Self {
        Self {
            exit_code: 0,
            stdout: StreamOutput::new(String::new()),
            stderr: StreamOutput::new(String::new()),
            aggregated_output: StreamOutput::new(String::new()),
            duration: Duration::ZERO,
            timed_out: false,
        }
    }
}

/// Attempts to convert arbitrary bytes to UTF-8 with best-effort encoding detection.
pub fn bytes_to_string_smart(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    if let Ok(utf8_str) = std::str::from_utf8(bytes) {
        return utf8_str.to_owned();
    }

    let encoding = detect_encoding(bytes);
    decode_bytes(bytes, encoding)
}

fn detect_encoding(bytes: &[u8]) -> &'static Encoding {
    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let (encoding, _is_confident) = detector.guess_assess(None, true);

    // chardetng classifies short C1 punctuation sequences such as curly
    // quotes and an en dash as Windows-1251. Prefer Windows-1252 when those
    // bytes are present; otherwise retain its broader language detection.
    let windows_1252_punctuation = bytes
        .iter()
        .filter(|byte| matches!(byte, 0x91 | 0x92 | 0x93 | 0x94 | 0x96 | 0x97))
        .count();
    if windows_1252_punctuation >= 2 {
        WINDOWS_1252
    } else {
        encoding
    }
}

fn decode_bytes(bytes: &[u8], encoding: &'static Encoding) -> String {
    let (decoded, _, had_errors) = encoding.decode(bytes);

    if had_errors {
        return String::from_utf8_lossy(bytes).into_owned();
    }

    decoded.into_owned()
}

#[cfg(test)]
#[path = "exec_output_tests.rs"]
mod tests;
