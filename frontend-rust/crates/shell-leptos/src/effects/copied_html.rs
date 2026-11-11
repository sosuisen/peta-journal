//! The HTML of a copy, which goes to the clipboard beside the markdown (ADR 027).
//!
//! Another app reads the HTML when the copy is pasted there. So a link to a page of this app
//! becomes a whole URL, which opens from that app. The AST comes from the server: only there
//! a link into the app is a node of its own kind.

use crate::page_links::AppLinks;
use flatmarkdown_ast::Node;
use flatmarkdown_ast2html::ast_to_html;

/// The HTML of a copy, from the AST of its tree markdown as JSON. An AST that cannot be read
/// gives no HTML: an empty copy has an empty string for its AST.
pub fn copied_html(ast: &str, links: AppLinks) -> String {
    let Ok(mut root) = Node::from_json(ast) else {
        return String::new();
    };
    link_to_the_app(&mut root, links);
    ast_to_html(&root)
}

/// Makes every wikilink and hashtag under `node` an ordinary link to its page.
fn link_to_the_app(node: &mut Node, links: AppLinks) {
    let link = match node {
        Node::Wikilink { url, children } => Some(Node::Link {
            url: links.wikilink(url),
            title: String::new(),
            children: std::mem::take(children),
        }),
        Node::Hashtag { value } => Some(Node::Link {
            url: links.hashtag(value),
            title: String::new(),
            children: vec![Node::Text {
                value: format!("#{value}"),
            }],
        }),
        _ => None,
    };
    if let Some(link) = link {
        *node = link;
    }
    for child in node.children_mut().into_iter().flatten() {
        link_to_the_app(child, links);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flatmarkdown::{BodyOptions, LinkResolution};

    const LINKS: AppLinks = AppLinks {
        app_url: "http://localhost:8080",
        tags_dir: "tags/",
    };

    /// The AST of tree markdown, as the journal server makes it: with wikilinks and hashtags.
    fn ast_of(tree_markdown: &str) -> String {
        let as_the_server = BodyOptions {
            link_resolution: LinkResolution::On {
                prefixes: vec!["./".to_string(), "../".to_string()],
            },
        };
        flatmarkdown::parse_body_with(tree_markdown, &as_the_server).to_json()
    }

    fn copied(tree_markdown: &str) -> String {
        copied_html(&ast_of(tree_markdown), LINKS)
    }

    /// The link that `ast_to_html` writes for a URL and a text.
    fn link(url: &str, text: &str) -> String {
        format!("<a href=\"{url}\">{text}</a>")
    }

    // コピーのASTから、HTMLができる。
    #[test]
    fn the_ast_of_a_copy_becomes_html() {
        assert_eq!(
            copied("- a **bold**\n  - b"),
            "<ul>\n<li>a <strong>bold</strong><ul>\n<li>b</li>\n</ul>\n</li>\n</ul>\n"
        );
    }

    // 空のコピーは、ASTが空の文字列である。HTMLも空である。
    #[test]
    fn an_empty_copy_has_no_html() {
        assert_eq!(copied_html("", LINKS), "");
    }

    // ウィキリンクは、そのページを開くふつうのリンクになる。表示する文字は変わらない。
    #[test]
    fn a_wikilink_becomes_an_ordinary_link_that_opens_its_page() {
        assert_eq!(
            copied("see [[Dir/Page|alias]]"),
            format!(
                "<p>see {}</p>\n",
                link(&LINKS.wikilink("Dir/Page"), "alias")
            )
        );
    }

    // .md への相対リンクも、サーバーのASTではウィキリンクなので、そのページを開くリンクになる。
    #[test]
    fn a_relative_link_to_an_md_file_opens_its_page_too() {
        assert_eq!(
            copied("[note](./note.md)"),
            format!("<p>{}</p>\n", link(&LINKS.wikilink("note"), "note"))
        );
    }

    // ハッシュタグは、タグのページを開くリンクになる。表示する文字は # とタグ名である。
    #[test]
    fn a_hashtag_becomes_a_link_that_opens_the_page_of_the_tag() {
        assert_eq!(
            copied("about #topic"),
            format!("<p>about {}</p>\n", link(&LINKS.hashtag("topic"), "#topic"))
        );
    }

    // リストの中など、深いところにあるリンクも置き換わる。
    #[test]
    fn a_link_deep_in_the_copy_is_replaced_too() {
        assert_eq!(
            copied("- a\n  - [[Page]]"),
            format!(
                "<ul>\n<li>a<ul>\n<li>{}</li>\n</ul>\n</li>\n</ul>\n",
                link(&LINKS.wikilink("Page"), "Page")
            )
        );
    }

    // 外のサイトへのリンクと画像は、そのままである。
    #[test]
    fn a_link_to_another_site_and_an_image_stay_as_they_are() {
        assert_eq!(
            copied("[site](https://example.com) ![pic](pic.png#w=300)"),
            "<p><a href=\"https://example.com\">site</a> <img src=\"pic.png\" alt=\"pic\" /></p>\n"
        );
    }
}
