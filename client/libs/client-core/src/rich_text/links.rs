//! Where a link of an item leads, from the address that the page writes.

use super::RichLink;
use percent_encoding::percent_decode_str;

/// The folder of the files of the pages, such as images, under the Markdown folder.
const ASSETS: &str = "assets/";

/// The page of a wikilink, which names the page and maybe an item of it (`notes#item`).
pub(super) fn page(page: &str) -> RichLink {
    RichLink::Page {
        url: format!("/pages/{page}"),
    }
}

/// The page of a tag, without its `#`. A tag page lies under the folder of the tags.
pub(super) fn tag(tags_dir: &str, tag: &str) -> RichLink {
    RichLink::Page {
        url: format!("/pages/{tags_dir}{tag}"),
    }
}

/// Where the address of a link leads. A relative address is read from the top of the app, so
/// that `../assets/report.pdf` names the file of the assets folder. `None` for any other link,
/// such as one to a file of the PC or a `mailto:`: it leads nowhere.
pub(super) fn target(address: &str) -> Option<RichLink> {
    let scheme = scheme_of(address);
    if let Some(scheme) = scheme {
        let web = scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http");
        return web.then(|| RichLink::Web {
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
    let place = place.split('?').next().unwrap_or(place);
    let path = decoded(&resolved(place));
    if path.starts_with("/pages/") {
        let fragment = fragment
            .map(|f| format!("#{}", decoded(f)))
            .unwrap_or_default();
        return Some(RichLink::Page {
            url: format!("{path}{fragment}"),
        });
    }
    asset_path(&path).map(|path| RichLink::File { path })
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
