//! The kinds of `Event` of `client-core`, read from its source.

/// The names of the variants of `pub enum Event` in `source`, the text of
/// `client-core/src/app.rs`, in their order there. A variant is a line indented by four spaces
/// that starts with a capital letter. `None` when the source has no `pub enum Event`.
pub fn event_variants(source: &str) -> Option<Vec<String>> {
    let mut lines = source.lines();
    lines.find(|line| line.starts_with("pub enum Event {"))?;
    let names = lines
        .take_while(|line| *line != "}")
        .filter_map(variant_name)
        .collect();
    Some(names)
}

fn variant_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("    ")?;
    if !rest.starts_with(|c: char| c.is_ascii_uppercase()) {
        return None;
    }
    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const APP_RS: &str = r#"
/// The route.
pub enum Route {
    Settings,
}

pub enum Event {
    /// The shell is up.
    Start {
        client_id: String,
    },
    // ---- the outline ----
    #[serde(skip)]
    PasteText {
        item_id: String,
    },
    Undo,
    KeyHeld(DeferredKey),
}

impl Event {
    Nothing,
}
"#;

    // `pub enum Event`の種類が、ソースの順に読まれる。
    #[test]
    fn reads_the_variants_of_event_in_their_order() {
        assert_eq!(
            event_variants(APP_RS),
            Some(vec![
                "Start".to_string(),
                "PasteText".to_string(),
                "Undo".to_string(),
                "KeyHeld".to_string(),
            ])
        );
    }

    // `pub enum Event`が無いソースからは、何も読まれない。
    #[test]
    fn reads_nothing_from_a_source_without_event() {
        assert_eq!(event_variants("pub enum Route {\n    Settings,\n}\n"), None);
    }
}
