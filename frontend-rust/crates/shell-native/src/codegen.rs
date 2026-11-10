//! Writes the types of the border for a shell that is not written in Rust (ADR 024). The types
//! come from the shape of the Rust types, so a shell stops compiling when a type changes.

use crate::shell::ShellEffect;
use crux_core::type_generation::facet::{Config, Export, TypeGenError, TypeRegistry};
use peta_core::view::PageView;
use peta_core::{Event, ViewModel};
use std::path::Path;

/// Writes the Swift package `PetaTypes` under `out/swift`: the events the shell sends, the view
/// model it draws, and the requests it gets with their answers.
pub fn write_swift_types(out: &Path) -> Result<(), TypeGenError> {
    let mut registry = TypeRegistry::new();
    ShellEffect::register_types(&mut registry)?;
    registry.register_type::<Event>()?;
    registry.register_type::<ViewModel>()?;
    // The generator does not look into the `Box` that keeps this one in the view model
    registry.register_type::<PageView>()?;
    let generator = registry.build()?;
    generator.swift(&Config::builder("PetaTypes", out.join("swift")).build())
}
