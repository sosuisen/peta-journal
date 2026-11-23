//! The links of the tags that the web shell draws as chips, as a hashtag links them.

/// The URL of the page of the tag `tag`, in the folder of the tags `tags_dir`.
pub fn tag_url(tags_dir: &str, tag: &str) -> String {
    page_urls::page_url(&page_urls::tag_page_name(tags_dir, tag))
}

#[cfg(test)]
mod tests {
    use super::*;

    // タグのページのURLは、タグのフォルダの中のページのURLで、符号化されている。
    #[test]
    fn the_url_of_a_tag_is_its_page_in_the_folder_of_the_tags_encoded() {
        assert_eq!(tag_url("tags/", "What?"), "/pages/tags/What%3F");
    }
}
