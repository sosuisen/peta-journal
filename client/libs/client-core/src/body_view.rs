//! What a row gives the shell to draw its body (ADR 042). The web shell draws the AST itself
//! (ADR 041); a native shell draws the `RichText` that the core makes from the AST. The core
//! and the view model take the kind as a type parameter, and each shell picks its own.

use crate::asts::parse;
use flatmarkdown_ast::Node;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Mutex;

/// The body of a row as a shell draws it, made from the AST of the body.
pub trait BodyView: Clone + Debug + PartialEq + Serialize + Send + Sync + 'static {
    /// What is shown of the body whose AST is `ast`. The page of a tag lies under `tags_dir`.
    fn of(ast: Node, tags_dir: &str) -> Self;
}

/// The web shell draws the AST as it is.
impl BodyView for Node {
    fn of(ast: Node, _tags_dir: &str) -> Node {
        ast
    }
}

/// What the last view showed of each body. The view is made after every event, and most bodies
/// are the same as the time before, so a body is made again only when it is new.
#[derive(Debug)]
pub struct Bodies<B> {
    last: Mutex<Shown<B>>,
}

#[derive(Debug)]
struct Shown<B> {
    /// The folder of the tags that the bodies were made with. They are made again when it
    /// changes: the links of the tags lead under it.
    tags_dir: String,
    bodies: HashMap<String, B>,
}

impl<B> Default for Bodies<B> {
    fn default() -> Self {
        Bodies {
            last: Mutex::new(Shown {
                tags_dir: String::new(),
                bodies: HashMap::new(),
            }),
        }
    }
}

impl<B: BodyView> Bodies<B> {
    /// What is shown of each of these bodies, in their order. What was shown of bodies that are
    /// not among them is forgotten, so that what is kept stays as large as what is shown.
    pub fn of<'a>(&self, bodies: impl IntoIterator<Item = &'a str>, tags_dir: &str) -> Vec<B> {
        // A view that panicked while it held the lock left only whole entries behind
        let mut last = self
            .last
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if last.tags_dir != tags_dir {
            last.tags_dir = tags_dir.to_string();
            last.bodies.clear();
        }
        let mut kept = HashMap::with_capacity(last.bodies.len());
        let shown = bodies
            .into_iter()
            .map(|body| {
                let shown = kept
                    .get(body)
                    .or_else(|| last.bodies.get(body))
                    .cloned()
                    .unwrap_or_else(|| B::of(parse(body), tags_dir));
                kept.insert(body.to_string(), shown.clone());
                shown
            })
            .collect();
        last.bodies = kept;
        shown
    }
}
