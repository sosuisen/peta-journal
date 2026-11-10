//! The view model: what the shell shows, computed from the model after every event.

use crate::app::{ConnectionState, Model, Screen};
use crate::model::{
    IndexSort, Item, ItemRef, Page, PropertyDefinition, PropertyValue, SearchResult, WeekNumbering,
};
use crate::ops::wiki;
use crate::outline::tree::starts_date_group;
use crate::outline::{FocusRequest, Heading};
use crate::screens::atlas::{self, Bounds};
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

/// One row of the outline: the item as shown plus what the row needs to draw itself.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Row {
    pub item: ItemRef,
    pub index: usize,
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
    pub index: usize,
    pub x: f64,
    pub y: f64,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct OutlineView {
    pub rows: Vec<Row>,
    pub selection_mode: bool,
    pub focus_request: Option<FocusRequest>,
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
    pub value: String,
    pub is_list: bool,
    /// False for the page's own dates, which every page has.
    pub removable: bool,
    /// False for `modified`: the server moves it on every save.
    pub editable: bool,
}

impl PageProperty {
    fn of(key: &str, value: &PropertyValue) -> PageProperty {
        let (value, is_list) = match value {
            PropertyValue::List(values) => (values.join(", "), true),
            PropertyValue::Text(text) => (text.clone(), false),
        };
        PageProperty {
            key: key.to_string(),
            value,
            is_list,
            removable: true,
            editable: true,
        }
    }

    fn date(key: &str, millis: i64) -> PageProperty {
        PageProperty {
            key: key.to_string(),
            value: format_instant(millis),
            is_list: false,
            removable: false,
            editable: key != "modified",
        }
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
        let url = format!("/pages/{}", page.canonical_path());
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
                    url: format!("{url}#{}", i.id),
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
    /// The pages, the create entries and the items, in display order.
    pub results: Vec<SearchEntry>,
    pub show_dropdown: bool,
    pub selected_index: Option<usize>,
    /// The results are still those of an older query, as the answer to this one is on its way.
    pub searching: bool,
}

/// One line of the search results. `index` is what `Event::SearchResultClicked` takes.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SearchEntry {
    pub index: usize,
    pub kind: SearchKind,
    /// The path of the page, or of the page to create.
    pub title: String,
    pub snippet: String,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchKind {
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
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ViewModel {
    pub screen: ScreenView,
    /// The cards of the atlas page on screen; empty on any other screen.
    pub atlas_cards: Vec<AtlasCard>,
    /// The callouts of the atlas page on screen; empty on any other screen.
    pub atlas_callouts: Vec<atlas::Callout>,
    /// The link cards of the atlas page on screen; empty on any other screen.
    pub atlas_links: Vec<atlas::LinkCard>,
    /// The page names offered in the dialog of a link.
    pub atlas_link_suggestions: Vec<String>,
    pub outline: OutlineView,
    pub sidebar: SidebarView,
    pub search: SearchView,
    pub config: ConfigView,
    pub connection: ConnectionState,
    pub today: String,
}

pub fn build(model: &Model) -> ViewModel {
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
        outline: outline(model),
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
        },
        connection: model.connection,
        today: model.today.clone(),
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
                properties: [
                    PageProperty::date("created", pg.created_date),
                    PageProperty::date("modified", pg.modified_date),
                ]
                .into_iter()
                .chain(pg.properties.iter().map(|(k, v)| PageProperty::of(k, v)))
                .collect(),
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

fn outline(model: &Model) -> OutlineView {
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
    let mut rows = Vec::with_capacity(visible.len());
    for (index, item) in visible.iter().enumerate() {
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
        let is_broken = state.is_broken_transclusion(&item.id);
        let date_header = (item.item_type == "journal"
            && item.name != journal_date
            && starts_date_group(visible, own_ids, index))
        .then(|| format_journal_date(&item.name));
        rows.push(Row {
            index,
            indent_px: (item.level - 1) * 24,
            is_editing,
            is_selected: selected.contains(&item.id),
            is_collapsed: item.collapsed || state.is_collapsed(&item.id),
            has_children: state.has_children(&item.id),
            heading_level,
            is_heading: item.body.trim_start().starts_with('#'),
            heading_line: heading_level > 0 && !item.body.contains('\n'),
            task: TaskState::of(item),
            has_own_zoom: (item.zoom() - 1.0).abs() > f64::EPSILON,
            is_transclusion: item.transclude_item_id.is_some(),
            is_broken_transclusion: is_broken,
            show_bullet: is_editing || !item.body.trim().is_empty(),
            effective_zoom,
            date_header,
            suggestions_visible: wiki::is_shown_for(model, &item.id),
            item: item.clone(),
        });
    }

    OutlineView {
        rows,
        selection_mode: state.item_selection_mode(),
        focus_request: state.focus_request().cloned(),
        focused_item_id: state.focused_item_id().map(str::to_string),
        key_relay_holding: model.key_relay.is_holding(),
        wiki: WikiView {
            item_id: model.wiki.item_id.clone(),
            suggestions: model.wiki.suggestions.clone(),
            index: model.wiki.index,
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
    let results = hits(SearchKind::Page, &model.search.page_results)
        .into_iter()
        .chain(creates)
        .chain(hits(SearchKind::Item, &model.search.item_results))
        .enumerate()
        .map(|(index, (kind, title, snippet))| SearchEntry {
            index,
            kind,
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
