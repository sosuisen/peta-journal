//! Where a link to a page of the app leads, and the HTML of a copy with such links (ADR 027).
//!
//! The shells share this: the web shell and the native shells make the same links, so that a
//! link opens the same page whichever shell made it. It is not in the core: the core makes no
//! HTML (ADR 025, ADR 026), and does not know where the app is served.

mod copied_html;
mod links;

pub use copied_html::copied_html;
pub use links::AppLinks;
