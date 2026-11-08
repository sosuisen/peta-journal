//! Runs the `main.js` of an extension function with QuickJS (see ADR 009).
//!
//! The script never touches the database. What it asks for is collected as requests, and the
//! caller writes them after the script has ended. So a failed script leaves nothing behind.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

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

/// Runs extension scripts. Each run gets a new QuickJS runtime, so extensions share nothing.
pub struct ExtensionRuntime {
    #[allow(dead_code)]
    splitter: Arc<FlatMarkdownSplitter>,
    ids: Arc<IdGenerator>,
}

impl ExtensionRuntime {
    pub fn new(splitter: Arc<FlatMarkdownSplitter>, ids: Arc<IdGenerator>) -> ExtensionRuntime {
        ExtensionRuntime { splitter, ids }
    }

    /// Runs the `main.js` in `dir` for `event`, and returns the items it asked for.
    pub fn run(&self, dir: &Path, event: &ExtensionEvent) -> anyhow::Result<Vec<ItemRequest>> {
        let source = std::fs::read_to_string(dir.join(MAIN_FILE))
            .with_context(|| format!("cannot read {MAIN_FILE} in {}", dir.display()))?;
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;
        let requests = Rc::new(RefCell::new(Vec::new()));
        let ExtensionEvent::PageCreate(page) = event;
        context.with(|ctx| -> anyhow::Result<()> {
            self.install_api(&ctx, &requests, page.id.clone())?;
            js(&ctx, ctx.eval::<(), _>(source))?;
            let handler: Value = ctx.globals().get("onPageCreate")?;
            if let Some(handler) = handler.as_function() {
                let page = to_js(&ctx, page)?;
                js(&ctx, handler.call::<_, ()>((page,)))?;
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
        ctx.globals().set("peta", peta)?;
        Ok(())
    }
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

    use super::{ExtensionEvent, ExtensionRuntime, ItemRequest, PageInfo};
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
}
