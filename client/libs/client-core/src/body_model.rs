//! What a row gives the shell to draw its body (ADR 042). The web shell draws the AST itself
//! (ADR 041); a native shell draws the `RichText` that the core makes from the AST. The core
//! and the view model take the kind as a type parameter, and each shell picks its own. The
//! core has no default: the web shell wraps the AST in a type of its own, as only the crate of
//! the trait or of the type may implement the trait for the type.

use crate::asts::parse;
use flatmarkdown_ast::Node;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Mutex;

/// The body of a row as a shell draws it, made from the AST of the body.
pub trait BodyModel: Clone + Debug + PartialEq + Serialize + Send + Sync + 'static {
    /// False when the model is made without the AST. Then the view keeps nothing of the bodies.
    const FROM_AST: bool = true;

    /// What is shown of the body whose AST is `ast`. The page of a tag lies under `tags_dir`.
    fn of(ast: Node, tags_dir: &str) -> Self;

    /// What is shown of the body `markdown`.
    fn of_markdown(markdown: &str, tags_dir: &str) -> Self {
        Self::of(parse(markdown), tags_dir)
    }
}

/// A shell that draws the Markdown itself, such as the Emacs shell, takes no body model. Its
/// bodies are not parsed.
impl BodyModel for () {
    const FROM_AST: bool = false;

    fn of(_ast: Node, _tags_dir: &str) {}

    fn of_markdown(_markdown: &str, _tags_dir: &str) {}
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

impl<B: BodyModel> Bodies<B> {
    /// What is shown of each of these bodies, in their order. What was shown of bodies that are
    /// not among them is forgotten, so that what is kept stays as large as what is shown.
    pub fn of<'a>(&self, bodies: impl IntoIterator<Item = &'a str>, tags_dir: &str) -> Vec<B> {
        if !B::FROM_AST {
            return bodies
                .into_iter()
                .map(|body| B::of_markdown(body, tags_dir))
                .collect();
        }
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
                    .unwrap_or_else(|| B::of_markdown(body, tags_dir));
                kept.insert(body.to_string(), shown.clone());
                shown
            })
            .collect();
        last.bodies = kept;
        shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn kept<B>(bodies: &Bodies<B>) -> usize {
        bodies.last.lock().unwrap().bodies.len()
    }

    /// A body model that counts how many bodies were made from their AST.
    #[derive(Clone, Debug, PartialEq, Serialize)]
    struct Counted;

    static MADE: AtomicUsize = AtomicUsize::new(0);

    impl BodyModel for Counted {
        fn of(_ast: Node, _tags_dir: &str) -> Self {
            MADE.fetch_add(1, Ordering::SeqCst);
            Counted
        }
    }

    // ASTから作る本文は、前の表示と同じ本文なら作り直さない。表示から消えた本文は忘れる。
    #[test]
    fn a_body_from_the_ast_is_made_again_only_when_it_is_new() {
        let bodies = Bodies::<Counted>::default();
        bodies.of(["a", "b"], "");
        bodies.of(["a", "c"], "");
        assert_eq!(MADE.load(Ordering::SeqCst), 3);
        assert_eq!(kept(&bodies), 2);
    }

    // Markdownを自分で描くシェルの本文は、何も覚えない。
    #[test]
    fn the_bodies_of_a_shell_that_draws_the_markdown_are_not_kept() {
        let bodies = Bodies::<()>::default();
        assert_eq!(bodies.of(["# a", "b"], ""), vec![(), ()]);
        assert_eq!(kept(&bodies), 0);
    }
}
