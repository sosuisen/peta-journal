//! The names of the files in `assets/`. See ADR-022.

use std::collections::HashMap;
use std::sync::LazyLock;

use hmtid::Hmtid;
use regex::{Captures, Regex};

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

/// Whether the file name is one `saved_name` gives: an ID, with or without an extension.
pub fn is_saved_name(name: &str) -> bool {
    let id = name.split_once('.').map_or(name, |(id, _)| id);
    Hmtid::parse(id).is_ok()
}

/// The destination of a link to a file of `assets/`, up to the end of the file name. A size
/// may follow the name after `#`.
static ASSET_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\]\(\.\./assets/([^)#\s]+)").unwrap());

/// The body with every link to a file in `new_names` pointing to the new name of the file.
/// The text of a link stays as it is.
pub fn relink(body: &str, new_names: &HashMap<String, String>) -> String {
    ASSET_LINK
        .replace_all(body, |link: &Captures| {
            let name = new_names.get(&link[1]).map_or(&link[1], String::as_str);
            format!("](../assets/{name}")
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> HashMap<String, String> {
        HashMap::from([(
            "image.png".to_string(),
            "20260203040506_ABCDEFG.png".to_string(),
        )])
    }

    // 画像のリンク先が、新しい名前に変わる。リンクの文字は変わらない。
    #[test]
    fn the_link_to_a_file_goes_to_its_new_name_and_keeps_its_text() {
        let body = "before ![pic](../assets/image.png) after";

        let relinked = relink(body, &names());

        assert_eq!(
            relinked,
            "before ![pic](../assets/20260203040506_ABCDEFG.png) after"
        );
    }

    // リンク先のうしろの大きさの指定は残る。
    #[test]
    fn the_size_after_the_name_stays() {
        let body = "![pic](../assets/image.png#w=100&h=50)";

        let relinked = relink(body, &names());

        assert_eq!(
            relinked,
            "![pic](../assets/20260203040506_ABCDEFG.png#w=100&h=50)"
        );
    }

    // 1つの本文にあるリンクは、すべて変わる。
    #[test]
    fn every_link_in_a_body_changes() {
        let body = "![a](../assets/image.png) ![b](../assets/image.png)";

        let relinked = relink(body, &names());

        assert_eq!(
            relinked,
            "![a](../assets/20260203040506_ABCDEFG.png) ![b](../assets/20260203040506_ABCDEFG.png)"
        );
    }

    // 表にない名前へのリンクは変わらない。表の名前で始まる名前や終わる名前も同じである。
    #[test]
    fn a_link_to_a_name_that_is_not_in_the_table_stays() {
        let body =
            "![a](../assets/other.png) ![b](../assets/image.png.bak) ![c](../assets/my-image.png)";

        let relinked = relink(body, &names());

        assert_eq!(relinked, body);
    }

    // リンクの外に書かれた名前は変わらない。
    #[test]
    fn a_name_outside_a_link_stays() {
        let body = "the file ../assets/image.png and (../assets/image.png)";

        let relinked = relink(body, &names());

        assert_eq!(relinked, body);
    }
}
