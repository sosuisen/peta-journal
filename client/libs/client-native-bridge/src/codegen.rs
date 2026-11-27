//! Writes the types of the border for a shell that is not written in Rust (ADR 024). The types
//! come from the shape of the Rust types, so a shell stops compiling when a type changes.

use crate::shell::{ShellEffect, ShellEffectFfi};
use crate::Changes;
use client_core::drag::DragMove;
use client_core::effects::{NavigateOp, UiOp};
use client_core::keys::KeyPress;
use client_core::typing::Typing;
use client_core::view::PageView;
use client_core::{Event, RichText, ViewModel};
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
use std::path::Path;

/// The name of the package of the types, in every language.
const PACKAGE: &str = "PetaTypes";

/// The package of the types in Kotlin, where a package is named after a domain.
const KOTLIN_PACKAGE: &str = "com.sosuisha.petajournal.types";

/// Writes the Swift package `PetaTypes` under `out/swift`: the events the shell sends, the view
/// model it draws, the keys, the drags and the typing it hands over, and the requests it gets with their answers.
pub fn write_swift_types(out: &Path) -> Result<(), TypeGenError> {
    let mut registry = TypeRegistry::new();
    ShellEffect::register_types(&mut registry)?;
    registry.register_type::<Event>()?;
    registry.register_type::<KeyPress>()?;
    registry.register_type::<DragMove>()?;
    registry.register_type::<Typing>()?;
    registry.register_type::<ViewModel<RichText>>()?;
    // The generator does not look into the `Box` that keeps this one in the view model
    registry.register_type::<PageView>()?;
    registry.register_type::<Changes>()?;
    let generator = registry.build()?;
    generator.swift(&Config::builder(PACKAGE, out.join("swift")).build())
}

/// The enums with data that a shell takes apart with a `switch`.
const CLOSED_RECORDS: [&str; 4] = ["RichBlock", "RichLink", "RowChange", "Panel"];

/// Writes the same types as the C# project `PetaTypes` under `out/csharp`, for the WinUI shell.
///
/// Not with the generator that Crux 0.20.0 brings (facet_generate 0.19): its C# does not
/// compile for these types. This one is a later version of the same generator, called
/// directly (ADR 028). So the types are listed here, as `ShellEffect::register_types` lists
/// them for Crux.
///
/// The generator writes an enum with data, such as `RichBlock`, as an
/// `abstract record`, which the compiler does not count as closed: a `switch` that leaves out a
/// kind only gets the warning that it gets anyway. So each is made a `closed record` of C# 15
/// here, and the project is made one of .NET 11, whose compiler knows `closed` (ADR 041).
pub fn write_csharp_types(out: &Path) -> Result<(), TypeGenError> {
    let folder = out.join("csharp");
    Installer::new(PACKAGE, &folder)
        .plugin(BincodePlugin)
        .generate(&border_types()?)
        .map_err(failed)?;
    for name in CLOSED_RECORDS {
        rewrite(
            &folder.join(PACKAGE).join(format!("{PACKAGE}.cs")),
            &format!("public abstract record {name} "),
            &format!("public closed record {name} "),
        )?;
    }
    rewrite(
        &folder.join(format!("{PACKAGE}.csproj")),
        "<TargetFramework>net10.0</TargetFramework>",
        "<TargetFramework>net11.0</TargetFramework>",
    )
}

/// Puts `new` in the place of `old` in the file. An `old` that is not there is an error: the
/// generator writes something else now, and what this changes would be lost without a word.
fn rewrite(file: &Path, old: &str, new: &str) -> Result<(), TypeGenError> {
    let text = std::fs::read_to_string(file).map_err(failed)?;
    if !text.contains(old) {
        return Err(failed(format!("{} has no {old:?}", file.display())));
    }
    std::fs::write(file, text.replacen(old, new, 1)).map_err(failed)
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
    let builder = add::<KeyPress>(builder)?;
    let builder = add::<DragMove>(builder)?;
    let builder = add::<Typing>(builder)?;
    let builder = add::<ViewModel<RichText>>(builder)?;
    let builder = add::<PageView>(builder)?;
    let builder = add::<Changes>(builder)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    // C#の型では、RichBlock、RichLink、RowChange、Panelは閉じたrecordで、プロジェクトは.NET 11である。
    #[test]
    fn the_enums_of_csharp_are_closed_records_of_dotnet_11() {
        let out = std::env::temp_dir().join(format!("peta-csharp-types-{}", std::process::id()));
        write_csharp_types(&out).unwrap();

        let types = std::fs::read_to_string(out.join("csharp/PetaTypes/PetaTypes.cs")).unwrap();
        let project = std::fs::read_to_string(out.join("csharp/PetaTypes.csproj")).unwrap();
        std::fs::remove_dir_all(&out).unwrap();
        for name in CLOSED_RECORDS {
            assert!(types.contains(&format!("public closed record {name} : ")));
            assert!(!types.contains(&format!("abstract record {name} ")));
        }
        assert!(project.contains("<TargetFramework>net11.0</TargetFramework>"));
    }

    /// Every file the generator wrote under `folder`, as one text.
    fn written(folder: &Path) -> String {
        let mut text = String::new();
        for entry in std::fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                text += &written(&path);
            } else {
                text += &std::fs::read_to_string(&path).unwrap_or_default();
            }
        }
        text
    }

    // 画面の差分の型（Changes、Panel、RowChange）が、Swift、C#、Kotlinの型として書かれる（ADR 076）。
    #[test]
    fn the_types_of_the_changes_are_written_for_every_shell() {
        let out = std::env::temp_dir().join(format!("peta-change-types-{}", std::process::id()));
        write_swift_types(&out).unwrap();
        write_csharp_types(&out).unwrap();
        write_kotlin_types(&out).unwrap();

        let languages = ["swift", "csharp", "kotlin"].map(|language| written(&out.join(language)));
        std::fs::remove_dir_all(&out).unwrap();
        for types in languages {
            for name in ["Changes", "Panel", "OutlinePanel", "RowChange"] {
                assert!(types.contains(name), "no {name}");
            }
        }
    }
}
