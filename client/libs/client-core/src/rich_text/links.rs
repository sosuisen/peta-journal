//! Where a link of an item leads, from the address that the page writes.

use super::RichLink;
use percent_encoding::percent_decode_str;

/// The folder of the files of the pages, such as images, under the Markdown folder.
const ASSETS: &str = "assets/";

/// The page of a wikilink, which names the page and maybe an item of it (`notes#<item id>`).
pub(super) fn page(content: &str) -> RichLink {
    let url = match page_urls::split_wikilink(content) {
        (name, Some(item)) => page_urls::item_url(name, item),
        (name, None) => page_urls::page_url(name),
    };
    RichLink::Page { url }
}

/// The page of a tag, without its `#`. A tag page lies under the folder of the tags.
pub(super) fn tag(tags_dir: &str, tag: &str) -> RichLink {
    RichLink::Page {
        url: page_urls::page_url(&page_urls::tag_page_name(tags_dir, tag)),
    }
}

/// Where the address of a link leads. An address with a scheme leads out of the app, whatever
/// the scheme is: `mailto:`, `obsidian://` and a drive of Windows (`C:`) as well. The shell has
/// the OS open it (#1071). A relative address is read from the top of the app, so that
/// `../assets/report.pdf` names the file of the assets folder. `None` for any other relative
/// address: it leads nowhere.
pub(super) fn target(address: &str) -> Option<RichLink> {
    if scheme_of(address).is_some() {
        return Some(RichLink::Web {
            url: address.to_string(),
        });
    }
    // An address of another host (`//host/path`) is not one of the app
    if address.starts_with("//") {
        return None;
    }
    let (place, fragment) = match address.split_once('#') {
        Some((place, fragment)) => (place, Some(fragment)),
        None => (address, None),
    };
    let place = resolved(place.split('?').next().unwrap_or(place));
    let whole = match fragment {
        Some(fragment) => format!("{place}#{fragment}"),
        None => place.clone(),
    };
    // The URL is made again, so that a link written without encoding gets the same URL
    if let Some(address) = page_urls::read_url(&whole) {
        let url = match address.item {
            Some(item) => page_urls::item_url(&address.name, &item),
            None => page_urls::page_url(&address.name),
        };
        return Some(RichLink::Page { url });
    }
    asset_path(&decoded(&place)).map(|path| RichLink::File { path })
}

/// The path under the Markdown folder of the file that the address names, when the file is in
/// the assets folder: `assets/picture.png` for `../assets/picture.png`. `None` for any other
/// address. The address may be encoded as a web address is.
pub fn asset_path(address: &str) -> Option<String> {
    let file = address.split('#').next().unwrap_or(address);
    let file = decoded(file);
    let path = file.trim_start_matches(['.', '/']);
    path.to_lowercase()
        .starts_with(ASSETS)
        .then(|| path.to_string())
}

/// The scheme of an address, such as `https`. `None` for a relative address.
fn scheme_of(address: &str) -> Option<&str> {
    let (scheme, _) = address.split_once(':')?;
    let mut letters = scheme.chars();
    let starts = letters.next()?.is_ascii_alphabetic();
    let rest = letters.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    (starts && rest).then_some(scheme)
}

/// The path that a relative path leads to from the top of the app, with no `.` or `..` left.
fn resolved(place: &str) -> String {
    let mut segments: Vec<&str> = Vec::new();
    for segment in place.split('/').filter(|s| !s.is_empty()) {
        match segment {
            "." => {}
            ".." => {
                segments.pop();
            }
            _ => segments.push(segment),
        }
    }
    format!("/{}", segments.join("/"))
}

fn decoded(text: &str) -> String {
    percent_decode_str(text).decode_utf8_lossy().into_owned()
}
