//! The journal operations: the pages of a day, a week, a month and a year.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use hmtid::Hmtid;
use serde::Serialize;

use crate::application::journal_service::{naive_date, MostRecentJournalResult};
use crate::domain::journal_name::JournalName;
use crate::dto::{ItemDto, PageDto};
use crate::error::{parse_id, AppError};

use super::{ChangedBy, Journal};
pub use journal_api_types::MoveToJournalInput;

/// A journal that was found or made.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalAnswer {
    pub page: PageDto,
    pub items: Vec<ItemDto>,
    pub created: bool,
    pub transcluded_trees: BTreeMap<String, Vec<ItemDto>>,
}

/// The journal the items moved to, and the items as they are now.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MovedToJournalAnswer {
    pub page: PageDto,
    pub created: bool,
    pub items: Vec<ItemDto>,
}

/// The newest journal before a day or a week. `found` is false when there is none.
/// The server and the clients share the type (ADR 082).
pub type MostRecentJournalAnswer = journal_api_types::JournalBefore<PageDto, ItemDto>;

impl Journal {
    /// Finds the journal of the name, or makes it. The name is a day (`2026-11-11`), a week
    /// (`2026-W46`), a month (`2026-11`) or a year (`2026`). The extensions answer a journal
    /// that was made, and their items come with it.
    pub fn get_or_create_journal(&self, name: &str) -> Result<JournalAnswer, AppError> {
        let mut result = self
            .services
            .journal_service
            .get_or_create_journal_by_name(name.parse::<JournalName>()?)?;
        if result.created {
            result.items = self.services.extensions.page_created(&result.page)?;
        }
        Ok(JournalAnswer {
            page: self.services.dtos.page_to_dto(&result.page)?,
            items: self.services.dtos.to_dtos(&result.items)?,
            created: result.created,
            transcluded_trees: self.services.dtos.transcluded_trees(&result.items)?,
        })
    }

    /// Moves items to the journal of a day, as a move key at the edge of a day does (ADR 011).
    pub fn move_items_to_journal(
        &self,
        name: &str,
        input: MoveToJournalInput,
        by: &ChangedBy,
    ) -> Result<MovedToJournalAnswer, AppError> {
        let JournalName::Daily(date) = name.parse::<JournalName>()? else {
            return Err(AppError::bad_request(
                "Items move to the journal of a day only",
            ));
        };
        let item_ids: Vec<Hmtid> = input
            .item_ids
            .iter()
            .map(|s| parse_id(s))
            .collect::<Result<_, AppError>>()?;
        let position = super::items::parse_position(Some(input.position.clone()))?;
        let moved = self.services.journal_service.move_items_to_journal(
            naive_date(date)?,
            &item_ids,
            &position,
            input.body.as_deref(),
        )?;
        if moved.created {
            // The extensions see the journal with the items that moved in
            self.services.extensions.page_created(&moved.page)?;
        }
        self.tell(&moved.page_ids, by);
        Ok(MovedToJournalAnswer {
            page: self.services.dtos.page_to_dto(&moved.page)?,
            created: moved.created,
            items: self.services.dtos.to_dtos(&moved.items)?,
        })
    }

    /// The newest journal of a day before the date (`2026-11-11`).
    pub fn most_recent_journal_before(
        &self,
        date: &str,
    ) -> Result<MostRecentJournalAnswer, AppError> {
        let before = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|e| AppError::bad_request(format!("Invalid date '{date}': {e}")))?;
        self.most_recent_answer(
            self.services
                .journal_service
                .get_most_recent_journal_before(before)?,
        )
    }

    /// The newest journal of a week before the week (`2026-W46`).
    pub fn most_recent_weekly_journal_before(
        &self,
        week: &str,
    ) -> Result<MostRecentJournalAnswer, AppError> {
        self.most_recent_answer(
            self.services
                .journal_service
                .get_most_recent_weekly_journal_before(week)?,
        )
    }

    fn most_recent_answer(
        &self,
        result: MostRecentJournalResult,
    ) -> Result<MostRecentJournalAnswer, AppError> {
        match result.page {
            None => Ok(MostRecentJournalAnswer {
                found: false,
                page: None,
                items: Vec::new(),
                transcluded_trees: BTreeMap::new(),
            }),
            Some(page) => Ok(MostRecentJournalAnswer {
                found: true,
                page: Some(self.services.dtos.page_to_dto(&page)?),
                items: self.services.dtos.to_dtos(&result.items)?,
                transcluded_trees: self.services.dtos.transcluded_trees(&result.items)?,
            }),
        }
    }
}
