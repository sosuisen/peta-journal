//! The body text of an item as the user wrote it, split into the stored text and the
//! facts the server derives from it.

pub const TASK_TYPE: &str = "task";
pub const HEADING_TYPE: &str = "heading";
pub const CODE_TYPE: &str = "code";
pub const BLOCKQUOTE_TYPE: &str = "blockquote";

const OPEN_TASK_PREFIX: &str = "- [ ] ";
const DONE_TASK_PREFIX: &str = "- [x] ";

/// A body without its GFM task prefix, and whether it had one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemBody {
    /// The body without the task prefix.
    pub text: String,
    /// True when the body started with a task prefix.
    pub task: bool,
}

impl ItemBody {
    /// Splits a raw body into its text and its task flag. `None` counts as empty.
    pub fn parse(raw: Option<&str>) -> ItemBody {
        let Some(raw) = raw else {
            return ItemBody {
                text: String::new(),
                task: false,
            };
        };
        if raw.starts_with(OPEN_TASK_PREFIX) || raw.starts_with(DONE_TASK_PREFIX) {
            return ItemBody {
                text: raw[OPEN_TASK_PREFIX.len()..].to_string(),
                task: true,
            };
        }
        ItemBody {
            text: raw.to_string(),
            task: false,
        }
    }

    /// The types read from the text: `heading`, `code` and `blockquote`, in that order.
    /// The task type is not included; see `task`.
    pub fn text_types(&self) -> Vec<String> {
        let trimmed = self.text.trim();
        let mut types = Vec::new();
        if trimmed.starts_with('#') {
            types.push(HEADING_TYPE.to_string());
        }
        if trimmed.starts_with("```") {
            types.push(CODE_TYPE.to_string());
        }
        if trimmed.starts_with('>') {
            types.push(BLOCKQUOTE_TYPE.to_string());
        }
        types
    }

    /// All types of the body: the text types plus `task` when it had the prefix.
    pub fn all_types(&self) -> Vec<String> {
        let mut types = self.text_types();
        if self.task {
            types.push(TASK_TYPE.to_string());
        }
        types
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_task_prefix() {
        let b = ItemBody::parse(Some("- [ ] buy milk"));
        assert_eq!(b.text, "buy milk");
        assert!(b.task);
        let b = ItemBody::parse(Some("- [x] done"));
        assert_eq!(b.text, "done");
        assert!(b.task);
        let b = ItemBody::parse(Some("plain"));
        assert!(!b.task);
        assert_eq!(ItemBody::parse(None).text, "");
    }

    #[test]
    fn detects_types() {
        assert_eq!(ItemBody::parse(Some("# Title")).text_types(), vec!["heading"]);
        assert_eq!(ItemBody::parse(Some("```js")).text_types(), vec!["code"]);
        assert_eq!(ItemBody::parse(Some("> quote")).text_types(), vec!["blockquote"]);
        assert!(ItemBody::parse(Some("text")).text_types().is_empty());
    }
}
