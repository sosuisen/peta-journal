//! Journal pages: daily, weekly, monthly and yearly.

use std::sync::Arc;

use chrono::{Datelike, Days, NaiveDate, Weekday};

use crate::domain::id_generator::IdGenerator;
use crate::domain::journal_name::{self, JournalType};
use crate::domain::model::{Item, ItemPosition, Page};
use crate::domain::search::SearchIndex;
use crate::error::{AppError, AppResult};
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::persistence::db::now_millis;
use crate::infrastructure::persistence::{ItemRepository, PageRepository};

use super::config_service::ConfigService;
use super::item_service::ItemService;

const JOURNAL_TYPE: &str = "journal";

/// Result of getting or creating a journal page.
#[derive(Debug, Clone)]
pub struct JournalResult {
    pub page: Page,
    pub items: Vec<Item>,
    pub created: bool,
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
        if let Some(page) = self.pages.find_journal(date)? {
            return self.existing_journal(page);
        }
        let page = self.create_journal_page(&date.format("%Y-%m-%d").to_string())?;
        let item = Item::new_on(self.ids.generate_item_id(), &page, page.created_date);
        let item = self.items.save(&item)?;
        self.export.queue_export(&page.id);
        Ok(JournalResult {
            page,
            items: vec![item],
            created: true,
        })
    }

    /// Gets or creates a journal page by file name: daily (`YYYY-MM-DD`), weekly (`YYYY-WNN`),
    /// monthly (`YYYY-MM`) or yearly (`YYYY`). A new weekly, monthly or yearly journal is filled
    /// with one wiki link per day, week or month it covers.
    pub fn get_or_create_journal_by_name(&self, file_name: &str) -> AppResult<JournalResult> {
        match journal_name::classify(file_name) {
            JournalType::None => Err(AppError::bad_request(format!(
                "Not a journal filename: {file_name}"
            ))),
            JournalType::Daily => {
                let date = NaiveDate::parse_from_str(file_name, "%Y-%m-%d")
                    .map_err(|e| AppError::bad_request(format!("Invalid date {file_name}: {e}")))?;
                self.get_or_create_journal(date)
            }
            JournalType::Weekly => {
                let (y, w) = file_name.split_once("-W").unwrap_or((file_name, "1"));
                let year: i32 = y
                    .parse()
                    .map_err(|_| AppError::bad_request(format!("Invalid week: {file_name}")))?;
                let week: u32 = w
                    .parse()
                    .map_err(|_| AppError::bad_request(format!("Invalid week: {file_name}")))?;
                self.get_or_create_link_journal(
                    &format!("{year}-W{week:02}"),
                    self.week_day_names(year, week)?,
                )
            }
            JournalType::Monthly => {
                let (y, m) = file_name.split_once('-').unwrap_or((file_name, "1"));
                let year: i32 = y
                    .parse()
                    .map_err(|_| AppError::bad_request(format!("Invalid month: {file_name}")))?;
                let month: u32 = m
                    .parse()
                    .map_err(|_| AppError::bad_request(format!("Invalid month: {file_name}")))?;
                self.get_or_create_link_journal(
                    &format!("{year}-{month:02}"),
                    self.week_names_in_month(year, month)?,
                )
            }
            JournalType::Yearly => {
                let year: i32 = file_name
                    .parse()
                    .map_err(|_| AppError::bad_request(format!("Invalid year: {file_name}")))?;
                self.get_or_create_link_journal(&year.to_string(), month_names(year))
            }
        }
    }

    fn get_or_create_link_journal(
        &self,
        file_name: &str,
        link_targets: Vec<String>,
    ) -> AppResult<JournalResult> {
        if let Some(page) =
            self.pages
                .find_by_type_dir_path_file_name(JOURNAL_TYPE, "", file_name)?
        {
            return self.existing_journal(page);
        }
        let page = self.create_journal_page(file_name)?;
        let mut items = Vec::new();
        let mut position = ItemPosition::First;
        for target in link_targets {
            let item = self.item_service.create_item(
                &page.id,
                &position,
                1,
                Some(&format!("[[{target}]]")),
                None,
            )?;
            position = ItemPosition::After(item.id.clone());
            items.push(item);
        }
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

    /// Saves and indexes a new journal page without items.
    fn create_journal_page(&self, file_name: &str) -> AppResult<Page> {
        let now = now_millis();
        let page = Page {
            id: self.ids.generate_page_id(),
            parent_id: None,
            dir_path: String::new(),
            name: file_name.to_string(),
            page_type: JOURNAL_TYPE.to_string(),
            created_date: now,
            modified_date: now,
        };
        let page = self.pages.save(&page)?;
        self.search.index_page(&page);
        Ok(page)
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

    #[test]
    fn week_numbers() {
        let d = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();
        assert_eq!(iso_week_number(d), 8);
        assert_eq!(us_week_number(d, 2026), 8);
        let jan1 = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        assert_eq!(iso_week_number(jan1), 53); // belongs to 2026-W53
        assert_eq!(us_week_number(jan1, 2027), 1);
    }

    #[test]
    fn previous_or_same_weekday() {
        let d = NaiveDate::from_ymd_opt(2026, 1, 4).unwrap(); // Sunday
        assert_eq!(
            previous_or_same(d, Weekday::Mon),
            NaiveDate::from_ymd_opt(2025, 12, 29).unwrap()
        );
        assert_eq!(previous_or_same(d, Weekday::Sun), d);
    }
}
