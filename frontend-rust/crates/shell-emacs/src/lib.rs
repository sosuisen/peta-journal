//! PetaJournal's Emacs frontend: a shell around the Crux core in `peta-core`. This process holds
//! the core and performs its effects. `lisp/peta-journal.el` only draws what it is told and sends
//! back what the user does, as JSON lines over stdin and stdout.

pub mod protocol;
pub mod route;
pub mod runtime;
pub mod shell;

#[cfg(test)]
mod tests;
