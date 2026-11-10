//! Leptos renderer for the Flat Markdown AST.
//!
//! Produces the same HTML as the Angular renderer `flatmarkdown-ast2angular` (`FmNodeComponent`):
//! one element per node, with the same class names and ids, so the existing stylesheet applies.
//! Wikilinks and hashtags are plain `<a href>` elements; the host intercepts their clicks.

use std::collections::BTreeMap;

use flatmarkdown_ast::{extract_text, Node};
use leptos::prelude::*;
use serde_json::Value;

/// Link targets and classes for wikilinks and hashtags.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderOptions {
    /// Prepended to the wikilink URL, e.g. `"/pages/"`.
    pub wikilink_prefix: String,
    /// `class` of the wikilink anchor, e.g. `"wiki-link"`; empty for none.
    pub wikilink_class: String,
    /// Prepended to the hashtag value, e.g. `"/pages/"`.
    pub hashtag_prefix: String,
    /// `class` of the hashtag anchor, e.g. `"hashtag-link"`; empty for none.
    pub hashtag_class: String,
}

/// What a parent node tells its descendants; mirrors the Angular `RenderContext`.
#[derive(Clone, Copy)]
struct Context<'a> {
    options: &'a RenderOptions,
    /// Inside a header row: cells render as `<th>`.
    in_header_row: bool,
    /// Column alignments of the enclosing table.
    table_alignments: &'a [String],
    /// Index of the current cell in its row.
    cell_index: usize,
    /// Inside a tight list item: paragraphs render without `<p>`.
    tight_list: bool,
    /// The enclosing list is tight. An item reads it: tight or loose is a property of the whole
    /// list in CommonMark, so the AST says it on the list and not on each item.
    in_tight_list: bool,
}

/// The heading shown for an alert whose `[!TYPE]` line carries no custom title.
/// Same words as the `flatmarkdown` crate's HTML output.
fn default_alert_title(alert_type: &str) -> &'static str {
    match alert_type {
        "tip" => "Tip",
        "important" => "Important",
        "warning" => "Warning",
        "caution" => "Caution",
        _ => "Note",
    }
}

/// Renders an AST node (and its descendants) to a view.
pub fn render_node(node: &Node, options: &RenderOptions) -> AnyView {
    let ctx = Context {
        options,
        in_header_row: false,
        table_alignments: &[],
        cell_index: 0,
        tight_list: false,
        in_tight_list: false,
    };
    render(node, ctx)
}

/// Parses an AST JSON string and renders it. Renders nothing when the JSON does not parse.
#[component]
pub fn FmRoot(
    /// The AST JSON (as written by `flatmarkdown_ast::Node::to_json`).
    ast: String,
    /// Link options; defaults to no prefixes and no classes.
    #[prop(optional)]
    options: RenderOptions,
) -> impl IntoView {
    flatmarkdown_ast::parse(&ast)
        .ok()
        .map(|root| render_node(&root, &options))
}

/// Renders every child with the given context.
fn children(nodes: &[Node], ctx: Context<'_>) -> Vec<AnyView> {
    nodes.iter().map(|child| render(child, ctx)).collect()
}

/// `Some(value)` for a non-empty string, used for attributes Angular binds to `x || null`.
fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

/// Pixel size from an embed prop (`w` / `h` of `![alt](img.png#w=300&h=240)`).
///
/// `None` unless the value is a positive integer: a bare flag (`#w`) arrives as `true`,
/// and a non-numeric value would make the browser ignore the attribute anyway, so the
/// `<img>` keeps its natural size on that axis. The parser writes every value as a string
/// (`SPEC_AST.md`, `embed`), so a number is not expected here.
fn dimension(props: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    let Value::String(s) = props.get(key)? else {
        return None;
    };
    let px = s.trim().parse::<u64>().ok()?;
    (px > 0).then(|| px.to_string())
}

fn render(node: &Node, ctx: Context<'_>) -> AnyView {
    match node {
        // ── Block nodes ──
        Node::Document { children: kids } | Node::Subtext { children: kids } => {
            children(kids, ctx).into_any()
        }

        Node::Paragraph { children: kids } => {
            if ctx.tight_list {
                children(kids, ctx).into_any()
            } else {
                view! { <p>{children(kids, ctx)}</p> }.into_any()
            }
        }

        Node::Heading {
            level,
            children: kids,
            ..
        } => {
            let inner = children(kids, ctx);
            match level {
                1 => view! { <h1>{inner}</h1> }.into_any(),
                2 => view! { <h2>{inner}</h2> }.into_any(),
                3 => view! { <h3>{inner}</h3> }.into_any(),
                4 => view! { <h4>{inner}</h4> }.into_any(),
                5 => view! { <h5>{inner}</h5> }.into_any(),
                6 => view! { <h6>{inner}</h6> }.into_any(),
                _ => ().into_any(),
            }
        }

        Node::CodeBlock { info, literal, .. } => {
            // Set through `.attr` so that an absent class yields no attribute at all
            // (Leptos' typed `class=None` still writes `class=""`).
            let class = non_empty(info).map(|info| format!("language-{info}"));
            let code = view! { <code>{literal.clone()}</code> }.attr("class", class);
            view! { <pre>{code}</pre> }.into_any()
        }

        Node::BlockQuote { children: kids } | Node::MultilineBlockQuote { children: kids } => {
            view! { <blockquote>{children(kids, ctx)}</blockquote> }.into_any()
        }

        Node::List {
            list_type,
            start,
            tight,
            children: kids,
            ..
        } => {
            let ctx = Context {
                in_tight_list: *tight,
                ..ctx
            };
            if list_type == "ordered" {
                let start = (*start != 1).then(|| start.to_string());
                view! { <ol start=start>{children(kids, ctx)}</ol> }.into_any()
            } else {
                view! { <ul>{children(kids, ctx)}</ul> }.into_any()
            }
        }

        Node::Item { children: kids, .. } => {
            // Only a paragraph directly under the item loses its `<p>`. A paragraph deeper in
            // the item, for example in a block quote, keeps it.
            let views: Vec<AnyView> = kids
                .iter()
                .map(|kid| {
                    let ctx = Context {
                        tight_list: ctx.in_tight_list && matches!(kid, Node::Paragraph { .. }),
                        ..ctx
                    };
                    render(kid, ctx)
                })
                .collect();
            view! { <li>{views}</li> }.into_any()
        }

        Node::TaskItem {
            symbol,
            children: kids,
        } => {
            let ctx = Context {
                tight_list: true,
                ..ctx
            };
            let checked = symbol.is_some();
            view! {
                <li>
                    <input type="checkbox" checked=checked disabled=true />
                    {children(kids, ctx)}
                </li>
            }
            .into_any()
        }

        Node::Table {
            alignments,
            children: kids,
            ..
        } => {
            let ctx = Context {
                table_alignments: alignments,
                ..ctx
            };
            view! { <table>{children(kids, ctx)}</table> }.into_any()
        }

        Node::TableRow {
            header,
            children: kids,
        } => {
            let cells: Vec<AnyView> = kids
                .iter()
                .enumerate()
                .map(|(index, cell)| {
                    render(
                        cell,
                        Context {
                            in_header_row: *header,
                            cell_index: index,
                            ..ctx
                        },
                    )
                })
                .collect();
            view! { <tr>{cells}</tr> }.into_any()
        }

        Node::TableCell { children: kids } => {
            let align = ctx
                .table_alignments
                .get(ctx.cell_index)
                .filter(|a| !a.is_empty() && a.as_str() != "none")
                .cloned();
            // `align` is a legacy attribute without a typed Leptos method.
            if ctx.in_header_row {
                view! { <th>{children(kids, ctx)}</th> }
                    .attr("align", align)
                    .into_any()
            } else {
                view! { <td>{children(kids, ctx)}</td> }
                    .attr("align", align)
                    .into_any()
            }
        }

        Node::ThematicBreak => view! { <hr /> }.into_any(),

        Node::HtmlBlock { literal, .. } => {
            view! { <span inner_html=literal.clone()></span> }.into_any()
        }

        Node::FootnoteDefinition {
            name,
            children: kids,
        } => {
            let id = format!("fn-{name}");
            let href = format!("#fnref-{name}");
            let text = format!("[{name}]:");
            // The label goes inside the first paragraph so that the body continues on the
            // same line without any CSS.
            match kids.split_first() {
                Some((Node::Paragraph { children: lead }, rest)) => view! {
                    <div class="footnote" id=id>
                        <p>
                            <a class="footnote-label" href=href>{text}</a>
                            " "
                            {children(lead, ctx)}
                        </p>
                        {children(rest, ctx)}
                    </div>
                }
                .into_any(),
                _ => view! {
                    <div class="footnote" id=id>
                        <a class="footnote-label" href=href>{text}</a>
                        {children(kids, ctx)}
                    </div>
                }
                .into_any(),
            }
        }

        Node::Alert {
            alert_type,
            title,
            children: kids,
        } => {
            let class = format!("alert alert-{alert_type}");
            // The Angular renderer drops the title. We follow the `flatmarkdown` crate's
            // HTML instead, so a reader can tell the alert kind without colour.
            let title = title
                .clone()
                .unwrap_or_else(|| default_alert_title(alert_type).to_string());
            view! {
                <div class=class>
                    <p class="alert-title">{title}</p>
                    {children(kids, ctx)}
                </div>
            }
            .into_any()
        }

        // ── Inline nodes ──
        Node::Text { value } => value.clone().into_any(),
        Node::Softbreak => "\n".into_any(),
        Node::Linebreak => view! { <br /> }.into_any(),

        Node::Emph { children: kids } => view! { <em>{children(kids, ctx)}</em> }.into_any(),
        Node::Strong { children: kids } => {
            view! { <strong>{children(kids, ctx)}</strong> }.into_any()
        }
        Node::Strikethrough { children: kids } => {
            view! { <del>{children(kids, ctx)}</del> }.into_any()
        }
        Node::Underline { children: kids } => view! { <u>{children(kids, ctx)}</u> }.into_any(),
        Node::Highlight { children: kids } => {
            view! { <mark>{children(kids, ctx)}</mark> }.into_any()
        }
        Node::Superscript { children: kids } => {
            view! { <sup>{children(kids, ctx)}</sup> }.into_any()
        }
        Node::Subscript { children: kids } => view! { <sub>{children(kids, ctx)}</sub> }.into_any(),
        Node::SpoileredText { children: kids } => {
            view! { <span class="spoiler">{children(kids, ctx)}</span> }.into_any()
        }

        Node::Code { literal } => view! { <code>{literal.clone()}</code> }.into_any(),

        Node::Link {
            url,
            title,
            children: kids,
        } => {
            let title = non_empty(title);
            view! { <a href=url.clone() title=title>{children(kids, ctx)}</a> }.into_any()
        }

        Node::Embed {
            url, title, props, ..
        } => {
            let alt = extract_text(node);
            let title = non_empty(title);
            let width = dimension(props, "w");
            let height = dimension(props, "h");
            view! { <img src=url.clone() alt=alt title=title width=width height=height /> }
                .into_any()
        }

        Node::Wikilink {
            url,
            children: kids,
        } => {
            let href = format!("{}{url}", ctx.options.wikilink_prefix);
            let class = non_empty(&ctx.options.wikilink_class);
            view! { <a href=href data-wikilink="true">{children(kids, ctx)}</a> }
                .attr("class", class)
                .into_any()
        }

        Node::Hashtag { value } => {
            let href = format!("{}{value}", ctx.options.hashtag_prefix);
            let class = non_empty(&ctx.options.hashtag_class);
            let text = format!("#{value}");
            view! { <a href=href data-hashtag="true">{text}</a> }
                .attr("class", class)
                .into_any()
        }

        Node::FootnoteReference { name, .. } => {
            // The label is shown instead of `ix`: an item body is parsed alone, so `ix`
            // restarts at 1 in every item.
            let id = format!("fnref-{name}");
            let href = format!("#fn-{name}");
            let text = format!("[{name}]");
            view! { <sup class="footnote-ref" id=id><a href=href>{text}</a></sup> }.into_any()
        }

        Node::Shortcode { emoji, .. } => emoji.clone().into_any(),

        // A `math` node is always inline math: `math_dollars` is off, so there is no display math.
        Node::Math { literal } => {
            view! { <code class="math math-inline">{literal.clone()}</code> }.into_any()
        }

        Node::HtmlInline { value } => view! { <span inner_html=value.clone()></span> }.into_any(),

        // Not rendered, as in the Angular template (which has no case for unknown types).
        Node::Unknown { .. } => ().into_any(),
    }
}
