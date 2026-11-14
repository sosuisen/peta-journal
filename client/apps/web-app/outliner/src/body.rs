//! The body model of the web shell: the AST of each body, which it renders to HTML itself
//! (ADR 041, ADR 042). The core types with this model have short names here.

use client_core::BodyModel;
use flatmarkdown_ast::Node;
use serde::Serialize;

/// The AST of a body, as the core parsed it. It wraps the AST because only the crate of a
/// trait or of a type may implement the trait for the type: here, neither is ours.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct AstBody(pub Node);

impl BodyModel for AstBody {
    fn of(ast: Node, _tags_dir: &str) -> AstBody {
        AstBody(ast)
    }
}

/// The core, with the rows of its view model in [`AstBody`].
pub type PetaApp = client_core::PetaApp<AstBody>;
/// The view model, with its rows in [`AstBody`].
pub type ViewModel = client_core::ViewModel<AstBody>;
/// A row of the outline, with its body in [`AstBody`].
pub type Row = client_core::view::Row<AstBody>;
