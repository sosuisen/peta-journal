//! What a row gives a native shell to draw its body (ADR 042).

use client_core::{BodyModel, RichText};
use flatmarkdown_ast::Node;
use serde::Serialize;

/// The body of a row as it crosses the border: the `RichText` that a native shell draws, and
/// the AST that it drew from before (ADR 041).
///
/// The AST stays here while the shells are switched to `RichText` one by one: each shell still
/// draws from it until its turn. It goes when the last shell no longer reads it, and the row
/// then gives `RichText` alone.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BorderBody {
    pub ast: Node,
    pub rich: RichText,
}

impl BodyModel for BorderBody {
    fn of(ast: Node, tags_dir: &str) -> BorderBody {
        let rich = RichText::of(ast.clone(), tags_dir);
        BorderBody { ast, rich }
    }
}

/// The core under the border.
pub type BorderApp = client_core::PetaApp<BorderBody>;

#[cfg(test)]
mod tests {
    use super::*;

    // 境界を渡る本文は、ASTと、そこからcoreが作ったRichTextの両方を持つ。
    #[test]
    fn the_body_at_the_border_has_both_the_ast_and_its_rich_text() {
        let ast = flatmarkdown::parse_body("**a** #t");
        let body = BorderBody::of(ast.clone(), "tags/");
        assert_eq!(body.ast, ast);
        assert_eq!(body.rich, RichText::of(ast, "tags/"));
        assert!(!body.rich.blocks.is_empty());
    }
}
