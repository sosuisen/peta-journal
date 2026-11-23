//! Writes the types of the border as TypeScript, for the web shell (ADR 080).

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
            "type ListChange",
            "type Panel",
            "type Event",
            "type KeyPress",
            "type Typing",
            "type DragMove",
            "type UiOp",
            "type UiOutput",
        ] {
            assert!(types.contains(&format!("export {name}")), "no {name}");
        }
        // The body of a row is its HTML, as a string
        assert!(types.contains("public body_model: str,"));
        assert!(runtime, "no reader of the JSON");
    }
}
