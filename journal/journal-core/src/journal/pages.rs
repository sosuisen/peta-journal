//! The page operations.

use std::collections::BTreeMap;

use journal_api_types::PropertyValue;
use serde::Serialize;

use crate::domain::journal_name::JournalName;
use crate::domain::model::Page;
use crate::dto::{
    backlink_dto, page_dto_with, BacklinkEntryDto, ItemDto, ItemFields, NarrowItem, PageDto,
};
use crate::error::{parse_id, AppError};

use super::{ChangedBy, Journal};
pub use journal_api_types::FindPageQuery;

/// How many recent pages are listed when the caller names no number.
const DEFAULT_RECENT_PAGES: i64 = 10;

const PAGE_TARGET: &str = "page";

/// A page with its items, and the trees of the items that its transclusions show.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageWithItems<I = ItemDto> {
    pub page: PageDto,
    pub items: Vec<I>,
    pub transcluded_trees: BTreeMap<String, Vec<I>>,
}

impl PageWithItems {
    /// The same answer with only these fields of each item.
    pub fn with_fields(self, fields: &ItemFields) -> PageWithItems<NarrowItem> {
        PageWithItems {
            items: fields.of_each(&self.items),
            transcluded_trees: self
                .transcluded_trees
                .iter()
                .map(|(source, tree)| (source.clone(), fields.of_each(tree)))
                .collect(),
            page: self.page,
        }
    }
}

/// A document page that was found or made. The server and the clients share the type (ADR 082).
pub type DocAnswer = journal_api_types::DocPage<PageDto, ItemDto>;

/// A page looked for without making it, or none when it is not there.
#[derive(Debug, Clone, Serialize)]
pub struct FoundPageAnswer {
    pub page: Option<PageDto>,
}

/// A page after a change.
#[derive(Debug, Clone, Serialize)]
pub struct PageAnswer {
    pub page: PageDto,
}

#[derive(Debug, Clone, Serialize)]
pub struct PagesAnswer {
    pub pages: Vec<PageDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacklinksAnswer {
    pub entries: Vec<BacklinkEntryDto>,
}

impl Journal {
    pub fn get_page(&self, id: &str) -> Result<PageWithItems, AppError> {
        let page_id = parse_id(id)?;
        let page = self
            .services
            .page_service
            .get_page(&page_id)?
            .ok_or_else(|| AppError::not_found(format!("Page not found: {id}")))?;
        let items = self.services.page_service.get_page_items(&page_id)?;
        Ok(PageWithItems {
            page: self.services.dtos.page_to_dto(&page)?,
            items: self.services.dtos.to_dtos(&items)?,
            transcluded_trees: self.services.dtos.transcluded_trees(&items)?,
        })
    }

    /// Finds the document page of the title in the folder, or makes it. The extensions answer
    /// a page that was made, and their items come with it.
    pub fn get_or_create_doc(
        &self,
        title: Option<&str>,
        dir_path: Option<&str>,
    ) -> Result<DocAnswer, AppError> {
        let mut result = self
            .services
            .page_service
            .get_or_create_doc(title, dir_path)?;
        if result.created {
            result.items = self.services.extensions.page_created(&result.page)?;
        }
        Ok(DocAnswer {
            page: self.services.dtos.page_to_dto(&result.page)?,
            items: self.services.dtos.to_dtos(&result.items)?,
            created: result.created,
        })
    }

    /// Looks for a journal or a document page, and makes none: going back to a page that
    /// sync deleted does not bring it back.
    pub fn find_page(&self, query: FindPageQuery) -> Result<FoundPageAnswer, AppError> {
        let page = match &query.journal {
            Some(name) => self
                .services
                .journal_service
                .find_journal(&name.parse::<JournalName>()?)?,
            None => self.services.page_service.find_doc(
                query.title.as_deref().unwrap_or_default(),
                query.dir_path.as_deref().unwrap_or_default(),
            )?,
        };
        Ok(FoundPageAnswer {
            page: page
                .map(|page| self.services.dtos.page_to_dto(&page))
                .transpose()?,
        })
    }

    /// Gives the page another title or folder. With `overwrite`, another page that has the
    /// new name is deleted, and the update is not refused.
    pub fn update_page(
        &self,
        id: &str,
        title: Option<&str>,
        dir_path: Option<&str>,
        overwrite: bool,
        by: &ChangedBy,
    ) -> Result<PageAnswer, AppError> {
        let page_id = parse_id(id)?;
        let (page, changed) = self
            .services
            .item_service
            .update_page(&page_id, title, dir_path, overwrite)?;
        // `changed`: the pages whose tags the rename of a tag page rewrote
        self.tell(std::iter::once(&page_id).chain(&changed), by);
        self.page_answer(&page)
    }

    /// Makes the index page of a folder a plain page of the folder above.
    pub fn demote_page(
        &self,
        id: &str,
        overwrite: bool,
        by: &ChangedBy,
    ) -> Result<PageAnswer, AppError> {
        let page_id = parse_id(id)?;
        let (page, changed) = self
            .services
            .item_service
            .demote_index_page(&page_id, overwrite)?;
        self.tell(std::iter::once(&page_id).chain(&changed), by);
        self.page_answer(&page)
    }

    /// Sets a property of the page. The extensions answer the tags that were added.
    pub fn set_page_property(
        &self,
        id: &str,
        key: &str,
        value_type: Option<&str>,
        value: Option<&str>,
        by: &ChangedBy,
    ) -> Result<PageAnswer, AppError> {
        let page_id = parse_id(id)?;
        let tags_before = self.services.properties.find_tags(PAGE_TARGET, id)?;
        let new_key = self.services.properties.find_definition(key)?.is_none();
        let page = self.services.page_service.set_property(
            &page_id,
            key,
            value_type.unwrap_or("text"),
            value.unwrap_or(""),
        )?;
        if new_key {
            self.definitions_changed()?;
        }
        self.tell([&page_id], by);
        if key == "tags" {
            let tags_after = self.services.properties.find_tags(PAGE_TARGET, id)?;
            self.services
                .extensions
                .tags_changed(&page, &tags_before, &tags_after);
        }
        self.page_answer(&page)
    }

    pub fn delete_page_property(
        &self,
        id: &str,
        key: &str,
        by: &ChangedBy,
    ) -> Result<PageAnswer, AppError> {
        let page_id = parse_id(id)?;
        let page = self.services.page_service.delete_property(&page_id, key)?;
        self.tell([&page_id], by);
        self.page_answer(&page)
    }

    /// Deletes the page and all its items.
    pub fn delete_page(&self, id: &str, by: &ChangedBy) -> Result<(), AppError> {
        let page_id = parse_id(id)?;
        self.services.page_service.delete_page(&page_id)?;
        self.tell([&page_id], by);
        Ok(())
    }

    /// The pages that changed last, newest first. An empty `page_type` means every type. A
    /// number of zero or less means the same as no number.
    pub fn recent_pages(
        &self,
        max_results: Option<i64>,
        page_type: Option<&str>,
    ) -> Result<PagesAnswer, AppError> {
        let page_type = page_type.filter(|t| !t.is_empty());
        let pages = self
            .services
            .page_service
            .get_recent_pages(max_results.unwrap_or(DEFAULT_RECENT_PAGES), page_type)?;
        self.pages_with_scalars(&pages)
    }

    /// The pages and items that link to the page.
    pub fn backlinks(&self, id: &str) -> Result<BacklinksAnswer, AppError> {
        let page_id = parse_id(id)?;
        let entries = self.services.page_service.get_backlinks(&page_id)?;
        Ok(BacklinksAnswer {
            entries: entries.iter().map(backlink_dto).collect(),
        })
    }

    /// The pages directly under the page.
    pub fn child_pages(&self, id: &str) -> Result<PagesAnswer, AppError> {
        let page_id = parse_id(id)?;
        let pages = self.services.pages.find_by_parent_id(&page_id)?;
        let mut answer = self.pages_with_scalars(&pages)?;
        // The atlas draws an icon from the tags of each card
        let ids: Vec<String> = pages.iter().map(|p| p.id.to_string()).collect();
        let lists = self
            .services
            .properties
            .find_list_by_targets(PAGE_TARGET, &ids)?;
        for dto in &mut answer.pages {
            if let Some(tags) = lists.get(&dto.page.id).and_then(|l| l.get("tags")) {
                dto.page
                    .properties
                    .insert("tags".into(), PropertyValue::List(tags.clone()));
            }
        }
        Ok(answer)
    }

    fn page_answer(&self, page: &Page) -> Result<PageAnswer, AppError> {
        Ok(PageAnswer {
            page: self.services.dtos.page_to_dto(page)?,
        })
    }

    /// The sidebar sorts pages by `published` and marks the published ones, so the page lists
    /// carry the scalar properties. The list properties stay out, but for the tags of children.
    fn pages_with_scalars(&self, pages: &[Page]) -> Result<PagesAnswer, AppError> {
        let ids: Vec<String> = pages.iter().map(|p| p.id.to_string()).collect();
        let scalars = self
            .services
            .properties
            .find_scalar_by_targets(PAGE_TARGET, &ids)?;
        Ok(PagesAnswer {
            pages: pages
                .iter()
                .map(|p| page_dto_with(p, scalars.get(&p.id.to_string()), None))
                .collect(),
        })
    }
}
