//! Writes the types of the border as TypeScript, for the web shell (ADR 080).

use crate::body::HtmlBody;
use crate::replay::Replay;
use client_core::changes::Changes;
use client_core::drag::DragMove;
use client_core::effects::{UiOp, UiOutput};
use client_core::keys::KeyPress;
use client_core::typing::Typing;
use client_core::Event;
use facet::Facet;
use facet_generate::generation::json::JsonPlugin;
use facet_generate::generation::typescript::Installer;
use facet_generate::reflection::RegistryBuilder;
use std::path::Path;

/// The name of the package of the types.
const PACKAGE: &str = "peta-types";

/// Writes the package of the types under `out`: the events, keys, typing and drags the shell
/// hands over, the operations of the screen it does with their answers, and the changes it
/// draws. Each type reads and writes the JSON of facet, as wasm does.
pub fn write_typescript_types(out: &Path) -> Result<(), String> {
    let builder = RegistryBuilder::new();
    let builder = add::<Event>(builder)?;
    let builder = add::<KeyPress>(builder)?;
    let builder = add::<Typing>(builder)?;
    let builder = add::<DragMove>(builder)?;
    let builder = add::<UiOp>(builder)?;
    let builder = add::<UiOutput>(builder)?;
    let builder = add::<Changes<HtmlBody>>(builder)?;
    let builder = add::<Replay>(builder)?;
    let registry = builder.build().map_err(|e| e.to_string())?;
    Installer::new(PACKAGE, out)
        .plugin(JsonPlugin)
        .generate(&registry)
        .map_err(|e| e.to_string())
}

fn add<'a, T: Facet<'a>>(builder: RegistryBuilder) -> Result<RegistryBuilder, String> {
    builder.add_type::<T>().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 境界の型が、TypeScriptの型として書かれる。
    #[test]
    fn the_types_of_the_border_are_written_as_typescript() {
        let out = std::env::temp_dir().join(format!("peta-web-types-{}", std::process::id()));

        write_typescript_types(&out).unwrap();

        let types = std::fs::read_to_string(out.join("peta-types.ts")).unwrap();
        let runtime = out.join("serde/json.ts").exists();
        std::fs::remove_dir_all(&out).unwrap();
        for name in [
            "class Changes",
            "type RowChange",
            "type Panel",
            "type Event",
            "type KeyPress",
            "type Typing",
            "type DragMove",
            "type UiOp",
            "type UiOutput",
            "type Replay",
        ] {
            assert!(types.contains(&format!("export {name}")), "no {name}");
        }
        // The body of a row is its HTML, as a string
        assert!(types.contains("public body_model: str,"));
        assert!(runtime, "no reader of the JSON");
    }
}
