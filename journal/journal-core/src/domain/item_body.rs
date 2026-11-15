//! The body text of an item as the user wrote it, split into the stored text and the
//! facts the server derives from it.

pub const TASK_TYPE: &str = "task";
pub const HEADING_TYPE: &str = "heading";
pub const CODE_TYPE: &str = "code";
pub const BLOCKQUOTE_TYPE: &str = "blockquote";

/// Why a body with a block that is not closed is not stored.
pub const UNCLOSED_BLOCK: &str =
    "Markdownのブロック要素は、開始記号と閉じる記号が揃わない限り、保存できません。";

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
    /// Splits a raw body into its text and its task flag.
    pub fn parse(raw: &str) -> ItemBody {
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

    /// Splits a raw body that comes in to be stored, as `parse` does. A body that ends inside
    /// a block that is not closed is refused (ADR 050): in the file of the page, the block
    /// would take the items after it.
    pub fn accept(raw: &str) -> Result<ItemBody, String> {
        if flatmarkdown::has_unclosed_block(raw) {
            return Err(UNCLOSED_BLOCK.to_string());
        }
        Ok(ItemBody::parse(raw))
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

    // タスクの接頭辞が本文から取り除かれる。
    #[test]
    fn strips_task_prefix() {
        let b = ItemBody::parse("- [ ] buy milk");
        assert_eq!(b.text, "buy milk");
        assert!(b.task);
        let b = ItemBody::parse("- [x] done");
        assert_eq!(b.text, "done");
        assert!(b.task);
        let b = ItemBody::parse("plain");
        assert!(!b.task);
    }

    // 本文の型が検出される。
    #[test]
    fn detects_types() {
        assert_eq!(ItemBody::parse("# Title").text_types(), vec!["heading"]);
        assert_eq!(ItemBody::parse("```js").text_types(), vec!["code"]);
        assert_eq!(ItemBody::parse("> quote").text_types(), vec!["blockquote"]);
        assert!(ItemBody::parse("text").text_types().is_empty());
    }
}
