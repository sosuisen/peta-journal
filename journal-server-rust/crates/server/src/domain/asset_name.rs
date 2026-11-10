//! The names of the files in `assets/`. See ADR-022.

use hmtid::Hmtid;

/// The name a file is saved under: the ID and the extension of the name it came with.
/// Why not that name itself: names collide, and the characters of many names cannot stay in
/// a file name that is also part of a URL. The link in the body keeps the name.
pub fn saved_name(id: &Hmtid, original_name: &str) -> String {
    match plain_extension(original_name) {
        Some(extension) => format!("{id}.{extension}"),
        None => id.to_string(),
    }
}

/// The extension of `name` in lower case, when it is made of ASCII letters and digits.
fn plain_extension(name: &str) -> Option<String> {
    let (_, extension) = name.rsplit_once('.')?;
    let plain = !extension.is_empty() && extension.chars().all(|c| c.is_ascii_alphanumeric());
    plain.then(|| extension.to_ascii_lowercase())
}
