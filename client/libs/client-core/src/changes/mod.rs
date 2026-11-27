//! What changed on the screen since a shell was last told (ADR 076): the changes to the rows
//! of the outline, and each panel that changed, whole. A shell of any body model takes them
//! (ADR 078).

mod diff;
pub mod examples;
#[cfg(feature = "border_examples")]
pub mod samples;

pub use diff::{list_changes, ListChange};

crate::list_change!(
    /// One change to the rows of the outline, as the shells take it.
    #[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
    #[derive(Debug, Clone, PartialEq, Serialize)]
    RowChange<B>,
    Row<B>
);

crate::list_change!(
    /// One change to the headings of the page on screen, as the shells take it.
    #[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
    #[derive(Debug, Clone, PartialEq, Serialize)]
    HeadingChange,
    Heading
);

crate::list_change!(
    /// One change to a list of pages of the sidebar, as the shells take it.
    #[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
    #[derive(Debug, Clone, PartialEq, Serialize)]
    PageChange,
    SidebarPage
);

use crate::app::ConnectionState;
use crate::model::IndexSort;
use crate::outline::{FocusOrder, FocusRequest, Heading};
use crate::screens::atlas::{Callout, LinkCard};
use crate::screens::calendar::CalendarView;
use crate::view::{
    AtlasCard, ConfigView, HistoryView, LoadState, Row, ScreenView, SearchView, SidebarPage,
    WikiView,
};
use crate::ViewModel;
use serde::Serialize;

/// What changed since the shell was last told.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Changes<B> {
    /// The changes to the rows of the outline, to apply in order.
    pub rows: Vec<RowChange<B>>,
    /// The changes to the headings of the page on screen, to apply in order (ADR 088).
    pub headings: Vec<HeadingChange>,
    /// The changes to the pages of the folder of the page on screen, in the order of the sort,
    /// to apply in order (ADR 088).
    pub index_pages: Vec<PageChange>,
    /// The changes to the doc pages changed last, to apply in order (ADR 088).
    pub recent_pages: Vec<PageChange>,
    /// The panels that changed, each whole.
    pub panels: Vec<Panel>,
}

/// A part of the screen that is small and of a fixed shape: one field of the view model. It is
/// sent whole when it changed.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Panel {
    Screen(ScreenView),
    AtlasCards(Vec<AtlasCard>),
    AtlasCallouts(Vec<Callout>),
    AtlasLinks(Vec<LinkCard>),
    AtlasLinkSuggestions(Vec<String>),
    Outline(OutlinePanel),
    IndexPages(IndexPagesPanel),
    Headings(HeadingsPanel),
    RecentPages(RecentPagesPanel),
    Search(SearchView),
    Config(ConfigView),
    Connection(ConnectionState),
    Today(String),
    Calendar(CalendarView),
    Title(String),
    History(HistoryView),
}

/// The outline of the view model without its rows, which change as a list.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OutlinePanel {
    pub selection_mode: bool,
    pub focus_request: Option<FocusRequest>,
    pub focus_order: Option<FocusOrder>,
    pub focused_item_id: Option<String>,
    pub key_relay_holding: bool,
    pub wiki: WikiView,
    pub scrolled_from_top: bool,
}

/// The headings of the view model without their list, which changes as a list.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HeadingsPanel {
    pub state: LoadState,
}

/// The pages of the folder in the view model without their list, which changes as a list.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IndexPagesPanel {
    pub header: String,
    /// The path of the index page that the header of the list opens.
    pub path: String,
    pub sort: IndexSort,
    /// The path the `../` entry opens; None in the root directory.
    pub parent_path: Option<String>,
    /// The page on screen, which the list marks.
    pub current_page_id: Option<String>,
    pub state: LoadState,
}

/// The recent pages in the view model without their list, which changes as a list.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecentPagesPanel {
    pub state: LoadState,
}

/// What the shell was last told: the rows and the panels it has now.
pub struct Shown<B> {
    /// The rows the shell has. None before it was first told.
    rows: Option<Vec<Row<B>>>,
    /// The headings the shell has. None before it was first told.
    headings: Option<Vec<Heading>>,
    /// The pages of the folder the shell has. None before it was first told.
    index_pages: Option<Vec<SidebarPage>>,
    /// The recent pages the shell has. None before it was first told.
    recent_pages: Option<Vec<SidebarPage>>,
    panels: ShownPanels,
}

// Not derived: the derive would ask `B: Default`, which no body model needs to be
impl<B> Default for Shown<B> {
    fn default() -> Shown<B> {
        Shown {
            rows: None,
            headings: None,
            index_pages: None,
            recent_pages: None,
            panels: ShownPanels::default(),
        }
    }
}

impl<B: Clone + PartialEq> Shown<B> {
    /// What changed from what the shell has to `view`. The shell has `view` after this.
    pub fn changes(&mut self, view: ViewModel<B>) -> Changes<B> {
        let panels = self.panels.changes(&view);
        let rows = changes_of(&mut self.rows, view.outline.rows, |row| &row.item.id);
        let headings = changes_of(&mut self.headings, view.headings.headings, |heading| {
            &heading.id
        });
        let index_pages = changes_of(&mut self.index_pages, view.index_pages.pages, page_id);
        let recent_pages = changes_of(&mut self.recent_pages, view.recent_pages.pages, page_id);
        Changes {
            rows: rows.into_iter().map(RowChange::from).collect(),
            headings: headings.into_iter().map(HeadingChange::from).collect(),
            index_pages: index_pages.into_iter().map(PageChange::from).collect(),
            recent_pages: recent_pages.into_iter().map(PageChange::from).collect(),
            panels,
        }
    }
}

/// The key of a page of the sidebar in its list.
fn page_id(page: &SidebarPage) -> &str {
    &page.page.id
}

/// The changes from the list the shell has to `now`: a reset the first time. The shell has
/// `now` after this.
fn changes_of<T: Clone + PartialEq>(
    had: &mut Option<Vec<T>>,
    now: Vec<T>,
    id: impl Fn(&T) -> &str,
) -> Vec<ListChange<T>> {
    let changes = match had {
        None => vec![ListChange::Reset { items: now.clone() }],
        Some(had) => list_changes(had, &now, id),
    };
    *had = Some(now);
    changes
}

/// The panels the shell was last told, for a shell that tells its rows in its own way, as
/// the Emacs shell does.
#[derive(Default)]
pub struct ShownPanels {
    /// None before the shell was first told.
    had: Option<Vec<Panel>>,
}

impl ShownPanels {
    /// The panels of `view` that changed since the shell was last told: all of them the first
    /// time. The shell has the panels of `view` after this.
    pub fn changes<B>(&mut self, view: &ViewModel<B>) -> Vec<Panel> {
        let now = panels(view);
        let changed = match &self.had {
            None => now.clone(),
            Some(had) => now
                .iter()
                .zip(had)
                .filter(|(panel, had)| panel != had)
                .map(|(panel, _)| panel.clone())
                .collect(),
        };
        self.had = Some(now);
        changed
    }
}

/// The panels of `view`: all of it but the rows of the outline.
fn panels<B>(view: &ViewModel<B>) -> Vec<Panel> {
    let outline = &view.outline;
    vec![
        Panel::Screen(view.screen.clone()),
        Panel::AtlasCards(view.atlas_cards.clone()),
        Panel::AtlasCallouts(view.atlas_callouts.clone()),
        Panel::AtlasLinks(view.atlas_links.clone()),
        Panel::AtlasLinkSuggestions(view.atlas_link_suggestions.clone()),
        Panel::Outline(OutlinePanel {
            selection_mode: outline.selection_mode,
            focus_request: outline.focus_request.clone(),
            focus_order: outline.focus_order.clone(),
            focused_item_id: outline.focused_item_id.clone(),
            key_relay_holding: outline.key_relay_holding,
            wiki: outline.wiki.clone(),
            scrolled_from_top: outline.scrolled_from_top,
        }),
        Panel::IndexPages(IndexPagesPanel {
            header: view.index_pages.header.clone(),
            path: view.index_pages.path.clone(),
            sort: view.index_pages.sort,
            parent_path: view.index_pages.parent_path.clone(),
            current_page_id: view.index_pages.current_page_id.clone(),
            state: view.index_pages.state,
        }),
        Panel::Headings(HeadingsPanel {
            state: view.headings.state,
        }),
        Panel::RecentPages(RecentPagesPanel {
            state: view.recent_pages.state,
        }),
        Panel::Search(view.search.clone()),
        Panel::Config(view.config.clone()),
        Panel::Connection(view.connection),
        Panel::Today(view.today.clone()),
        Panel::Calendar(view.calendar.clone()),
        Panel::Title(view.title.clone()),
        Panel::History(view.history.clone()),
    ]
}
