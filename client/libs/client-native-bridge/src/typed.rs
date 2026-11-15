//! The closing added as an opening is typed (ADR 050), for a shell that counts its cursor in
//! UTF-16 units.

/// The body with the closing of the `[[` or the block opening just typed. `units` is the
/// cursor right after the typed character, in UTF-16 units. None when the body stays as it is.
pub fn closed_as_typed(body: &str, units: usize) -> Option<String> {
    let cursor = client_core::util::utf16::chars_of_utf16(body, units);
    client_core::util::auto_close(body, cursor)
}
