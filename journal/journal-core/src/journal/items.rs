//! The item operations.
//!
//! An operation that takes more than an ID takes its values as one input. The inputs are the
//! bodies of the REST requests, so a caller that has such a body reads it straight into the
//! input.

use std::collections::BTreeMap;

use hmtid::Hmtid;
use serde::Serialize;

use crate::application::item_service::{BatchOp, MoveRequest, TaskState, TreeMarkdownExport};
use crate::application::paste::PasteMode;
use crate::domain::model::{Item, ItemPosition};
use crate::domain::property_key::PropertyKey;
use crate::dto::{ItemDto, ItemFields, NarrowItem};
use crate::error::{parse_id, AppError};

use super::{ChangedBy, Journal};
pub use journal_api_types::{
    BatchInput, BatchOpInput, CreateItemInput, FlatMarkdownInput, ItemIdsInput, MergeItemsInput,
    MoveItemsInput, PasteInput, PropertyInput, PropertyValue, ShiftLevelsInput, SplitInput,
    StashInput, TaskStateInput, TransclusionInput, TreeMarkdownInput, UpdateItemInput,
};

// ---- inputs ----

/// The operation of a batch, with its IDs and its position read.
fn parse_batch_op(input: BatchOpInput) -> Result<BatchOp, AppError> {
    Ok(match input {
        BatchOpInput::Create {
            id,
            page_id,
            position,
            level,
            body,
            transclude_item_id,
            properties,
        } => BatchOp::Create {
            id: parse_id(&id)?,
            page_id: parse_id(&page_id)?,
            position: parse_position(Some(position))?,
            level,
            body,
            transclude: transclude_item_id.as_deref().map(parse_id).transpose()?,
            properties: properties
                .into_iter()
                .map(|(key, value)| {
                    let values = match value {
                        PropertyValue::Text(text) => vec![text],
                        PropertyValue::List(values) => values,
                    };
                    Ok((PropertyKey::parse(&key)?, values))
                })
                .collect::<Result<_, AppError>>()?,
        },
        BatchOpInput::Update {
            id,
            body,
            level,
            collapsed,
            task,
        } => BatchOp::Update {
            id: parse_id(&id)?,
            body,
            level,
            collapsed,
            task: task.map(|task| TaskState {
                task: task.task,
                begin: task.begin,
                end: task.end,
                plan: task.plan,
            }),
        },
        BatchOpInput::Move {
            ids,
            page_id,
            position,
        } => BatchOp::Move {
            ids: parse_ids(Some(&ids), "ids")?,
            page_id: parse_id(&page_id)?,
            position: parse_position(Some(position))?,
        },
        BatchOpInput::Delete { id } => BatchOp::Delete { id: parse_id(&id)? },
    })
}

// ---- answers ----

#[derive(Debug, Clone, Serialize)]
pub struct ItemAnswer<I = ItemDto> {
    pub item: I,
}

impl ItemAnswer {
    /// The same answer with only these fields of the item.
    pub fn with_fields(self, fields: &ItemFields) -> ItemAnswer<NarrowItem> {
        ItemAnswer {
            item: fields.of(&self.item),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemsAnswer<I = ItemDto> {
    pub items: Vec<I>,
}

impl ItemsAnswer {
    /// The same answer with only these fields of each item.
    pub fn with_fields(self, fields: &ItemFields) -> ItemsAnswer<NarrowItem> {
        ItemsAnswer {
            items: fields.of_each(&self.items),
        }
    }
}

/// Where the cursor goes after a paste. The server and the clients share the type (ADR 082).
pub use journal_api_types::PasteCursor as CursorDto;

/// What a paste came to. The server and the clients share the type (ADR 082).
pub type PasteAnswer = journal_api_types::Pasted<ItemDto>;

/// What a split came to. The server and the clients share the type (ADR 082).
pub type SplitAnswer = journal_api_types::Split<ItemDto>;

/// What a stash came to. The server and the clients share the type (ADR 082).
pub type StashAnswer = journal_api_types::Stashed<ItemDto>;

/// The items a batch wrote, and the empty items made on the pages it left without an item.
/// The server and the clients share the type (ADR 082).
pub type BatchAnswer = journal_api_types::Batched<ItemDto>;

/// The empty items made on the pages a deletion left without an item
#[derive(Debug, Clone, Serialize)]
pub struct DeletedItemsAnswer {
    pub created: Vec<ItemDto>,
}

/// The ids, among those asked about, of the items some other item transcludes
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscludedItemsAnswer {
    pub item_ids: Vec<String>,
}

/// A tree as Markdown. The server and the clients share the type (ADR 082).
pub use journal_api_types::TreeExport as TreeMarkdownAnswer;

impl From<TreeMarkdownExport> for TreeMarkdownAnswer {
    fn from(export: TreeMarkdownExport) -> TreeMarkdownAnswer {
        TreeMarkdownAnswer {
            tree_markdown: export.tree_markdown,
            flat_markdown: export.flat_markdown,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatMarkdownAnswer {
    pub flat_markdown: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedFlatItem {
    pub level: i32,
    pub id: Option<String>,
    pub properties: BTreeMap<String, String>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParsedItemsAnswer {
    pub items: Vec<ParsedFlatItem>,
}

// ---- reading the values ----

fn required<'a>(value: Option<&'a str>, name: &str) -> Result<&'a str, AppError> {
    value.ok_or_else(|| AppError::bad_request(format!("{name} is required")))
}

fn parse_ids(ids: Option<&[String]>, name: &str) -> Result<Vec<Hmtid>, AppError> {
    let ids = ids.ok_or_else(|| AppError::bad_request(format!("{name} is required")))?;
    ids.iter().map(|s| parse_id(s)).collect()
}

pub(super) fn parse_position(
    position: Option<journal_api_types::ItemPosition>,
) -> Result<ItemPosition, AppError> {
    let position =
        position.ok_or_else(|| AppError::bad_request("position must not be null or empty"))?;
    ItemPosition::try_from(position).map_err(AppError::bad_request)
}

fn parse_paste_mode(mode: Option<&str>) -> Result<PasteMode, AppError> {
    match mode {
        None | Some("auto") => Ok(PasteMode::Auto),
        Some("flat") => Ok(PasteMode::Flat),
        Some("raw") => Ok(PasteMode::Raw),
        Some(other) => Err(AppError::bad_request(format!(
            "mode must be auto, flat or raw, but was {other}"
        ))),
    }
}

impl Journal {
    // ---- reading ----

    pub fn get_item(&self, id: &str) -> Result<ItemAnswer, AppError> {
        let item_id = parse_id(id)?;
        self.item_answer(&self.services.item_service.get_item_by_id(&item_id)?)
    }

    /// The items that have all of the types. `without_property` leaves out the items that
    /// have a value for that property.
    pub fn items_by_types(
        &self,
        types: &[String],
        without_property: Option<&str>,
    ) -> Result<ItemsAnswer, AppError> {
        let items = self
            .services
            .item_service
            .get_items_by_types(types, without_property)?;
        self.items_answer(&items)
    }

    /// Which of the items are transcluded by other items, as a client asks before it deletes
    /// them.
    pub fn transcluded_items(
        &self,
        input: ItemIdsInput,
    ) -> Result<TranscludedItemsAnswer, AppError> {
        let ids = parse_ids(input.item_ids.as_deref(), "itemIds")?;
        let mut item_ids = Vec::new();
        for id in ids {
            if !self.services.items.find_transcluders(&id)?.is_empty() {
                item_ids.push(id.to_string());
            }
        }
        Ok(TranscludedItemsAnswer { item_ids })
    }

    // ---- changing ----

    pub fn create_item(
        &self,
        input: CreateItemInput,
        by: &ChangedBy,
    ) -> Result<ItemAnswer, AppError> {
        let page_id = parse_id(required(input.page_id.as_deref(), "pageId")?)?;
        let id = match input.id.as_deref() {
            Some(s) if !s.is_empty() => Some(parse_id(s)?),
            _ => None,
        };
        let position = parse_position(input.position.clone())?;
        let item = self.services.item_service.create_item(
            &page_id,
            &position,
            input.level,
            input.body.as_deref().unwrap_or_default(),
            id,
        )?;
        self.tell([&page_id], by);
        self.item_answer(&item)
    }

    pub fn update_item(
        &self,
        id: &str,
        input: UpdateItemInput,
        by: &ChangedBy,
    ) -> Result<ItemAnswer, AppError> {
        let item_id = parse_id(id)?;
        let item = self.services.item_service.update_item(
            &item_id,
            input.body.as_deref(),
            input.level,
            input.collapsed,
        )?;
        self.tell([&item.page_id], by);
        self.tell_transcluders(&item_id, &item.page_id, by)?;
        self.item_answer(&item)
    }

    /// Puts a text into an item at the cursor. A text of several items makes new items.
    pub fn paste(
        &self,
        id: &str,
        input: PasteInput,
        by: &ChangedBy,
    ) -> Result<PasteAnswer, AppError> {
        let item_id = parse_id(id)?;
        let mode = parse_paste_mode(input.mode.as_deref())?;
        let pasted = self.services.item_service.paste(
            &item_id,
            input.body.as_deref().unwrap_or_default(),
            input.cursor,
            input.text.as_deref().unwrap_or_default(),
            mode,
        )?;
        // The first item is the one written to: the source item, when a transclusion was
        // pasted into
        let written = &pasted.items[0];
        self.tell([&written.page_id], by);
        self.tell_transcluders(&written.id, &written.page_id, by)?;
        Ok(PasteAnswer {
            items: self.services.dtos.to_dtos(&pasted.items)?,
            cursor: CursorDto {
                item_id: pasted.cursor_item_id.to_string(),
                position: pasted.cursor,
            },
        })
    }

    /// Splits an item in two at the cursor.
    pub fn split(
        &self,
        id: &str,
        input: SplitInput,
        by: &ChangedBy,
    ) -> Result<SplitAnswer, AppError> {
        let item_id = parse_id(id)?;
        let root_id = input.root_id.as_deref().map(parse_id).transpose()?;
        let split = self.services.item_service.split(
            &item_id,
            root_id.as_ref(),
            input.body.as_deref().unwrap_or_default(),
            input.cursor,
        )?;
        self.tell(&split.page_ids, by);
        Ok(SplitAnswer {
            item: self.services.dtos.to_dto(&split.item)?,
            created: self.services.dtos.to_dto(&split.created)?,
        })
    }

    /// Sends an item with its descendants under the `# Stash` heading of its page (ADR 011).
    pub fn stash(
        &self,
        id: &str,
        input: StashInput,
        by: &ChangedBy,
    ) -> Result<StashAnswer, AppError> {
        let item_id = parse_id(id)?;
        let stashed = self
            .services
            .item_service
            .stash(&item_id, input.body.as_deref())?;
        self.tell(&stashed.page_ids, by);
        Ok(StashAnswer {
            heading: self.services.dtos.to_dto(&stashed.heading)?,
            created: stashed.created,
            items: self.services.dtos.to_dtos(&stashed.items)?,
        })
    }

    /// Applies several changes of items in one transaction.
    pub fn apply_batch(&self, input: BatchInput, by: &ChangedBy) -> Result<BatchAnswer, AppError> {
        let ops = input
            .ops
            .into_iter()
            .map(parse_batch_op)
            .collect::<Result<Vec<_>, AppError>>()?;
        let applied = self.services.item_service.apply_batch(&ops)?;
        self.tell(&applied.page_ids, by);
        Ok(BatchAnswer {
            items: self.services.dtos.to_dtos(&applied.items)?,
            created: self.services.dtos.to_dtos(&applied.created)?,
        })
    }

    pub fn delete_items(
        &self,
        input: ItemIdsInput,
        by: &ChangedBy,
    ) -> Result<DeletedItemsAnswer, AppError> {
        let item_ids = parse_ids(input.item_ids.as_deref(), "itemIds")?;
        // The items are gone after the deletion: the pages that show them are found first
        let pages = self.services.item_service.pages_showing(&item_ids)?;
        let created = self.services.item_service.delete_items(&item_ids)?;
        self.tell(&pages, by);
        Ok(DeletedItemsAnswer {
            created: self.services.dtos.to_dtos(&created)?,
        })
    }

    pub fn delete_item(&self, id: &str, by: &ChangedBy) -> Result<(), AppError> {
        let item_id = parse_id(id)?;
        let pages = self
            .services
            .item_service
            .pages_showing(std::slice::from_ref(&item_id))?;
        self.services.item_service.delete_item(&item_id)?;
        self.tell(&pages, by);
        Ok(())
    }

    /// Joins an item to the end of another. The item goes away, and the other takes its text.
    pub fn merge_items(
        &self,
        input: MergeItemsInput,
        by: &ChangedBy,
    ) -> Result<ItemAnswer, AppError> {
        let item_id = parse_id(input.item_id.as_deref().unwrap_or(""))?;
        let into_id = parse_id(input.into_id.as_deref().unwrap_or(""))?;
        // The merged item goes away: its transcluders are found first so their pages can be told
        let transcluders = self.services.items.find_transcluders(&item_id)?;
        let item_page_id = self.services.item_service.get_item_page_id(&item_id)?;
        let into = self.services.item_service.merge_item(
            &item_id,
            &into_id,
            input.body.as_deref(),
            input.into_body.as_deref(),
        )?;
        self.tell([&into.page_id], by);
        self.tell(&item_page_id, by);
        self.tell_transcluders(&into_id, &into.page_id, by)?;
        self.tell(transcluders.iter().map(|t| &t.page_id), by);
        self.item_answer(&into)
    }

    /// Makes the item show another item.
    pub fn set_transclusion(
        &self,
        id: &str,
        input: TransclusionInput,
        by: &ChangedBy,
    ) -> Result<ItemAnswer, AppError> {
        let item_id = parse_id(id)?;
        let source_id = parse_id(required(input.source_item_id.as_deref(), "sourceItemId")?)?;
        let item = self
            .services
            .item_service
            .set_transclusion(&item_id, &source_id)?;
        self.tell([&item.page_id], by);
        self.item_answer(&item)
    }

    pub fn remove_transclusion(&self, id: &str, by: &ChangedBy) -> Result<ItemAnswer, AppError> {
        let item_id = parse_id(id)?;
        let item = self.services.item_service.remove_transclusion(&item_id)?;
        self.tell([&item.page_id], by);
        self.item_answer(&item)
    }

    /// Takes the item to its next task state.
    pub fn cycle_task(&self, id: &str, by: &ChangedBy) -> Result<ItemAnswer, AppError> {
        let item_id = parse_id(id)?;
        let item = self.services.item_service.cycle_task(&item_id)?;
        self.tell([&item.page_id], by);
        self.item_answer(&item)
    }

    pub fn move_items(
        &self,
        input: MoveItemsInput,
        by: &ChangedBy,
    ) -> Result<ItemsAnswer, AppError> {
        let position = parse_position(input.position.clone())?;
        let ids = parse_ids(input.item_ids.as_deref(), "itemIds")?;
        let target = parse_id(required(input.target_page_id.as_deref(), "targetPageId")?)?;
        let moved = self.services.item_service.move_edited_items(&MoveRequest {
            item_ids: &ids,
            target_page_id: &target,
            position: &position,
            level_delta: input.level_delta.unwrap_or(0),
            body: input.body.as_deref(),
        })?;
        self.tell(&moved.page_ids, by);
        self.items_answer(&moved.items)
    }

    /// Indents or outdents a group of items in one request. The change is relative, so
    /// requests that arrive out of order still add up correctly.
    pub fn shift_levels(
        &self,
        input: ShiftLevelsInput,
        by: &ChangedBy,
    ) -> Result<ItemsAnswer, AppError> {
        let ids = parse_ids(input.item_ids.as_deref(), "itemIds")?;
        let items = self.services.item_service.shift_levels(&ids, input.delta)?;
        let mut seen: Vec<&Hmtid> = Vec::new();
        for item in &items {
            let page_id = &item.page_id;
            if !seen.contains(&page_id) {
                self.tell([page_id], by);
                seen.push(page_id);
            }
        }
        for item in &items {
            self.tell_transcluders(&item.id, &item.page_id, by)?;
        }
        self.items_answer(&items)
    }

    pub fn set_item_property(
        &self,
        id: &str,
        key: &str,
        input: PropertyInput,
        by: &ChangedBy,
    ) -> Result<ItemAnswer, AppError> {
        let item_id = parse_id(id)?;
        let new_key = self.services.properties.find_definition(key)?.is_none();
        let item = self.services.item_service.set_property(
            &item_id,
            key,
            input.value_type.as_deref().unwrap_or("text"),
            input.value.as_deref().unwrap_or(""),
        )?;
        if new_key {
            self.definitions_changed()?;
        }
        self.tell([&item.page_id], by);
        // The answer is built after the upsert, so it carries the new property
        self.item_answer(&item)
    }

    pub fn delete_item_property(
        &self,
        id: &str,
        key: &str,
        by: &ChangedBy,
    ) -> Result<ItemAnswer, AppError> {
        let item_id = parse_id(id)?;
        let item = self.services.item_service.delete_property(&item_id, key)?;
        self.tell([&item.page_id], by);
        // The answer is built after the delete, so the property is already gone from it
        self.item_answer(&item)
    }

    // ---- markdown ----

    pub fn export_tree_markdown(
        &self,
        input: ItemIdsInput,
    ) -> Result<TreeMarkdownAnswer, AppError> {
        let ids = parse_ids(input.item_ids.as_deref(), "itemIds")?;
        Ok(self
            .services
            .item_service
            .export_tree_markdown(&ids)?
            .into())
    }

    pub fn export_page_markdown(
        &self,
        page_id: Option<&str>,
    ) -> Result<TreeMarkdownAnswer, AppError> {
        let page_id = parse_id(required(page_id, "pageId")?)?;
        Ok(self
            .services
            .item_service
            .export_page_markdown(&page_id)?
            .into())
    }

    /// The item and all its descendants.
    pub fn export_subtree_markdown(
        &self,
        item_id: Option<&str>,
    ) -> Result<TreeMarkdownAnswer, AppError> {
        let item_id = parse_id(required(item_id, "itemId")?)?;
        Ok(self
            .services
            .item_service
            .export_subtree_markdown(&item_id)?
            .into())
    }

    /// Turns tree markdown into Flat Markdown. Nothing is stored.
    pub fn import_tree_markdown(&self, input: TreeMarkdownInput) -> FlatMarkdownAnswer {
        FlatMarkdownAnswer {
            flat_markdown: self
                .services
                .item_service
                .import_tree_markdown(input.tree_markdown.as_deref().unwrap_or("")),
        }
    }

    /// Splits Flat Markdown into items by the rules of the spec. Nothing is stored.
    /// Bodies keep their GFM task prefix, so they can be passed to `create_item` as they are.
    pub fn parse_flat_markdown(
        &self,
        input: FlatMarkdownInput,
    ) -> Result<ParsedItemsAnswer, AppError> {
        let page = self
            .services
            .splitter
            .split(input.markdown.as_deref().unwrap_or(""))?;
        Ok(ParsedItemsAnswer {
            items: page
                .items
                .into_iter()
                .map(|i| ParsedFlatItem {
                    level: i.level,
                    id: i.id.clone(),
                    properties: i.properties_map(),
                    body: i.body,
                })
                .collect(),
        })
    }

    // ---- shared ----

    /// An item as the API answers with it, with its properties.
    fn item_answer(&self, item: &Item) -> Result<ItemAnswer, AppError> {
        Ok(ItemAnswer {
            item: self.services.dtos.to_dto(item)?,
        })
    }

    fn items_answer(&self, items: &[Item]) -> Result<ItemsAnswer, AppError> {
        Ok(ItemsAnswer {
            items: self.services.dtos.to_dtos(items)?,
        })
    }

    /// Tells the pages that show the item through a transclusion, but for `except_page_id`,
    /// which was told already.
    fn tell_transcluders(
        &self,
        source_item_id: &Hmtid,
        except_page_id: &Hmtid,
        by: &ChangedBy,
    ) -> Result<(), AppError> {
        let transcluders = self.services.items.find_transcluders(source_item_id)?;
        self.tell(
            transcluders
                .iter()
                .map(|t| &t.page_id)
                .filter(|page_id| *page_id != except_page_id),
            by,
        );
        Ok(())
    }
}
