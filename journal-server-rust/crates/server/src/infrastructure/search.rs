//! Full-text search with tantivy and a Japanese tokenizer (lindera + IPADIC), the counterpart of
//! Lucene's `JapaneseAnalyzer`: decompose mode, base forms, part-of-speech stop tags, NFKC
//! normalization, katakana stemming and lower casing.
//!
//! Writes only queue documents on the index writer, which is cheap; the commit that makes them
//! durable and searchable runs in the background (see `start_background_commits`), before any
//! search that would otherwise miss pending writes, and on `flush()`. An item save therefore
//! does not pay the commit (about 25 ms) on the request path.

use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use hmtid::Hmtid;
use lindera::dictionary::load_dictionary;
use lindera::mode::{Mode, Penalty};
use lindera::segmenter::Segmenter;
use lindera_analysis::character_filter::unicode_normalize::{
    UnicodeNormalizeCharacterFilter, UnicodeNormalizeKind,
};
use lindera_analysis::character_filter::BoxCharacterFilter;
use lindera_analysis::token_filter::japanese_base_form::JapaneseBaseFormTokenFilter;
use lindera_analysis::token_filter::japanese_katakana_stem::JapaneseKatakanaStemTokenFilter;
use lindera_analysis::token_filter::japanese_stop_tags::JapaneseStopTagsTokenFilter;
use lindera_analysis::token_filter::lowercase::LowercaseTokenFilter;
use lindera_analysis::token_filter::BoxTokenFilter;
use lindera_tantivy::tokenizer::LinderaTokenizer;
use parking_lot::Mutex;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, Query, QueryParser, RegexQuery, TermQuery};
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, Value, STORED, STRING,
};
use tantivy::snippet::SnippetGenerator;
use tantivy::tokenizer::TextAnalyzer;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term};

use crate::domain::model::{Item, Page, SearchResult};
use crate::domain::search::SearchIndex;

const TOKENIZER: &str = "lang_ja";
const TYPE_PAGE: &str = "page";
const TYPE_ITEM: &str = "item";

struct Fields {
    doc_type: Field,
    page_id: Field,
    page_title: Field,
    item_id: Field,
    body: Field,
}

pub struct TantivySearchService {
    index: Index,
    reader: IndexReader,
    writer: Mutex<IndexWriter<TantivyDocument>>,
    fields: Fields,
    index_dir: String,
    /// Writes queued on the writer since the last commit.
    dirty: AtomicBool,
}

/// How often pending writes are committed in the background.
pub const COMMIT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

/// Part-of-speech tags Lucene's `JapaneseAnalyzer` drops by default.
fn stop_tags() -> HashSet<String> {
    [
        "接続詞",
        "助詞",
        "助詞,格助詞",
        "助詞,格助詞,一般",
        "助詞,格助詞,引用",
        "助詞,格助詞,連語",
        "助詞,接続助詞",
        "助詞,係助詞",
        "助詞,副助詞",
        "助詞,間投助詞",
        "助詞,並立助詞",
        "助詞,終助詞",
        "助詞,副助詞／並立助詞／終助詞",
        "助詞,連体化",
        "助詞,副詞化",
        "助詞,特殊",
        "助動詞",
        "記号",
        "記号,一般",
        "記号,読点",
        "記号,句点",
        "記号,空白",
        "記号,括弧閉",
        "記号,括弧開",
        "記号,アルファベット",
        "その他,間投",
        "フィラー",
        "非言語音",
        "語断片",
        "未知語",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn build_tokenizer() -> anyhow::Result<LinderaTokenizer> {
    let dictionary = load_dictionary("embedded://ipadic")
        .map_err(|e| anyhow::anyhow!("Cannot load the IPADIC dictionary: {e}"))?;
    let segmenter = Segmenter::new(Mode::Decompose(Penalty::default()), dictionary, None);
    let mut tokenizer = LinderaTokenizer::from_segmenter(segmenter);
    tokenizer.append_character_filter(BoxCharacterFilter::from(
        UnicodeNormalizeCharacterFilter::new(UnicodeNormalizeKind::NFKC),
    ));
    tokenizer.append_token_filter(BoxTokenFilter::from(JapaneseBaseFormTokenFilter::new()));
    tokenizer.append_token_filter(BoxTokenFilter::from(JapaneseStopTagsTokenFilter::new(
        stop_tags(),
    )));
    tokenizer.append_token_filter(BoxTokenFilter::from(JapaneseKatakanaStemTokenFilter::new(
        NonZeroUsize::new(4).expect("non-zero"),
    )));
    tokenizer.append_token_filter(BoxTokenFilter::from(LowercaseTokenFilter::new()));
    Ok(tokenizer)
}

fn schema() -> (Schema, Fields) {
    let mut builder = Schema::builder();
    let text = TextOptions::default()
        .set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(TOKENIZER)
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
        .set_stored();
    let doc_type = builder.add_text_field("type", STRING | STORED);
    let page_id = builder.add_text_field("pageId", STRING | STORED);
    let page_title = builder.add_text_field("pageTitle", text.clone());
    let item_id = builder.add_text_field("itemId", STRING | STORED);
    let body = builder.add_text_field("body", text);
    (
        builder.build(),
        Fields {
            doc_type,
            page_id,
            page_title,
            item_id,
            body,
        },
    )
}

/// True when the directory holds a Lucene index (from the Java server), which tantivy cannot read.
fn is_lucene_index(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|e| {
        let name = e.file_name().to_string_lossy().into_owned();
        name.starts_with("segments")
            || name == "write.lock"
            || name.ends_with(".cfs")
            || name.ends_with(".si")
    })
}

impl TantivySearchService {
    pub fn open(index_dir: &Path) -> anyhow::Result<TantivySearchService> {
        std::fs::create_dir_all(index_dir)?;
        if is_lucene_index(index_dir) && !index_dir.join("meta.json").exists() {
            tracing::info!(
                "Replacing the Lucene index at {} with a tantivy index; it is rebuilt from the database",
                index_dir.display()
            );
            for entry in std::fs::read_dir(index_dir)?.flatten() {
                let path = entry.path();
                let _ = if path.is_dir() {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_file(&path)
                };
            }
        }
        let (schema, fields) = schema();
        let dir = tantivy::directory::MmapDirectory::open(index_dir)?;
        let index = Index::open_or_create(dir, schema)?;
        index.tokenizers().register(TOKENIZER, build_tokenizer()?);
        let writer = index.writer::<TantivyDocument>(50_000_000)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        tracing::info!("Search index initialized at: {}", index_dir.display());
        Ok(TantivySearchService {
            index,
            reader,
            writer: Mutex::new(writer),
            fields,
            index_dir: index_dir.display().to_string(),
            dirty: AtomicBool::new(false),
        })
    }

    /// Commits the pending writes, if any, and reloads the reader. Cheap when nothing is pending.
    fn commit_pending(&self) {
        if !self.dirty.load(Ordering::SeqCst) {
            return;
        }
        let mut writer = self.writer.lock();
        if self.dirty.load(Ordering::SeqCst) {
            self.commit(&mut writer);
        }
    }

    /// Commits pending writes every `COMMIT_INTERVAL` on the async runtime.
    pub fn start_background_commits(self: &Arc<Self>) {
        let me = Arc::clone(self);
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(COMMIT_INTERVAL);
            loop {
                tick.tick().await;
                let s = Arc::clone(&me);
                let _ = tokio::task::spawn_blocking(move || s.commit_pending()).await;
            }
        });
    }

    /// Commits and reloads the reader. The caller holds the writer lock, so no write can land in
    /// between; `dirty` is cleared only at the end, so a searcher that finds it set waits for the
    /// lock and therefore for this commit and reload.
    fn commit(&self, writer: &mut IndexWriter<TantivyDocument>) {
        let t0 = std::time::Instant::now();
        if let Err(e) = writer.commit() {
            tracing::warn!("Failed to commit search index: {e}");
            return;
        }
        let ms = t0.elapsed().as_millis();
        if ms > 2000 {
            tracing::warn!(
                "Search index commit took {ms} ms (index dir: {}); check antivirus or sync tools on that folder",
                self.index_dir
            );
        }
        if let Err(e) = self.reader.reload() {
            tracing::warn!("Failed to reload search index: {e}");
        }
        self.dirty.store(false, Ordering::SeqCst);
    }

    fn replace_item_document(
        &self,
        writer: &IndexWriter<TantivyDocument>,
        item: &Item,
        page_title: &str,
    ) {
        writer.delete_term(Term::from_field_text(self.fields.item_id, item.id.as_str()));
        let result = writer.add_document(doc!(
            self.fields.doc_type => TYPE_ITEM,
            self.fields.page_id => item.page_id.as_str(),
            self.fields.page_title => page_title,
            self.fields.item_id => item.id.as_str(),
            self.fields.body => item.body.as_str(),
        ));
        if let Err(e) = result {
            tracing::warn!("Failed to index item {}: {e}", item.id);
        }
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        let Some(mut analyzer): Option<TextAnalyzer> = self.index.tokenizers().get(TOKENIZER)
        else {
            return vec![text.to_lowercase()];
        };
        let mut tokens = Vec::new();
        let mut stream = analyzer.token_stream(text);
        while stream.advance() {
            tokens.push(stream.token().text.clone());
        }
        tokens
    }

    fn page_name_query(&self, query: &str, contains: bool) -> Option<Box<dyn Query>> {
        let tokens = self.tokenize(query);
        if tokens.is_empty() {
            return None;
        }
        let mut clauses: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for token in tokens {
            let pattern = if contains {
                format!(".*{}.*", regex::escape(&token))
            } else {
                format!("{}.*", regex::escape(&token))
            };
            let q = RegexQuery::from_pattern(&pattern, self.fields.page_title).ok()?;
            clauses.push((Occur::Must, Box::new(q)));
        }
        clauses.push((
            Occur::Must,
            Box::new(TermQuery::new(
                Term::from_field_text(self.fields.doc_type, TYPE_PAGE),
                IndexRecordOption::Basic,
            )),
        ));
        Some(Box::new(BooleanQuery::new(clauses)))
    }

    fn search_page_names_by(
        &self,
        query: &str,
        max_results: usize,
        contains: bool,
    ) -> Vec<SearchResult> {
        let Some(q) = self.page_name_query(query, contains) else {
            return Vec::new();
        };
        let searcher = self.reader.searcher();
        let Ok(top) = searcher.search(
            &q,
            &TopDocs::with_limit(max_results.max(1)).order_by_score(),
        ) else {
            return Vec::new();
        };
        let mut results = Vec::new();
        for (score, addr) in top {
            let Ok(doc) = searcher.doc::<TantivyDocument>(addr) else {
                continue;
            };
            let page_id = get_str(&doc, self.fields.page_id);
            let title = get_str(&doc, self.fields.page_title);
            let Ok(page_id) = Hmtid::parse(&page_id) else {
                continue;
            };
            results.push(SearchResult {
                page_id,
                page_title: title.clone(),
                item_id: String::new(),
                snippet: title,
                score,
            });
        }
        results
    }
}

fn get_str(doc: &TantivyDocument, field: Field) -> String {
    doc.get_first(field)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

impl SearchIndex for TantivySearchService {
    fn index_page(&self, page: &Page) {
        let writer = self.writer.lock();
        // Delete the existing page document (pageId AND type)
        let delete = BooleanQuery::new(vec![
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.fields.page_id, page.id.as_str()),
                    IndexRecordOption::Basic,
                )) as Box<dyn Query>,
            ),
            (
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.fields.doc_type, TYPE_PAGE),
                    IndexRecordOption::Basic,
                )),
            ),
        ]);
        if let Err(e) = writer.delete_query(Box::new(delete)) {
            tracing::warn!("Failed to delete page document {}: {e}", page.id);
        }
        let result = writer.add_document(doc!(
            self.fields.doc_type => TYPE_PAGE,
            self.fields.page_id => page.id.as_str(),
            self.fields.page_title => page.title().as_str(),
            self.fields.item_id => "",
        ));
        if let Err(e) = result {
            tracing::warn!("Failed to index page {}: {e}", page.id);
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    fn index_item(&self, item: &Item, page_title: &str) {
        let writer = self.writer.lock();
        self.replace_item_document(&writer, item, page_title);
        self.dirty.store(true, Ordering::SeqCst);
    }

    fn index_items(&self, items: &[Item], page_title: &str) {
        if items.is_empty() {
            return;
        }
        let writer = self.writer.lock();
        for item in items {
            self.replace_item_document(&writer, item, page_title);
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    fn remove_items(&self, item_ids: &[String]) {
        if item_ids.is_empty() {
            return;
        }
        let writer = self.writer.lock();
        for id in item_ids {
            writer.delete_term(Term::from_field_text(self.fields.item_id, id));
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    fn flush(&self) {
        self.commit_pending();
    }

    fn remove_page(&self, page_id: &str) {
        let writer = self.writer.lock();
        writer.delete_term(Term::from_field_text(self.fields.page_id, page_id));
        self.dirty.store(true, Ordering::SeqCst);
    }

    fn remove_item(&self, item_id: &str) {
        let writer = self.writer.lock();
        writer.delete_term(Term::from_field_text(self.fields.item_id, item_id));
        self.dirty.store(true, Ordering::SeqCst);
    }

    fn search_page_names(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        // A search must see everything saved before it, so pending writes are committed first
        self.commit_pending();
        let max = if max_results == 0 { 20 } else { max_results };
        // Try prefix search first (query*), then contains (*query*)
        let results = self.search_page_names_by(query, max, false);
        if !results.is_empty() {
            return results;
        }
        self.search_page_names_by(query, max, true)
    }

    fn search_full_text(&self, query: &str, max_results: usize) -> Vec<SearchResult> {
        self.commit_pending();
        let max = if max_results == 0 { 20 } else { max_results };
        let searcher = self.reader.searcher();
        let mut parser = QueryParser::for_index(&self.index, vec![self.fields.body]);
        parser.set_conjunction_by_default();
        let (q, errors) = parser.parse_query_lenient(query);
        if !errors.is_empty() {
            tracing::debug!("Query '{query}' parsed with errors: {errors:?}");
        }
        let Ok(top) = searcher.search(&q, &TopDocs::with_limit(max.max(1)).order_by_score()) else {
            return Vec::new();
        };
        let snippets = SnippetGenerator::create(&searcher, &*q, self.fields.body)
            .ok()
            .map(|mut g| {
                g.set_max_num_chars(100);
                g
            });
        let mut results = Vec::new();
        for (score, addr) in top {
            let Ok(doc) = searcher.doc::<TantivyDocument>(addr) else {
                continue;
            };
            let page_id = get_str(&doc, self.fields.page_id);
            let Ok(page_id) = Hmtid::parse(&page_id) else {
                continue;
            };
            let body = get_str(&doc, self.fields.body);
            let mut snippet = body.clone();
            if let Some(g) = &snippets {
                let mut s = g.snippet(&body);
                if !s.is_empty() {
                    s.set_snippet_prefix_postfix("<mark>", "</mark>");
                    snippet = s.to_html();
                }
            }
            results.push(SearchResult {
                page_id,
                page_title: get_str(&doc, self.fields.page_title),
                item_id: get_str(&doc, self.fields.item_id),
                snippet,
                score,
            });
        }
        results
    }

    fn is_empty(&self) -> bool {
        self.commit_pending();
        self.reader.searcher().num_docs() == 0
    }

    fn delete_all(&self) {
        let mut writer = self.writer.lock();
        if let Err(e) = writer.delete_all_documents() {
            tracing::warn!("Failed to clear search index: {e}");
        }
        self.commit(&mut writer);
        tracing::info!("Search index cleared");
    }
}
