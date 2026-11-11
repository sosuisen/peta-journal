//! The one form the names of pages are kept in, see ADR 007.

use unicode_normalization::UnicodeNormalization;

/// The text in Unicode NFC. Called where a name enters the server. The code inside compares
/// names as plain strings and does not call this again.
pub fn to_nfc(text: &str) -> String {
    text.nfc().collect()
}
