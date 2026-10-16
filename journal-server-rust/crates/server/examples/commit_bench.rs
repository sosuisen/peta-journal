//! Times the parts of an item save: markdown parsing, the SQLite patch and the search index commit.
use std::time::Instant;
use hmtid::Hmtid;
use journal_server::application::markdown_parser::MarkdownParser;
use journal_server::domain::model::Item;
use journal_server::domain::search::SearchIndex;
use journal_server::infrastructure::search::TantivySearchService;

fn main() {
    let dir = tempfile::tempdir().unwrap();
    let t = Instant::now();
    let search = TantivySearchService::open(dir.path()).unwrap();
    println!("open index: {:.1} ms", t.elapsed().as_secs_f64() * 1000.0);
    let item = Item {
        id: Hmtid::parse("2026-09-29-00-00-01-AAAAAAA").unwrap(),
        page_id: Hmtid::parse("2026-09-29-00-00-00-AAAAAAA").unwrap(),
        parent_item_id: None, dir_path: String::new(), name: "p".into(), item_type: "doc".into(),
        created_date: 0, modified_date: 0, idx: "z0".into(), level: 1,
        body: "typed text 12 with a [[link]] and #tag3 東京国際空港に行きました".into(),
        ast: String::new(), collapsed: false, types: vec![], transclude_item_id: None,
    };
    let parser = MarkdownParser;
    let mut parse = vec![]; let mut index = vec![];
    for _ in 0..60 {
        let t = Instant::now(); let _ = parser.parse_to_ast(&item.body); parse.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now(); search.index_item(&item, "p"); index.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    let p50 = |v: &mut Vec<f64>| { v.sort_by(|a, b| a.partial_cmp(b).unwrap()); v[v.len() / 2] };
    println!("parse_to_ast p50 {:.2} ms", p50(&mut parse));
    println!("index_item (delete+add+commit+reload) p50 {:.2} ms", p50(&mut index));
}
