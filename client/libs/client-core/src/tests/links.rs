//! A click on a link in the text of an item: the core decides how it opens (ADR 083).

use crate::app::Event;
use crate::effects::UiOp;
use crate::harness::Harness;
use crate::keys::Held;
use crate::rich_text::RichLink;

fn click(link: RichLink, held: Held) -> Harness {
    let mut h = Harness::new();
    h.send(Event::LinkClicked { link, held });
    h
}

fn page(url: &str) -> RichLink {
    RichLink::Page { url: url.into() }
}

const NONE: Held = Held {
    shift: false,
    ctrl: false,
    cmd: false,
};
const SHIFT: Held = Held {
    shift: true,
    ..NONE
};
const CTRL: Held = Held { ctrl: true, ..NONE };
const CMD: Held = Held { cmd: true, ..NONE };

fn opened_new_views(h: &Harness) -> Vec<&str> {
    h.ui.iter()
        .filter_map(|op| match op {
            UiOp::OpenNewView { url } => Some(url.as_str()),
            _ => None,
        })
        .collect()
}

fn opened_in_browser(h: &Harness) -> Vec<&str> {
    h.ui.iter()
        .filter_map(|op| match op {
            UiOp::OpenInBrowser { address } => Some(address.as_str()),
            _ => None,
        })
        .collect()
}

// キーを押さずにページへのリンクをクリックすると、そのページがこの画面で開く。
#[test]
fn a_plain_click_on_a_page_opens_it_here() {
    let h = click(page("/pages/my%20notes#item"), NONE);

    let navigation = h.navigations.last().expect("the page opens");
    assert_eq!(navigation.url, "/pages/my%20notes#item");
    assert!(!navigation.replace);
    assert!(opened_new_views(&h).is_empty());
    assert!(opened_in_browser(&h).is_empty());
}

// CtrlかCommandを押してページへのリンクをクリックすると、アプリの新しい画面で開く。この画面は変わらない。
#[test]
fn ctrl_or_command_opens_a_page_in_a_new_view() {
    for held in [CTRL, CMD] {
        let h = click(page("/pages/notes"), held);

        assert_eq!(opened_new_views(&h), ["/pages/notes"]);
        assert!(h.navigations.is_empty());
    }
}

// Shiftを押してページへのリンクをクリックすると、Web版のそのページがブラウザで開く。Ctrlも押していればShiftが勝つ。
#[test]
fn shift_opens_a_page_in_the_browser() {
    for held in [
        SHIFT,
        Held {
            ctrl: true,
            ..SHIFT
        },
    ] {
        let h = click(page("/pages/notes"), held);

        assert_eq!(opened_in_browser(&h), ["/pages/notes"]);
        assert!(opened_new_views(&h).is_empty());
        assert!(h.navigations.is_empty());
    }
}

// 外へのリンクは、どのキーを押していても、ブラウザで開く。
#[test]
fn a_link_out_of_the_journal_opens_in_the_browser() {
    for held in [NONE, SHIFT, CTRL] {
        let h = click(
            RichLink::Web {
                url: "obsidian://open?vault=a".into(),
            },
            held,
        );

        assert_eq!(opened_in_browser(&h), ["obsidian://open?vault=a"]);
        assert!(h.navigations.is_empty());
    }
}

// assetsのファイルへのリンクは、journalサーバーの/assets/のアドレスで、ブラウザで開く。名前は符号化される。
#[test]
fn a_file_of_the_assets_opens_in_the_browser_from_the_server() {
    let h = click(
        RichLink::File {
            path: "assets/my report.pdf".into(),
        },
        CMD,
    );

    assert_eq!(opened_in_browser(&h), ["/assets/my%20report.pdf"]);
}
