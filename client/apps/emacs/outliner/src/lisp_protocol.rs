//! Writes `lisp/peta-journal-protocol.el` from the facet registry of the protocol (ADR 078).
//! The names of the lines come from the Rust types alone, so Lisp does not repeat them.

use facet_generate::reflection::format::{
    ContainerFormat, Doc, EnumTagging, Format, Named, VariantFormat,
};
use facet_generate::Registry;
use std::collections::BTreeMap;
use std::fmt::Write;

/// The prefix of every name in the Lisp file.
const PREFIX: &str = "peta-journal-protocol";

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
        let doc = docstring(&field.doc, &format!("The {} of a {}.", field.name, kind));
        let _ = write!(
            lisp,
            "(defsubst {PREFIX}-{kind}-{} (value)
  {doc}
  (plist-get value :{}))

",
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
    let doc = docstring(doc, &format!("The names of a {kind}."));
    let _ = write!(
        lisp,
        "(defconst {PREFIX}-{kind}-names '({})
  {doc})

",
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
    let _ = write!(
        lisp,
        "(defconst {PREFIX}-{kind}-fields
  '({})
  \"The fields of each kind of a {kind}, in the order its handler takes them.\")

",
        lists.join(" ")
    );
}

/// The function that calls the handler of the kind of a `kind`.
fn handler(lisp: &mut String, kind: &str, tagging: &EnumTagging, kinds: &[Kind]) {
    let _ = write!(
        lisp,
        "(defun {PREFIX}-handle-{kind} (value handlers)
  \"Call the function in HANDLERS for the kind of VALUE, with its fields.
HANDLERS is a plist from the keyword of each kind to its function.
The fields come in the order of `{PREFIX}-{kind}-fields'.\"
"
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
            "
    ({pattern}
     (funcall (plist-get handlers :{})
              {}))",
            lisp_name(&each.name),
            arguments.join(" ")
        );
    }
    lisp.push_str(
        "))

",
    );
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
