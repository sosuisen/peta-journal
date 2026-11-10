//! Journal pages: daily, weekly, monthly and yearly.

use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::{Datelike, Days, NaiveDate, Weekday};
use rusqlite::Connection;

use crate::domain::id_generator::IdGenerator;
use crate::domain::journal_name::JournalName;
use crate::domain::model::{Item, ItemPosition, Page};
use crate::domain::search::SearchIndex;
use crate::error::{AppError, AppResult};
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::persistence::db::now_millis;
use crate::infrastructure::persistence::{ItemRepository, PageRepository};
use hmtid::Hmtid;

use super::config_service::ConfigService;
use super::item_service::{AfterCommit, ItemService};

const JOURNAL_TYPE: &str = "journal";

/// Result of getting or creating a journal page.
#[derive(Debug, Clone)]
pub struct JournalResult {
    pub page: Page,
    pub items: Vec<Item>,
    pub created: bool,
}

/// What a move of items to the journal of a day did.
#[derive(Debug, Clone)]
pub struct MovedToJournal {
    pub page: Page,
    pub created: bool,
    pub items: Vec<Item>,
    /// The pages the move changed: the journal and the pages the items left.
    pub page_ids: BTreeSet<Hmtid>,
}

/// Result of finding the most recent journal before a date.
#[derive(Debug, Clone)]
pub struct MostRecentJournalResult {
    pub page: Option<Page>,
    pub items: Vec<Item>,
}

impl MostRecentJournalResult {
    pub fn found(&self) -> bool {
        self.page.is_some()
    }
}

pub struct JournalService {
    pages: PageRepository,
    items: ItemRepository,
    search: Arc<dyn SearchIndex>,
    config: Arc<ConfigService>,
    item_service: Arc<ItemService>,
    ids: Arc<IdGenerator>,
    export: Arc<MarkdownExportService>,
}

impl JournalService {
    pub fn new(
        pages: PageRepository,
        items: ItemRepository,
        search: Arc<dyn SearchIndex>,
        config: Arc<ConfigService>,
        item_service: Arc<ItemService>,
        ids: Arc<IdGenerator>,
        export: Arc<MarkdownExportService>,
    ) -> JournalService {
        JournalService {
            pages,
            items,
            search,
            config,
            item_service,
            ids,
            export,
        }
    }

    /// Gets an existing daily journal page for the given date, or creates one with a single empty item.
    pub fn get_or_create_journal(&self, date: NaiveDate) -> AppResult<JournalResult> {
        let file_name = date.format("%Y-%m-%d").to_string();
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        if let Some(page) = Self::find_journal_in(&tx, &file_name)? {
            return self.existing_journal(page);
        }
        let page = Self::create_journal_page_in(&tx, self.ids.generate_page_id(), &file_name)?;
        let item = Item::new_on(self.ids.generate_item_id(), &page, page.created_date);
        ItemRepository::save_in(&tx, &item)?;
        tx.commit()?;
        self.search.index_page(&page);
        self.export.queue_export(&page.id);
        Ok(JournalResult {
            page,
            items: vec![item],
            created: true,
        })
    }

    /// Moves items to the daily journal of `date`, as a move key at the edge of a day does,
    /// in one transaction (ADR 011): the journal is made when there is none, its single
    /// empty item goes, the items move with `body` (the text the editor of the first item
    /// has now) saved to the first item, and a page the items leave empty keeps one empty
    /// item. The caller lets the extensions answer a journal that was made.
    pub fn move_items_to_journal(
        &self,
        date: NaiveDate,
        item_ids: &[Hmtid],
        position: &ItemPosition,
        body: Option<&str>,
    ) -> AppResult<MovedToJournal> {
        let file_name = date.format("%Y-%m-%d").to_string();
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        let (page, created) = match Self::find_journal_in(&tx, &file_name)? {
            Some(page) => {
                self.remove_single_empty_item_in(&tx, &page.id, &mut effects)?;
                (page, false)
            }
            None => (
                Self::create_journal_page_in(&tx, self.ids.generate_page_id(), &file_name)?,
                true,
            ),
        };
        let left_pages = Self::pages_of_in(&tx, item_ids)?;
        let items = self.item_service.move_edited_items_in(
            &tx,
            item_ids,
            &page.id,
            position,
            None,
            body,
            &mut effects,
        )?;
        for left in left_pages.iter().filter(|left| **left != page.id) {
            self.keep_page_non_empty_in(&tx, left, &mut effects)?;
        }
        tx.commit()?;
        if created {
            self.search.index_page(&page);
        }
        let page_ids = effects.pages().clone();
        self.item_service.finish(effects);
        Ok(MovedToJournal {
            page,
            created,
            items,
            page_ids,
        })
    }

    /// Removes the one empty item a journal starts with, so that it does not stay above or
    /// below the items that move in.
    fn remove_single_empty_item_in(
        &self,
        tx: &Connection,
        page_id: &Hmtid,
        effects: &mut AfterCommit,
    ) -> AppResult<()> {
        let items = ItemRepository::find_by_page_id_in(tx, page_id)?;
        if let [only] = items.as_slice() {
            if only.body.is_empty() {
                self.item_service
                    .remove_group_in(tx, std::slice::from_ref(&only.id), effects)?;
            }
        }
        Ok(())
    }

    /// A page keeps at least one item: an empty one is made when the page has none left.
    fn keep_page_non_empty_in(
        &self,
        tx: &Connection,
        page_id: &Hmtid,
        effects: &mut AfterCommit,
    ) -> AppResult<()> {
        if ItemRepository::find_by_page_id_in(tx, page_id)?.is_empty() {
            self.item_service.create_item_in(
                tx,
                page_id,
                &ItemPosition::First,
                1,
                "",
                None,
                effects,
            )?;
        }
        Ok(())
    }

    /// The pages the items are on.
    fn pages_of_in(tx: &Connection, item_ids: &[Hmtid]) -> AppResult<BTreeSet<Hmtid>> {
        let mut pages = BTreeSet::new();
        for id in item_ids {
            let item = ItemRepository::find_by_id_in(tx, id)?
                .ok_or_else(|| AppError::not_found(format!("Item not found: {id}")))?;
            pages.insert(item.page_id);
        }
        Ok(pages)
    }

    /// Gets or creates a journal page by name. A new weekly, monthly or yearly journal is
    /// filled with one wiki link per day, week or month it covers.
    pub fn get_or_create_journal_by_name(&self, name: JournalName) -> AppResult<JournalResult> {
        match name {
            JournalName::Daily(date) => self.get_or_create_journal(date),
            JournalName::Weekly { year, week } => {
                self.get_or_create_link_journal(&name.to_string(), self.week_day_names(year, week)?)
            }
            JournalName::Monthly { year, month } => self.get_or_create_link_journal(
                &name.to_string(),
                self.week_names_in_month(year, month)?,
            ),
            JournalName::Yearly(year) => {
                self.get_or_create_link_journal(&name.to_string(), month_names(year))
            }
        }
    }

    fn get_or_create_link_journal(
        &self,
        file_name: &str,
        link_targets: Vec<String>,
    ) -> AppResult<JournalResult> {
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        if let Some(page) = Self::find_journal_in(&tx, file_name)? {
            return self.existing_journal(page);
        }
        let page = Self::create_journal_page_in(&tx, self.ids.generate_page_id(), file_name)?;
        let mut effects = AfterCommit::default();
        let mut items = Vec::new();
        let mut position = ItemPosition::First;
        for target in link_targets {
            let item = self.item_service.create_item_in(
                &tx,
                &page.id,
                &position,
                1,
                &format!("[[{target}]]"),
                None,
                &mut effects,
            )?;
            position = ItemPosition::After(item.id.clone());
            items.push(item);
        }
        tx.commit()?;
        self.search.index_page(&page);
        // The items are indexed and the page is written by the item effects
        self.item_service.finish(effects);
        Ok(JournalResult {
            page,
            items,
            created: true,
        })
    }

    fn existing_journal(&self, page: Page) -> AppResult<JournalResult> {
        let items = self.items.find_by_page_id(&page.id)?;
        Ok(JournalResult {
            page,
            items,
            created: false,
        })
    }

    fn find_journal_in(tx: &Connection, file_name: &str) -> AppResult<Option<Page>> {
        PageRepository::find_by_type_dir_path_file_name_in(tx, JOURNAL_TYPE, "", file_name)
    }

    /// Saves a new journal page without items, in the transaction. The caller indexes it
    /// once the transaction is committed.
    fn create_journal_page_in(tx: &Connection, id: Hmtid, file_name: &str) -> AppResult<Page> {
        PageRepository::save_in(
            tx,
            &Page::new(id, "", file_name, JOURNAL_TYPE, now_millis()),
        )
    }

    fn is_iso_week_numbering(&self) -> bool {
        self.config.week_numbering() == "iso"
    }

    /// The names of the seven daily journals of a week, e.g. `2026-02-16`.
    fn week_day_names(&self, year: i32, week: u32) -> AppResult<Vec<String>> {
        let start = if self.is_iso_week_numbering() {
            // ISO: weeks start Monday, Jan 4 is always in week 1
            let jan4 = ymd(year, 1, 4)?;
            previous_or_same(jan4, Weekday::Mon) + Days::new(7 * (week as u64).saturating_sub(1))
        } else {
            // US: weeks start Sunday, week 1 contains Jan 1
            let jan1 = ymd(year, 1, 1)?;
            previous_or_same(jan1, Weekday::Sun) + Days::new(7 * (week as u64).saturating_sub(1))
        };
        Ok((0..7)
            .map(|i| (start + Days::new(i)).format("%Y-%m-%d").to_string())
            .collect())
    }

    /// The names of the weekly journals that overlap a month, e.g. `2026-W07`.
    fn week_names_in_month(&self, year: i32, month: u32) -> AppResult<Vec<String>> {
        let iso = self.is_iso_week_numbering();
        let first = ymd(year, month, 1)?;
        let last = last_day_of_month(year, month)?;
        let mut weeks: Vec<u32> = Vec::new();
        let mut date = first;
        while date <= last {
            let week_num = if iso {
                iso_week_number(date)
            } else {
                us_week_number(date, year)
            };
            if weeks.last() != Some(&week_num) {
                weeks.push(week_num);
            }
            date = date + Days::new(1);
        }
        Ok(weeks.iter().map(|w| format!("{year}-W{w:02}")).collect())
    }

    pub fn get_most_recent_journal_before(
        &self,
        before: NaiveDate,
    ) -> AppResult<MostRecentJournalResult> {
        self.to_most_recent(self.pages.find_most_recent_journal_before(before)?)
    }

    pub fn get_most_recent_weekly_journal_before(
        &self,
        before_week: &str,
    ) -> AppResult<MostRecentJournalResult> {
        self.to_most_recent(
            self.pages
                .find_most_recent_weekly_journal_before(before_week)?,
        )
    }

    fn to_most_recent(&self, page: Option<Page>) -> AppResult<MostRecentJournalResult> {
        match page {
            Some(p) => {
                let items = self.items.find_by_page_id(&p.id)?;
                Ok(MostRecentJournalResult {
                    page: Some(p),
                    items,
                })
            }
            None => Ok(MostRecentJournalResult {
                page: None,
                items: Vec::new(),
            }),
        }
    }
}

fn ymd(year: i32, month: u32, day: u32) -> AppResult<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| AppError::bad_request(format!("Invalid date: {year}-{month}-{day}")))
}

fn last_day_of_month(year: i32, month: u32) -> AppResult<NaiveDate> {
    let next = if month == 12 {
        ymd(year + 1, 1, 1)?
    } else {
        ymd(year, month + 1, 1)?
    };
    Ok(next.pred_opt().unwrap_or(next))
}

fn previous_or_same(date: NaiveDate, weekday: Weekday) -> NaiveDate {
    let diff = (date.weekday().num_days_from_monday() + 7 - weekday.num_days_from_monday()) % 7;
    date - Days::new(diff as u64)
}

/// The names of the twelve monthly journals of a year, e.g. `2026-02`.
fn month_names(year: i32) -> Vec<String> {
    (1..=12).map(|m| format!("{year}-{m:02}")).collect()
}

fn iso_week_number(date: NaiveDate) -> u32 {
    // ISO 8601: week starts Monday, Jan 4 is always in week 1
    let jan4 = NaiveDate::from_ymd_opt(date.year(), 1, 4).unwrap_or(date);
    let monday1 = previous_or_same(jan4, Weekday::Mon);
    if date < monday1 {
        let prev_jan4 = NaiveDate::from_ymd_opt(date.year() - 1, 1, 4).unwrap_or(date);
        let prev_monday1 = previous_or_same(prev_jan4, Weekday::Mon);
        return ((date - prev_monday1).num_days() / 7) as u32 + 1;
    }
    ((date - monday1).num_days() / 7) as u32 + 1
}

fn us_week_number(date: NaiveDate, year: i32) -> u32 {
    // US: week 1 contains Jan 1, weeks start Sunday
    let jan1 = NaiveDate::from_ymd_opt(year, 1, 1).unwrap_or(date);
    let sunday1 = previous_or_same(jan1, Weekday::Sun);
    ((date - sunday1).num_days() / 7) as u32 + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    // ISO週番号と米国式週番号は、年の変わり目で食い違う。
    #[test]
    fn iso_and_us_week_numbers_part_at_the_turn_of_the_year() {
        let d = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();
        assert_eq!(iso_week_number(d), 8);
        assert_eq!(us_week_number(d, 2026), 8);
        let jan1 = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        assert_eq!(iso_week_number(jan1), 53); // belongs to 2026-W53
        assert_eq!(us_week_number(jan1, 2027), 1);
    }

    // 指定の曜日の、同じ日か直前の日が見つかる。
    #[test]
    fn the_previous_or_same_weekday_is_found() {
        let d = NaiveDate::from_ymd_opt(2026, 1, 4).unwrap(); // Sunday
        assert_eq!(
            previous_or_same(d, Weekday::Mon),
            NaiveDate::from_ymd_opt(2025, 12, 29).unwrap()
        );
        assert_eq!(previous_or_same(d, Weekday::Sun), d);
    }
}
