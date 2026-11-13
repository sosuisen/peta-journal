//! Writes the types of the border for a shell that is not written in Rust (ADR 024). The types
//! come from the shape of the Rust types, so a shell stops compiling when a type changes.

use crate::shell::{ShellEffect, ShellEffectFfi};
use crux_core::bridge::Requests;
use crux_core::capability::Operation;
use crux_core::render::RenderOperation;
use crux_core::type_generation::facet::{Config, Export, TypeGenError, TypeRegistry};
use facet::Facet;
use facet_generate::generation::bincode::BincodePlugin;
use facet_generate::generation::csharp::Installer;
use facet_generate::generation::kotlin;
use facet_generate::reflection::RegistryBuilder;
use facet_generate::Registry;
use peta_core::effects::{NavigateOp, UiOp};
use peta_core::view::PageView;
use peta_core::{Event, ViewModel};
use std::path::Path;

/// The name of the package of the types, in every language.
const PACKAGE: &str = "PetaTypes";

/// The package of the types in Kotlin, where a package is named after a domain.
const KOTLIN_PACKAGE: &str = "net.sosuisen.petajournal.types";

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
    generator.swift(&Config::builder(PACKAGE, out.join("swift")).build())
}

/// Writes the same types as the C# project `PetaTypes` under `out/csharp`, for the WinUI shell.
///
/// Not with the generator that Crux 0.20.0 brings (facet_generate 0.19): its C# does not
/// compile for these types. This one is a later version of the same generator, called
/// directly (ADR 028). So the types are listed here, as `ShellEffect::register_types` lists
/// them for Crux.
pub fn write_csharp_types(out: &Path) -> Result<(), TypeGenError> {
    Installer::new(PACKAGE, out.join("csharp"))
        .plugin(BincodePlugin)
        .generate(&border_types()?)
        .map_err(failed)
}

/// Writes the same types as Kotlin under `out/kotlin`, for the Android shell. They are in the
/// package [`KOTLIN_PACKAGE`], with the Kotlin that reads and writes their bytes beside them.
///
/// With the later generator too: the Kotlin of the one that Crux brings does not compile for
/// these types either. A variant named `List` hides the list of Kotlin.
pub fn write_kotlin_types(out: &Path) -> Result<(), TypeGenError> {
    kotlin::Installer::new(KOTLIN_PACKAGE, out.join("kotlin"))
        .plugin(BincodePlugin)
        .generate(&border_types()?)
        .map_err(failed)
}

/// The types of the border, for the generator that is called directly.
fn border_types() -> Result<Registry, TypeGenError> {
    let builder = RegistryBuilder::new();
    let builder = operation::<RenderOperation>(builder)?;
    let builder = operation::<UiOp>(builder)?;
    let builder = operation::<NavigateOp>(builder)?;
    let builder = add::<Requests<ShellEffectFfi>>(builder)?;
    let builder = add::<Event>(builder)?;
    let builder = add::<ViewModel>(builder)?;
    let builder = add::<PageView>(builder)?;
    builder.build().map_err(failed)
}

/// An effect of the shell: what the core asks, and what the shell answers.
fn operation<'a, Op>(builder: RegistryBuilder) -> Result<RegistryBuilder, TypeGenError>
where
    Op: Operation + Facet<'a>,
    Op::Output: Facet<'a>,
{
    add::<Op::Output>(add::<Op>(builder)?)
}

fn add<'a, T: Facet<'a>>(builder: RegistryBuilder) -> Result<RegistryBuilder, TypeGenError> {
    builder.add_type::<T>().map_err(failed)
}

fn failed(error: impl std::fmt::Display) -> TypeGenError {
    TypeGenError::Generation(error.to_string())
}
