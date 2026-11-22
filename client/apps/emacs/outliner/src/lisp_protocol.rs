//! Writes `lisp/peta-journal-protocol.el` from the facet registry of the protocol (ADR 078).
//! The names of the lines come from the Rust types alone, so Lisp does not repeat them.

use crate::protocol::Outbound;
use facet_generate::reflection::format::{
    ContainerFormat, Doc, EnumTagging, Format, Named, VariantFormat,
};
use facet_generate::reflection::RegistryBuilder;
use facet_generate::Registry;
use std::collections::BTreeMap;
use std::fmt::Write;

/// The prefix of every name in the Lisp file.
const PREFIX: &str = "peta-journal-protocol";

/// The head of the Lisp file.
const HEADER: &str = r#";;; peta-journal-protocol.el --- The lines of the shell -*- lexical-binding: t; -*-

;;; Commentary:

;; Written from the Rust types of src/protocol.rs (ADR 078).  Do not edit it by hand.
;; Write it again with:
;;   PETA_WRITE_LISP=1 cargo test -p outliner-emacs the_committed_lisp_protocol

;;; Code:

"#;

/// The file `lisp/peta-journal-protocol.el`: the Lisp of the lines the shell sends to Emacs.
pub fn protocol_file() -> String {
    let registry = RegistryBuilder::new()
        .add_type::<Outbound>()
        .and_then(RegistryBuilder::build)
        .expect("the protocol has a registry");
    format!(
        "{HEADER}{}(provide 'peta-journal-protocol)\n;;; peta-journal-protocol.el ends here\n",
        lisp_of(&registry)
    )
}

/// The Lisp of the types in `registry`.
pub fn lisp_of(registry: &Registry) -> String {
    let mut lisp = String::new();
    for (name, format) in registry {
        let kind = lisp_name(&name.name);
        match format {
            ContainerFormat::Struct(fields, _) => readers(&mut lisp, &kind, fields),
            ContainerFormat::Enum(variants, tagging, doc) => {
                if variants
                    .values()
                    .all(|variant| matches!(variant.value, VariantFormat::Unit))
                {
                    names(&mut lisp, &kind, variants, doc);
                } else {
                    let kinds = kinds(registry, variants);
                    fields_of_kinds(&mut lisp, &kind, &kinds);
                    handler(&mut lisp, &kind, tagging, &kinds);
                }
            }
            other => panic!("no Lisp is written for {name}: {other:?}"),
        }
    }
    lisp
}

/// A kind of an enum with data: its name as it is sent, and the names of its fields.
struct Kind {
    name: String,
    fields: Vec<String>,
}

/// The kinds of an enum with data. The fields of a kind that holds a struct are those of
/// the struct, as the line has them beside its tag.
fn kinds(registry: &Registry, variants: &BTreeMap<u32, Named<VariantFormat>>) -> Vec<Kind> {
    variants
        .values()
        .map(|variant| {
            let fields = match &variant.value {
                VariantFormat::Struct(fields) => fields.as_slice(),
                VariantFormat::NewType(format) => match format.as_ref() {
                    Format::TypeName(name) => match registry.get(name) {
                        Some(ContainerFormat::Struct(fields, _)) => fields.as_slice(),
                        other => panic!("no Lisp is written for a kind holding {other:?}"),
                    },
                    other => panic!("no Lisp is written for a kind holding {other:?}"),
                },
                other => panic!(
                    "no Lisp is written for the kind {}: {other:?}",
                    variant.name
                ),
            };
            Kind {
                name: variant.name.clone(),
                fields: fields.iter().map(|field| field.name.clone()).collect(),
            }
        })
        .collect()
}

/// A reader for each field of the struct `kind`.
fn readers(lisp: &mut String, kind: &str, fields: &[Named<Format>]) {
    for field in fields {
        let doc = docstring(
            &field.doc,
            &format!("The field {} of the type {}.", field.name, kind),
        );
        let _ = write!(
            lisp,
            "(defsubst {PREFIX}-{kind}-{} (value)\n  {doc}\n  (plist-get value :{}))\n\n",
            lisp_name(&field.name),
            field.name
        );
    }
}

/// The list of the names of the enum of names `kind`.
fn names(lisp: &mut String, kind: &str, variants: &BTreeMap<u32, Named<VariantFormat>>, doc: &Doc) {
    let names: Vec<String> = variants
        .values()
        .map(|variant| format!("\"{}\"", variant.name))
        .collect();
    let doc = docstring(doc, &format!("The names of the type {kind}."));
    let _ = write!(
        lisp,
        "(defconst {PREFIX}-{kind}-names '({})\n  {doc})\n\n",
        names.join(" ")
    );
}

/// The fields of each kind of `kind`, by the keyword of the kind.
fn fields_of_kinds(lisp: &mut String, kind: &str, kinds: &[Kind]) {
    let lists: Vec<String> = kinds
        .iter()
        .map(|each| {
            let mut list = format!("(:{}", lisp_name(&each.name));
            for field in &each.fields {
                let _ = write!(list, " \"{field}\"");
            }
            list.push(')');
            list
        })
        .collect();
    let doc = docstring(
        &Doc::default(),
        &format!(
            "The fields of each kind of the type {kind}, in the order its handler takes them."
        ),
    );
    let _ = write!(
        lisp,
        "(defconst {PREFIX}-{kind}-fields\n  '({})\n  {doc})\n\n",
        lists.join(" ")
    );
}

/// The function that calls the handler of the kind of a `kind`.
fn handler(lisp: &mut String, kind: &str, tagging: &EnumTagging, kinds: &[Kind]) {
    let _ = write!(
        lisp,
        concat!(
            "(defun {PREFIX}-handle-{kind} (value handlers)\n",
            "  \"Call the function in HANDLERS for the kind of VALUE, with its fields.\n",
            "HANDLERS is a plist from the keyword of each kind to its function.\n",
            "The fields come in the order of `{PREFIX}-{kind}-fields'.\"\n",
        ),
        PREFIX = PREFIX,
        kind = kind
    );
    let (head, fields) = match tagging {
        EnumTagging::Internal { tag } => (format!("(plist-get value :{tag})"), "value"),
        EnumTagging::External => ("value".to_string(), "fields"),
        EnumTagging::Adjacent { .. } => panic!("no Lisp is written for the enum {kind}"),
    };
    let _ = write!(lisp, "  (pcase {head}");
    for each in kinds {
        let pattern = match tagging {
            EnumTagging::Internal { .. } => format!("\"{}\"", each.name),
            _ => format!("`(:{} ,fields)", each.name),
        };
        let arguments: Vec<String> = each
            .fields
            .iter()
            .map(|field| format!("(plist-get {fields} :{field})"))
            .collect();
        let _ = write!(
            lisp,
            "\n    ({pattern}\n     (funcall (plist-get handlers :{})\n              {}))",
            lisp_name(&each.name),
            arguments.join(" ")
        );
    }
    lisp.push_str("))\n\n");
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
        otherwise
    } else {
        text.trim()
    };
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    // The first line starts behind the indent of two and the quote
    let mut lines = vec![String::new()];
    let mut width = 3;
    for word in escaped.split_whitespace() {
        let line = lines.last_mut().expect("there is a line");
        if !line.is_empty() && width + 1 + word.chars().count() > WIDTH {
            lines.push(word.to_string());
            width = word.chars().count();
        } else {
            if !line.is_empty() {
                line.push(' ');
                width += 1;
            }
            line.push_str(word);
            width += word.chars().count();
        }
    }
    format!("\"{}\"", lines.join("\n"))
}

/// The width of a line of a docstring, with room for its closing quote and parenthesis, as
/// the byte compiler asks for at most 80.
const WIDTH: usize = 76;
