//! Times the steps of an item body save against a throw-away server state.
use journal_server::config::Settings;
use journal_server::domain::model::ItemPosition;
use journal_server::state::{AppState, BuildOptions};
use std::sync::Arc;
use std::time::Instant;

fn p50(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("config")).unwrap();
    std::fs::write(
        dir.path().join("config/config.json"),
        format!(
            "{{\"markdownDir\": \"{}\"}}",
            dir.path().join("md").display()
        ),
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("md")).unwrap();
    let state: Arc<AppState> = AppState::build(
        Settings::under(dir.path(), 0),
        BuildOptions { interactive: false },
    )
    .unwrap();
    let doc = state
        .page_service
        .get_or_create_doc(Some("bench"), Some(""))
        .unwrap();
    let item = state
        .item_service
        .create_item(&doc.page.id, &ItemPosition::Last, 1, Some("hello"), None)
        .unwrap();
    let id = item.id.clone();
    let mut full = vec![];
    let mut patch = vec![];
    let mut tags = vec![];
    let mut touch = vec![];
    let mut find = vec![];
    let mut queue = vec![];
    for i in 0..100 {
        let body = format!("typed text {i} with a [[link]] and #tag{}", i % 7);
        let t = Instant::now();
        state
            .item_service
            .update_item(&id, Some(&body), None, None)
            .unwrap();
        full.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        let it = state.items.find_by_id(&id).unwrap().unwrap();
        find.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        let p = journal_server::domain::model::ItemPatch::new(
            Some(body.clone()),
            Some(String::new()),
            Some(vec![]),
            None,
            None,
            None,
            0,
        )
        .unwrap();
        state.items.patch(&id, &p).unwrap();
        patch.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        state
            .properties
            .sync_tags("item", id.as_str(), &["a".to_string()])
            .unwrap();
        tags.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        state.pages.save(&doc.page).unwrap();
        touch.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        state.export.queue_export(&it.page_id);
        queue.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    println!("update_item total p50 {:.2} ms", p50(&mut full));
    println!("  items.find_by_id  p50 {:.2} ms", p50(&mut find));
    println!("  items.patch       p50 {:.2} ms", p50(&mut patch));
    println!("  sync_tags         p50 {:.2} ms", p50(&mut tags));
    println!("  pages.save(touch) p50 {:.2} ms", p50(&mut touch));
    println!("  queue_export      p50 {:.2} ms", p50(&mut queue));
}
