use crate::route::parse_url;
use peta_core::Route;

fn page(name: &str, fragment: Option<&str>) -> Option<Route> {
    Some(Route::Page {
        name: name.to_string(),
        fragment: fragment.map(str::to_string),
    })
}

// #の後ろは、開いた後で移動する項目のidになる。
#[test]
fn the_fragment_is_the_item_to_go_to() {
    assert_eq!(parse_url("/pages/a#item1"), page("a", Some("item1")));
}

// #のないURLは、ページだけを指す。
#[test]
fn a_url_without_a_fragment_names_only_the_page() {
    assert_eq!(parse_url("/pages/a"), page("a", None));
}
