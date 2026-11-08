//! Runs the `main.js` of an extension function with QuickJS (see ADR 009).
//!
//! The script never touches the database. What it asks for is collected as requests, and the
//! caller writes them after the script has ended. So a failed script leaves nothing behind.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context as _};
use hmtid::Hmtid;
use rquickjs::{CatchResultExt, Context, Ctx, Function, Object, Runtime, Value};
use serde::{Deserialize, Serialize};

use crate::application::flat_markdown_splitter::FlatMarkdownSplitter;
use crate::domain::id_generator::IdGenerator;
use crate::domain::model::ItemPosition;

const MAIN_FILE: &str = "main.js";

/// The page an event is about, as the script sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub id: String,
    pub file_name: String,
    pub dir_path: String,
    pub page_type: String,
}

/// What happened, which the script may answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionEvent {
    PageCreate(PageInfo),
    /// Tags were added to the tags property of the page.
    PageTagAdd(PageInfo, Vec<String>),
}

impl ExtensionEvent {
    fn page(&self) -> &PageInfo {
        match self {
            ExtensionEvent::PageCreate(page) | ExtensionEvent::PageTagAdd(page, _) => page,
        }
    }

    /// The name of the global function that answers the event.
    fn handler_name(&self) -> &'static str {
        match self {
            ExtensionEvent::PageCreate(_) => "onPageCreate",
            ExtensionEvent::PageTagAdd(..) => "onPageTagAdd",
        }
    }

    fn call<'js>(&self, ctx: &Ctx<'js>, handler: &Function<'js>) -> anyhow::Result<()> {
        let page = to_js(ctx, self.page())?;
        let result = match self {
            ExtensionEvent::PageCreate(_) => handler.call::<_, ()>((page,)),
            ExtensionEvent::PageTagAdd(_, tags) => handler.call::<_, ()>((page, to_js(ctx, tags)?)),
        };
        js(ctx, result)
    }
}

/// An item the script asked for. Its id is given out at once, so the script can place the
/// next item after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemRequest {
    pub id: Hmtid,
    pub page_id: String,
    pub position: ItemPosition,
    pub level: i32,
    pub body: String,
    pub transclude: Option<String>,
}

#[derive(Deserialize)]
struct CreateItemArgs {
    position: String,
    level: i32,
    body: String,
    #[serde(default)]
    transclude: Option<String>,
}

/// How long a run may take and how much memory it may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtensionLimits {
    pub time: Duration,
    /// In bytes.
    pub memory: usize,
}

impl Default for ExtensionLimits {
    /// A run blocks the creation of a page, so it gets little time. A template takes a few
    /// milliseconds.
    fn default() -> Self {
        ExtensionLimits {
            time: Duration::from_secs(2),
            memory: 32 * 1024 * 1024,
        }
    }
}

/// Runs extension scripts. Each run gets a new QuickJS runtime, so extensions share nothing.
pub struct ExtensionRuntime {
    #[allow(dead_code)]
    splitter: Arc<FlatMarkdownSplitter>,
    ids: Arc<IdGenerator>,
    limits: ExtensionLimits,
}

impl ExtensionRuntime {
    pub fn new(splitter: Arc<FlatMarkdownSplitter>, ids: Arc<IdGenerator>) -> ExtensionRuntime {
        ExtensionRuntime {
            splitter,
            ids,
            limits: ExtensionLimits::default(),
        }
    }

    pub fn with_limits(self, limits: ExtensionLimits) -> ExtensionRuntime {
        ExtensionRuntime { limits, ..self }
    }

    /// Runs the `main.js` in `dir` for `event`, and returns the items it asked for.
    pub fn run(&self, dir: &Path, event: &ExtensionEvent) -> anyhow::Result<Vec<ItemRequest>> {
        let source = std::fs::read_to_string(dir.join(MAIN_FILE))
            .with_context(|| format!("cannot read {MAIN_FILE} in {}", dir.display()))?;
        let runtime = Runtime::new()?;
        runtime.set_memory_limit(self.limits.memory);
        let deadline = Instant::now() + self.limits.time;
        runtime.set_interrupt_handler(Some(Box::new(move || Instant::now() > deadline)));
        let context = Context::full(&runtime)?;
        let requests = Rc::new(RefCell::new(Vec::new()));
        context.with(|ctx| -> anyhow::Result<()> {
            self.install_api(&ctx, &requests, event.page().id.clone(), dir.to_path_buf())?;
            js(&ctx, ctx.eval::<(), _>(source))?;
            let handler: Value = ctx.globals().get(event.handler_name())?;
            if let Some(handler) = handler.as_function() {
                event.call(&ctx, handler)?;
            }
            Ok(())
        })?;
        Ok(requests.take())
    }

    /// Puts the `peta` object, the only way out of the script, into its globals.
    fn install_api<'js>(
        &self,
        ctx: &Ctx<'js>,
        requests: &Rc<RefCell<Vec<ItemRequest>>>,
        page_id: String,
        dir: PathBuf,
    ) -> anyhow::Result<()> {
        let peta = Object::new(ctx.clone())?;
        let ids = Arc::clone(&self.ids);
        let requests = Rc::clone(requests);
        let create_item = Function::new(
            ctx.clone(),
            move |ctx: Ctx<'js>, args: Value<'js>| -> rquickjs::Result<String> {
                let args: CreateItemArgs = from_js(&ctx, args)?;
                let position = ItemPosition::parse(&args.position)
                    .map_err(|e| rquickjs::Exception::throw_type(&ctx, &e))?;
                let id = ids.generate_item_id();
                requests.borrow_mut().push(ItemRequest {
                    id: id.clone(),
                    page_id: page_id.clone(),
                    position,
                    level: args.level,
                    body: args.body,
                    transclude: args.transclude,
                });
                Ok(id.to_string())
            },
        )?;
        peta.set("createItem", create_item)?;
        let read_dir = dir.clone();
        let files = Function::new(
            ctx.clone(),
            move |ctx: Ctx<'js>| -> rquickjs::Result<Vec<String>> {
                file_names(&dir)
                    .map_err(|e| rquickjs::Exception::throw_internal(&ctx, &e.to_string()))
            },
        )?;
        peta.set("files", files)?;
        let read_file = Function::new(
            ctx.clone(),
            move |ctx: Ctx<'js>, name: String| -> rquickjs::Result<String> {
                read_own_file(&read_dir, &name)
                    .map_err(|e| rquickjs::Exception::throw_internal(&ctx, &e.to_string()))
            },
        )?;
        peta.set("readFile", read_file)?;
        ctx.globals().set("peta", peta)?;
        Ok(())
    }
}

/// Reads the file `name` directly in `dir`. A name with a folder in it is refused, so an
/// extension cannot read outside its own folder.
fn read_own_file(dir: &Path, name: &str) -> anyhow::Result<String> {
    let mut components = Path::new(name).components();
    let (Some(std::path::Component::Normal(file)), None) = (components.next(), components.next())
    else {
        return Err(anyhow!("not a file name of the extension's folder: {name}"));
    };
    Ok(std::fs::read_to_string(dir.join(file))?)
}

/// The names of the files directly in `dir`, sorted. Folders are left out.
fn file_names(dir: &Path) -> std::io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    Ok(names)
}

/// Turns a JS exception into an error with its message and stack.
fn js<T>(ctx: &Ctx<'_>, result: rquickjs::Result<T>) -> anyhow::Result<T> {
    result.catch(ctx).map_err(|e| anyhow!("{e}"))
}

fn to_js<'js, T: Serialize>(ctx: &Ctx<'js>, value: &T) -> anyhow::Result<Value<'js>> {
    let json = serde_json::to_string(value)?;
    js(ctx, ctx.json_parse(json))
}

fn from_js<'js, T: for<'de> Deserialize<'de>>(
    ctx: &Ctx<'js>,
    value: Value<'js>,
) -> rquickjs::Result<T> {
    let json = ctx
        .json_stringify(value)?
        .map(|s| s.to_string())
        .transpose()?
        .unwrap_or_default();
    serde_json::from_str(&json).map_err(|e| rquickjs::Exception::throw_type(ctx, &e.to_string()))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::{ExtensionEvent, ExtensionLimits, ExtensionRuntime, ItemRequest, PageInfo};
    use crate::application::flat_markdown_splitter::FlatMarkdownSplitter;
    use crate::application::markdown_parser::MarkdownParser;
    use crate::domain::id_generator::IdGenerator;
    use crate::domain::model::ItemPosition;

    fn runtime() -> ExtensionRuntime {
        ExtensionRuntime::new(
            Arc::new(FlatMarkdownSplitter::new(MarkdownParser)),
            Arc::new(IdGenerator::new()),
        )
    }

    fn function_dir(main_js: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.js"), main_js).unwrap();
        dir
    }

    fn page() -> PageInfo {
        PageInfo {
            id: "page-1".to_string(),
            file_name: "2026-11-08".to_string(),
            dir_path: "journals/".to_string(),
            page_type: "journal".to_string(),
        }
    }

    // onPageCreateはページの情報を受け取り、createItemで頼んだ項目が操作として返る。
    #[test]
    fn on_page_create_gets_the_page_and_returns_the_items_it_asks_for() {
        let dir = function_dir(
            "globalThis.onPageCreate = (page) => {
                peta.createItem({ position: 'last', level: 1, body: page.fileName + ' ' + page.pageType });
            };",
        );

        let requests = runtime()
            .run(dir.path(), &ExtensionEvent::PageCreate(page()))
            .unwrap();

        assert_eq!(requests.len(), 1);
        let ItemRequest {
            page_id,
            position,
            level,
            body,
            transclude,
            ..
        } = &requests[0];
        assert_eq!(page_id, "page-1");
        assert_eq!(*position, ItemPosition::Last);
        assert_eq!(*level, 1);
        assert_eq!(body, "2026-11-08 journal");
        assert_eq!(*transclude, None);
    }

    // createItemは新しい項目のIDを返し、そのIDを次の項目の置き場所に使える。
    #[test]
    fn create_item_returns_an_id_that_places_the_next_item() {
        let dir = function_dir(
            "globalThis.onPageCreate = () => {
                const first = peta.createItem({ position: 'last', level: 1, body: 'a' });
                peta.createItem({ position: 'after:' + first, level: 2, body: 'b' });
            };",
        );

        let requests = runtime()
            .run(dir.path(), &ExtensionEvent::PageCreate(page()))
            .unwrap();

        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests[1].position,
            ItemPosition::After(requests[0].id.clone())
        );
        assert_ne!(requests[0].id, requests[1].id);
    }

    // onPageTagAddが、ページの情報と足されたタグを受け取る。
    #[test]
    fn on_page_tag_add_gets_the_page_and_the_added_tags() {
        let dir = function_dir(
            "globalThis.onPageTagAdd = (page, tags) => {
                peta.createItem({ position: 'last', level: 1, body: page.fileName + ': ' + tags.join(',') });
            };",
        );
        let event = ExtensionEvent::PageTagAdd(page(), vec!["book".into(), "idea".into()]);

        let requests = runtime().run(dir.path(), &event).unwrap();

        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].body, "2026-11-08: book,idea");
    }

    // ページ作成の拡張は、タグの追加では呼ばれない。
    #[test]
    fn on_page_create_is_not_called_for_a_tag_add() {
        let dir = function_dir(
            "globalThis.onPageCreate = () => {
                peta.createItem({ position: 'last', level: 1, body: 'a' });
            };",
        );
        let event = ExtensionEvent::PageTagAdd(page(), vec!["book".into()]);

        let requests = runtime().run(dir.path(), &event).unwrap();

        assert!(requests.is_empty());
    }

    // 終わらない拡張は、時間の上限で失敗になる。
    #[test]
    fn an_extension_that_never_ends_fails_at_the_time_limit() {
        let dir = function_dir("globalThis.onPageCreate = () => { for (;;) {} };");
        let runtime = runtime().with_limits(ExtensionLimits {
            time: Duration::from_millis(100),
            ..ExtensionLimits::default()
        });

        let started = Instant::now();
        let result = runtime.run(dir.path(), &ExtensionEvent::PageCreate(page()));

        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    // メモリを使いすぎる拡張は、上限で失敗になる。
    #[test]
    fn an_extension_that_uses_too_much_memory_fails_at_the_limit() {
        let dir = function_dir(
            "globalThis.onPageCreate = () => {
                const parts = [];
                for (;;) { parts.push('x'.repeat(1024 * 1024)); }
            };",
        );
        let runtime = runtime().with_limits(ExtensionLimits {
            memory: 8 * 1024 * 1024,
            ..ExtensionLimits::default()
        });

        let started = Instant::now();
        let result = runtime.run(dir.path(), &ExtensionEvent::PageCreate(page()));

        let error = result.unwrap_err().to_string();
        assert!(error.contains("out of memory"), "{error}");
        assert!(started.elapsed() < ExtensionLimits::default().time);
    }

    // files()は、自分のフォルダのファイル名を名前順に返す。
    #[test]
    fn files_lists_the_file_names_of_its_own_folder_in_name_order() {
        let dir = function_dir(
            "globalThis.onPageCreate = () => {
                peta.createItem({ position: 'last', level: 1, body: peta.files().join(',') });
            };",
        );
        std::fs::write(dir.path().join("b.md"), "").unwrap();
        std::fs::write(dir.path().join("a.md"), "").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();

        let requests = runtime()
            .run(dir.path(), &ExtensionEvent::PageCreate(page()))
            .unwrap();

        assert_eq!(requests[0].body, "a.md,b.md,main.js");
    }

    // readFile(name)は、自分のフォルダのファイルを読む。
    #[test]
    fn read_file_reads_a_file_of_its_own_folder() {
        let dir = function_dir(
            "globalThis.onPageCreate = () => {
                peta.createItem({ position: 'last', level: 1, body: peta.readFile('template.md') });
            };",
        );
        std::fs::write(dir.path().join("template.md"), "日記の型").unwrap();

        let requests = runtime()
            .run(dir.path(), &ExtensionEvent::PageCreate(page()))
            .unwrap();

        assert_eq!(requests[0].body, "日記の型");
    }

    // readFile(name)は、フォルダの外のファイルを読めない。
    #[test]
    fn read_file_cannot_read_outside_its_own_folder() {
        let outer = tempfile::tempdir().unwrap();
        std::fs::write(outer.path().join("secret.txt"), "secret").unwrap();
        let dir = outer.path().join("function");
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(
            dir.join("main.js"),
            "globalThis.onPageCreate = () => {
                peta.createItem({ position: 'last', level: 1, body: peta.readFile('../secret.txt') });
            };",
        )
        .unwrap();

        let result = runtime().run(&dir, &ExtensionEvent::PageCreate(page()));

        assert!(result.is_err());
    }

    // 出来事の関数を持たない拡張は、何も頼まない。
    #[test]
    fn an_extension_without_the_event_function_asks_for_nothing() {
        let dir = function_dir("const unused = 1;");

        let requests = runtime()
            .run(dir.path(), &ExtensionEvent::PageCreate(page()))
            .unwrap();

        assert!(requests.is_empty());
    }

    // JSの例外は失敗になり、頼んだ操作は捨てられる。
    #[test]
    fn a_js_exception_fails_the_run_and_drops_what_it_asked_for() {
        let dir = function_dir(
            "globalThis.onPageCreate = () => {
                peta.createItem({ position: 'last', level: 1, body: 'a' });
                throw new Error('broken template');
            };",
        );

        let result = runtime().run(dir.path(), &ExtensionEvent::PageCreate(page()));

        assert!(result.is_err());
    }
}
