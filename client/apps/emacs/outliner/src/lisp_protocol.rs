//! Writes `lisp/peta-journal-protocol.el` from the facet registry of the protocol (ADR 078).
//! The names of the lines come from the Rust types alone, so Lisp does not repeat them.

use facet_generate::reflection::format::{ContainerFormat, Doc};
use facet_generate::Registry;
use std::fmt::Write;

/// The prefix of every name in the Lisp file.
const PREFIX: &str = "peta-journal-protocol";

/// The Lisp of the types in `registry`.
pub fn lisp_of(registry: &Registry) -> String {
    let mut lisp = String::new();
    for (name, format) in registry {
        let kind = lisp_name(&name.name);
        if let ContainerFormat::Struct(fields, _) = format {
            for field in fields {
                let doc = docstring(&field.doc, &format!("The {} of a {}.", field.name, kind));
                let _ = write!(
                    lisp,
                    "(defsubst {PREFIX}-{kind}-{} (value)\n  {doc}\n  (plist-get value :{}))\n\n",
                    lisp_name(&field.name),
                    field.name
                );
            }
        }
    }
    lisp
}

/// A name of Rust, `RowView` or `page_id`, as Lisp writes it: `row-view`, `page-id`.
fn lisp_name(name: &str) -> String {
    let mut lisp = String::new();
    for (at, letter) in name.chars().enumerate() {
        if letter == '_' {
            lisp.push('-');
        } else if letter.is_uppercase() {
            if at > 0 {
                lisp.push('-');
            }
            lisp.extend(letter.to_lowercase());
        } else {
            lisp.push(letter);
        }
    }
    lisp
}

/// The doc comment as a Lisp string, or `otherwise` when there is none.
fn docstring(doc: &Doc, otherwise: &str) -> String {
    let text = doc.comments().join(" ");
    let text = if text.trim().is_empty() {
        otherwise.to_string()
    } else {
        text.trim().to_string()
    };
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}
