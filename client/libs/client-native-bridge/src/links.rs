//! The addresses at which the browser opens a page or a file of the journal server. The shell
//! is given the URL of the app as the core writes it; the encoding is that of the copied HTML
//! and of the web frontend (`client_page_links::AppLinks`).

use client_page_links::AppLinks;

/// The address on the journal server at `app_url` of a URL of the app, such as
/// `/pages/notes#item`.
pub fn page_address(app_url: &str, url: &str) -> String {
    links(app_url).page(url)
}

/// The address on the journal server at `app_url` of a file under the Markdown folder, such
/// as `assets/picture.png`.
pub fn asset_address(app_url: &str, path: &str) -> String {
    links(app_url).asset(path)
}

/// Neither address leads to the page of a tag, so the folder of the tags is not needed.
fn links(app_url: &str) -> AppLinks<'_> {
    AppLinks {
        app_url,
        tags_dir: "",
    }
}
