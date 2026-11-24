//! The address at which a shell loads a file of the journal server, such as an image. The
//! encoding is that of the copied HTML and of the web frontend (`client_page_links::AppLinks`).
//! The browser opens what the core orders, with the address of the server put before it
//! (`shell.rs`, ADR 083).

use client_page_links::AppLinks;

/// The address on the journal server at `app_url` of a file under the Markdown folder, such
/// as `assets/picture.png`.
pub fn asset_address(app_url: &str, path: &str) -> String {
    links(app_url).asset(path)
}

/// The address leads to no page of a tag, so the folder of the tags is not needed.
fn links(app_url: &str) -> AppLinks<'_> {
    AppLinks {
        app_url,
        tags_dir: "",
    }
}
