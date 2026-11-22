//! PetaJournal's Emacs frontend: a shell around the Crux core in `client-core`. This process holds
//! the core and performs its effects. `lisp/peta-journal.el` only draws what it is told and sends
//! back what the user does, as JSON lines over stdin and stdout.

pub mod emacs_text;
pub mod protocol;
pub mod runtime;
pub mod shell;

#[cfg(test)]
mod tests;

/// Written only by a test, which keeps the committed file as the types make it.
#[cfg(test)]
mod lisp_protocol;
