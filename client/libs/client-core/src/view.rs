//! The view model: what the shell shows, computed from the model after every event.

use crate::app::{ConnectionState, Model, Screen};
use crate::body_model::{Bodies, BodyModel};
use crate::model::ItemExt;
use crate::model::PageExt;
use crate::model::{
    shown_name, value_type_of, IndexSort, Item, ItemRef, Page, PropertyDefinition, PropertyValue,
    SearchResult, WeekNumbering,
};
use crate::ops::wiki;
use crate::outline::tree::starts_date_group;
use crate::outline::{FocusOrder, FocusRequest, Heading};
use crate::screens::atlas::{self, Bounds};
use crate::screens::calendar::{self, CalendarView};
use crate::screens::{journal, page, sidebar};
use crate::util::date::{format_instant, format_journal_date};
use crate::util::filename::JournalType;
use serde::Serialize;

/// Where a task stands, by its `begin` and `end` properties.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TaskState {
    NotStarted,
    /// Begun and not ended: the checkbox shows the in-between state.
    Running,
    Done,
}

impl TaskState {
    /// The state of a task item; `None` for an item that is not a task.
    pub fn of(item: &Item) -> Option<TaskState> {
        if !item.is_task() {
            return None;
        }
        let begun = item.prop_str("begin").is_some();
        let ended = item.prop_str("end").is_some();
        Some(match (begun, ended) {
            (_, true) => TaskState::Done,
            (true, false) => TaskState::Running,
            (false, false) => TaskState::NotStarted,
        })
    }
}

/// One row of the outline: the item as shown plus what the row needs to draw itself. `B` is
/// the model that the shell draws the body from (ADR 042).
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Row<B> {
    pub item: ItemRef,
    /// The model that the shell draws the body from: the AST for the web shell (ADR 041), and
    /// `RichText` for a native shell. The core makes it from the body alone.
    pub body_model: B,
    pub position: usize,
    pub indent_px: i32,
    pub is_editing: bool,
    pub is_selected: bool,
    pub is_collapsed: bool,
    pub has_children: bool,
    /// 1..=5 for a body that is a heading line; 0 otherwise.
    pub heading_level: u8,
    pub is_heading: bool,
    /// True when the whole body is one heading line, which the editor shows large and bold.
    pub heading_line: bool,
    /// How much larger than the usual text the first line of the item is drawn. An item that
    /// is edited shows its Markdown, which is large only when it is one heading line. Every
    /// shell takes it from here (#795).
    pub first_line_scale: f32,
    /// The task's state, for the checkbox; `None` when the item is not a task.
    pub task: Option<TaskState>,
    /// True when the item has a zoom of its own, which the menu offers to reset.
    pub has_own_zoom: bool,
    pub is_transclusion: bool,
    pub is_broken_transclusion: bool,
    pub show_bullet: bool,
    /// The product of the item's zoom and its ancestors' zooms.
    pub effective_zoom: f64,
    /// The formatted date above the first item of a journal day.
    pub date_header: Option<String>,
    pub suggestions_visible: bool,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct WikiView {
    pub item_id: Option<String>,
    pub suggestions: Vec<String>,
    pub selected_index: usize,
    pub x: f64,
    pub y: f64,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OutlineView<B> {
    pub rows: Vec<Row<B>>,
    pub selection_mode: bool,
    pub focus_request: Option<FocusRequest>,
    /// The last order to focus an editor. The core also asks for it with `UiOp::FocusEditor`.
    pub focus_order: Option<FocusOrder>,
    pub focused_item_id: Option<String>,
    pub key_relay_holding: bool,
    pub wiki: WikiView,
    pub scrolled_from_top: bool,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JournalView {
    pub current_date: String,
    pub journal_type: Option<JournalType>,
    pub display_date: String,
    pub page_id: Option<String>,
}

/// One property row of a page: the value is text, a list joined by `, `.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PageProperty {
    pub key: String,
    /// The name the screen shows: the body of the key, with its type when another key
    /// shares the body (see `shown_name`).
    pub name: String,
    pub value: String,
    pub is_list: bool,
    /// The items of a list, in order; none for a value that is no list.
    pub values: Vec<String>,
    /// The type to send a new value with (see `value_type_of`).
    pub value_type: String,
    /// False for the page's own dates, which every page has.
    pub removable: bool,
    /// False for `modified`: the server moves it on every save.
    pub editable: bool,
}

impl PageProperty {
    fn of(key: &str, value: &PropertyValue, definitions: &[PropertyDefinition]) -> PageProperty {
        let (value, is_list, values) = match value {
            PropertyValue::List(values) => (values.join(", "), true, values.clone()),
            PropertyValue::Text(text) => (text.clone(), false, Vec::new()),
        };
        PageProperty {
            key: key.to_string(),
            name: shown_name(key, definitions),
            value,
            is_list,
            values,
            value_type: value_type_of(key, is_list, definitions),
            removable: true,
            editable: true,
        }
    }

    fn date(key: &str, millis: i64, definitions: &[PropertyDefinition]) -> PageProperty {
        PageProperty {
            key: key.to_string(),
            name: key.to_string(),
            value: format_instant(millis),
            is_list: false,
            values: Vec::new(),
            value_type: value_type_of(key, false, definitions),
            removable: false,
            editable: key != "modified",
        }
    }
}

/// A key that a new property row of the page may take.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FreeKey {
    pub key: String,
    /// The name the screen shows (see `shown_name`).
    pub name: String,
    /// The type to send the first value with (see `value_type_of`).
    pub value_type: String,
}

impl FreeKey {
    /// The keys of `definitions` usable on pages that `properties` does not have yet.
    fn of(properties: &[PageProperty], definitions: &[PropertyDefinition]) -> Vec<FreeKey> {
        definitions
            .iter()
            .filter(|d| d.page && !properties.iter().any(|p| p.key == d.key))
            .map(|d| FreeKey {
                key: d.key.clone(),
                name: shown_name(&d.key, definitions),
                value_type: d.value_type.clone(),
            })
            .collect()
    }
}

/// One folder on the way to a page. Why not a pair: the Swift of a native shell cannot compare
/// a type that has a tuple.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DirSegment {
    pub name: String,
    /// The folders from the root down to this one, with `/` between them.
    pub path: String,
    /// The path of the index page of this folder, which the segment opens.
    pub index_path: String,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PageView {
    pub page: Page,
    pub title: String,
    pub dir_segments: Vec<DirSegment>,
    pub is_index: bool,
    pub is_root_index: bool,
    pub is_atlas: bool,
    pub can_demote: bool,
    pub demote_name: String,
    pub focus_item_id: Option<String>,
    pub properties: Vec<PageProperty>,
    /// The keys a new property row may take: usable on pages and not on the page yet.
    pub free_keys: Vec<FreeKey>,
    /// True when the property rows are shown: the page has some, or the menu opened them.
    pub show_properties: bool,
    pub backlinks: Vec<BacklinkGroup>,
}

/// A page that links to the shown page, with its items that link. The items are empty
/// when the page links by the tag in its tags property only.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BacklinkGroup {
    /// The path of the page as the sidebar shows it; a journal as its date.
    pub title: String,
    pub url: String,
    pub items: Vec<BacklinkItem>,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BacklinkItem {
    pub id: String,
    /// The first line of the body.
    pub text: String,
    /// The page URL with the item as its fragment.
    pub url: String,
}

impl BacklinkGroup {
    fn of(page: &Page, items: &[Item]) -> BacklinkGroup {
        let path = page.canonical_path();
        let url = page_urls::page_url(&path);
        BacklinkGroup {
            title: sidebar::page_title(page),
            items: items
                .iter()
                .map(|i| BacklinkItem {
                    id: i.id.clone(),
                    text: i
                        .body
                        .lines()
                        .find(|l| !l.trim().is_empty())
                        .unwrap_or("")
                        .trim()
                        .to_string(),
                    url: page_urls::item_url(&path, &i.id),
                })
                .collect(),
            url,
        }
    }
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ScreenView {
    None,
    Journal(JournalView),
    /// None while the page is loading. Boxed: a page view is far larger than the others.
    Page(Option<Box<PageView>>),
    Settings,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SidebarPage {
    pub page: Page,
    pub label: String,
    /// True when the page has a `published` property; its icon is filled.
    pub published: bool,
    /// True when the entry is a subdirectory of the index list; its icon is a folder.
    pub is_dir: bool,
    /// The directory shown small above `label`, when the page has one.
    pub dir_label: Option<String>,
    pub path: String,
}

/// A page or folder of the directory of an atlas page. A card without bounds waits in the
/// list of unplaced cards.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AtlasCard {
    pub page_id: String,
    pub label: String,
    /// The path that opens the page. A folder opens as its atlas, so the map stays a map
    /// on the way down.
    pub path: String,
    pub is_dir: bool,
    pub bounds: Option<Bounds>,
    /// The key of the group the card belongs to, if any.
    pub group: Option<String>,
    /// The color of that group as `#RRGGBB`.
    pub group_color: Option<String>,
    /// The tags the icon of the card is drawn from; none for no icon.
    pub icon_tags: Vec<String>,
    /// The title of the card's group, the same on every card of it.
    pub group_title: Option<atlas::GroupTitle>,
    /// True when the page has a `published` date; the blob of its card has a line around it.
    pub published: bool,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct SidebarView {
    pub index_header: String,
    /// The path of the index page that the header of the index list opens.
    pub index_path: String,
    pub index_sort: IndexSort,
    /// The path the `../` entry opens; None in the root directory.
    pub index_parent_path: Option<String>,
    pub index_pages: Vec<SidebarPage>,
    pub headings: Vec<Heading>,
    pub recent_pages: Vec<SidebarPage>,
    pub current_page_id: Option<String>,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct SearchView {
    pub query: String,
    /// The entry of today's journal, the pages, the create entries and the items, in display
    /// order.
    pub results: Vec<SearchEntry>,
    pub show_dropdown: bool,
    pub selected_index: Option<usize>,
    /// The results are still those of an older query, as the answer to this one is on its way.
    pub searching: bool,
}

/// One line of the search results. `position` is what `Event::SearchResultClicked` takes.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchEntry {
    pub position: usize,
    pub kind: SearchKind,
    /// The path of the page, or of the page to create.
    pub title: String,
    /// What the line says. Every shell shows it as it is, so that all say the same.
    pub label: String,
    pub snippet: String,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchKind {
    /// Opens today's journal.
    Today,
    Page,
    Create,
    Item,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct ConfigView {
    pub markdown_dir: String,
    pub week_numbering: WeekNumbering,
    pub logical_outdent: bool,
    pub tags_dir: String,
    pub home_link: String,
    pub property_definitions: Vec<PropertyDefinition>,
    pub definition_error: Option<String>,
    /// How many keys the server accepted from the settings form. The shell clears the form
    /// when it grows.
    pub definitions_added: u32,
}

/// Where the history of the opened pages can go (ADR 035). A shell that keeps no history in
/// the core, as the web app, which has the history of the browser, has nothing in it.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct HistoryView {
    pub can_go_back: bool,
    pub can_go_forward: bool,
    /// The URL on screen, as it was opened. A shell keeps it to open the screen again at the
    /// next start. None while nothing was opened through the history.
    pub url: Option<String>,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ViewModel<B> {
    pub screen: ScreenView,
    /// The cards of the atlas page on screen; empty on any other screen.
    pub atlas_cards: Vec<AtlasCard>,
    /// The callouts of the atlas page on screen; empty on any other screen.
    pub atlas_callouts: Vec<atlas::Callout>,
    /// The link cards of the atlas page on screen; empty on any other screen.
    pub atlas_links: Vec<atlas::LinkCard>,
    /// The page names offered in the dialog of a link.
    pub atlas_link_suggestions: Vec<String>,
    pub outline: OutlineView<B>,
    pub sidebar: SidebarView,
    pub search: SearchView,
    pub config: ConfigView,
    pub connection: ConnectionState,
    pub today: String,
    pub calendar: CalendarView,
    /// The name of what is open, for the title of a window. For a journal it is the date of
    /// the day at the top, with "(Today)" behind the date of today. For anything else it is
    /// the name the core last gave with `UiOp::SetTitle`, which a browser shows on its tab.
    pub title: String,
    pub history: HistoryView,
}

pub fn build<B: BodyModel>(model: &Model, bodies: &Bodies<B>) -> ViewModel<B> {
    ViewModel {
        screen: screen(model),
        atlas_cards: atlas::cards(model)
            .into_iter()
            .map(|(p, bounds, group, group_title)| AtlasCard {
                page_id: p.id.clone(),
                label: sidebar::index_page_name(&p),
                path: if sidebar::is_directory(&p) {
                    format!("{}{}", p.dir_path, page::ATLAS_NAME)
                } else {
                    p.canonical_path()
                },
                is_dir: sidebar::is_directory(&p),
                bounds,
                group_color: group.as_deref().map(atlas::group_color),
                icon_tags: atlas::icon_tags(&p, &model.config.tags_dir),
                group,
                group_title,
                published: sidebar::is_published(&p),
            })
            .collect(),
        atlas_callouts: atlas::shown_callouts(model),
        atlas_links: atlas::link_cards(model),
        atlas_link_suggestions: model.atlas_link_search.suggestions.clone(),
        outline: outline(model, bodies),
        sidebar: sidebar(model),
        search: search(model),
        config: ConfigView {
            markdown_dir: model.config.markdown_dir.clone(),
            week_numbering: model.config.week_numbering,
            logical_outdent: model.config.logical_outdent,
            tags_dir: model.config.tags_dir.clone(),
            home_link: model.config.home_link.clone(),
            property_definitions: model.config.property_definitions.clone(),
            definition_error: model.config.definition_error.clone(),
            definitions_added: model.config.definitions_added,
        },
        connection: model.connection,
        today: model.today.clone(),
        calendar: calendar::view(model),
        title: match &model.screen {
            Screen::Journal(j) => journal::window_title(j, &model.today),
            _ => model.title.clone(),
        },
        history: model.history.view(),
    }
}

fn screen(model: &Model) -> ScreenView {
    match &model.screen {
        Screen::None => ScreenView::None,
        Screen::Settings => ScreenView::Settings,
        Screen::Journal(j) => ScreenView::Journal(JournalView {
            current_date: j.current_date.clone(),
            journal_type: j.journal_type,
            display_date: journal::display_date(j),
            page_id: j.page.as_ref().map(|p| p.id.clone()),
        }),
        Screen::Page(p) => ScreenView::Page(p.page.as_ref().map(|pg| {
            let definitions = &model.config.property_definitions;
            let properties: Vec<PageProperty> = [
                PageProperty::date("created", pg.created_date, definitions),
                PageProperty::date("modified", pg.modified_date, definitions),
            ]
            .into_iter()
            .chain(
                pg.properties
                    .iter()
                    .map(|(k, v)| PageProperty::of(k, v, definitions)),
            )
            .collect();
            Box::new(PageView {
                page: pg.clone(),
                title: page::page_title(pg),
                dir_segments: page::dir_path_segments(&pg.dir_path),
                is_index: pg.name == "index",
                is_root_index: pg.name == "index" && pg.dir_path.is_empty(),
                is_atlas: page::is_atlas(pg),
                can_demote: p.can_demote_index,
                demote_name: page::demote_name(pg),
                focus_item_id: p.focus_item_id.clone(),
                free_keys: FreeKey::of(&properties, definitions),
                properties,
                // The dates do not count: every page has them
                show_properties: p.properties_open || !pg.properties.is_empty(),
                backlinks: page::backlink_groups(&p.backlinks)
                    .iter()
                    .map(|(page, items)| BacklinkGroup::of(page, items))
                    .collect(),
            })
        })),
    }
}

fn outline<B: BodyModel>(model: &Model, bodies: &Bodies<B>) -> OutlineView<B> {
    let state = &model.state;
    let visible = state.visible_items();
    let own_ids = state.own_item_ids();
    let journal_date = journal::journal(model)
        .map(|j| j.current_date.clone())
        .unwrap_or_default();
    let selected = state.selected_item_ids();
    let editing = state.editing_item_ids();

    // Cumulative zoom: the product of the zooms of the item and its ancestors
    let mut stack: Vec<(i32, f64)> = Vec::new();
    let texts = visible.iter().map(|item| item.body.as_str());
    let body_models = bodies.of(texts, &model.config.tags_dir);
    let mut rows = Vec::with_capacity(visible.len());
    for ((position, item), body_model) in visible.iter().enumerate().zip(body_models) {
        while stack
            .last()
            .map(|(level, _)| *level >= item.level)
            .unwrap_or(false)
        {
            stack.pop();
        }
        let parent_zoom = stack.last().map(|(_, z)| *z).unwrap_or(1.0);
        let effective_zoom = parent_zoom * item.zoom();
        stack.push((item.level, effective_zoom));

        let is_editing = editing.contains(&item.id);
        let heading_level = item.heading_level_of_body();
        let heading_line = heading_level > 0 && !item.body.contains('\n');
        let is_broken = state.is_broken_transclusion(&item.id);
        let date_header = (item.item_type == "journal"
            && item.name != journal_date
            && starts_date_group(visible, own_ids, position))
        .then(|| format_journal_date(&item.name));
        rows.push(Row {
            position,
            indent_px: (item.level - 1) * 24,
            is_editing,
            is_selected: selected.contains(&item.id),
            is_collapsed: item.collapsed || state.is_collapsed(&item.id),
            has_children: state.has_children(&item.id),
            heading_level,
            is_heading: item.body.trim_start().starts_with('#'),
            heading_line,
            first_line_scale: crate::rich_text::heading_scale(if is_editing && !heading_line {
                0
            } else {
                heading_level
            }),
            task: TaskState::of(item),
            has_own_zoom: (item.zoom() - 1.0).abs() > f64::EPSILON,
            is_transclusion: item.transclude_item_id.is_some(),
            is_broken_transclusion: is_broken,
            show_bullet: is_editing || !item.body.trim().is_empty(),
            effective_zoom,
            date_header,
            suggestions_visible: wiki::is_shown_for(model, &item.id),
            body_model,
            item: item.clone(),
        });
    }

    OutlineView {
        rows,
        selection_mode: state.item_selection_mode(),
        focus_request: state.focus_request().cloned(),
        focus_order: state.focus_order().cloned(),
        focused_item_id: state.focused_item_id().map(str::to_string),
        key_relay_holding: model.key_relay.is_holding(),
        wiki: WikiView {
            item_id: model.wiki.item_id.clone(),
            suggestions: model.wiki.suggestions.clone(),
            selected_index: model.wiki.index,
            x: model.wiki.x,
            y: model.wiki.y,
        },
        scrolled_from_top: model.scrolled_from_top,
    }
}

fn sidebar(model: &Model) -> SidebarView {
    let headings = match &model.current_page {
        Some(p) if p.page_type == "doc" => model.state.headings(),
        _ => Vec::new(),
    };
    let index_sort = sidebar::index_sort(model);
    SidebarView {
        index_header: model.sidebar.index_header.clone(),
        index_path: sidebar::index_path(&model.sidebar.index_header),
        index_sort,
        index_parent_path: sidebar::parent_index_path(&model.sidebar.index_header),
        index_pages: sidebar::sorted_index_pages(&model.sidebar.index_pages, index_sort)
            .into_iter()
            .map(|p| SidebarPage {
                label: sidebar::index_page_name(&p),
                published: sidebar::is_published(&p),
                is_dir: sidebar::is_directory(&p),
                dir_label: None,
                path: format!("{}{}", p.dir_path, p.name),
                page: p,
            })
            .collect(),
        headings,
        recent_pages: model
            .recent_pages
            .iter()
            .map(|p| {
                let (dir_label, label) = sidebar::recent_page_label(p);
                SidebarPage {
                    label,
                    published: sidebar::is_published(p),
                    is_dir: false,
                    dir_label,
                    path: format!("{}{}", p.dir_path, p.name),
                    page: p.clone(),
                }
            })
            .collect(),
        current_page_id: model.current_page.as_ref().map(|p| p.id.clone()),
    }
}

/// What the search line of this kind and title says.
fn search_label(kind: SearchKind, title: &str) -> String {
    match kind {
        SearchKind::Create => format!("Create page \"{title}\""),
        SearchKind::Today | SearchKind::Page | SearchKind::Item => title.to_string(),
    }
}

fn search(model: &Model) -> SearchView {
    let hits = |kind, results: &[SearchResult]| {
        results
            .iter()
            .map(|r| (kind, r.page_title.clone(), r.snippet.clone()))
            .collect::<Vec<_>>()
    };
    let creates = crate::screens::search::create_titles(model)
        .into_iter()
        .map(|title| (SearchKind::Create, title, String::new()));
    let today = crate::screens::search::offers_today(model).then(|| {
        let title = "Open Today".to_string();
        (SearchKind::Today, title, String::new())
    });
    let results = today
        .into_iter()
        .chain(hits(SearchKind::Page, &model.search.page_results))
        .chain(creates)
        .chain(hits(SearchKind::Item, &model.search.item_results))
        .enumerate()
        .map(|(position, (kind, title, snippet))| SearchEntry {
            position,
            kind,
            label: search_label(kind, &title),
            title,
            snippet,
        })
        .collect();
    SearchView {
        query: model.search.query.clone(),
        results,
        show_dropdown: model.search.show_dropdown,
        selected_index: model.search.selected_index,
        searching: model.search.searching,
    }
}

#[cfg(test)]
mod timing;
