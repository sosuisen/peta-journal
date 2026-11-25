//! What changed on the screen since a shell was last told (ADR 076): the changes to the rows
//! of the outline, and each panel that changed, whole. A shell of any body model takes them
//! (ADR 078).

mod diff;

pub use diff::{list_changes, ListChange};

use crate::app::ConnectionState;
use crate::outline::{FocusOrder, FocusRequest};
use crate::screens::atlas::{Callout, LinkCard};
use crate::screens::calendar::CalendarView;
use crate::view::{
    AtlasCard, ConfigView, HeadingsView, HistoryView, IndexPagesView, RecentPagesView, Row,
    ScreenView, SearchView, WikiView,
};
use crate::ViewModel;
use serde::Serialize;

/// What changed since the shell was last told.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Changes<B> {
    /// The changes to the rows of the outline, to apply in order.
    pub rows: Vec<ListChange<Row<B>>>,
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
    IndexPages(IndexPagesView),
    Headings(HeadingsView),
    RecentPages(RecentPagesView),
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

/// What the shell was last told: the rows and the panels it has now.
pub struct Shown<B> {
    /// The rows the shell has. None before it was first told.
    rows: Option<Vec<Row<B>>>,
    panels: ShownPanels,
}

// Not derived: the derive would ask `B: Default`, which no body model needs to be
impl<B> Default for Shown<B> {
    fn default() -> Shown<B> {
        Shown {
            rows: None,
            panels: ShownPanels::default(),
        }
    }
}

impl<B: Clone + PartialEq> Shown<B> {
    /// What changed from what the shell has to `view`. The shell has `view` after this.
    pub fn changes(&mut self, view: ViewModel<B>) -> Changes<B> {
        let panels = self.panels.changes(&view);
        let rows = view.outline.rows;
        let changes = match &self.rows {
            None => vec![ListChange::Reset {
                items: rows.clone(),
            }],
            Some(had) => list_changes(had, &rows, |row| &row.item.id),
        };
        self.rows = Some(rows);
        Changes {
            rows: changes,
            panels,
        }
    }
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
        Panel::IndexPages(view.index_pages.clone()),
        Panel::Headings(view.headings.clone()),
        Panel::RecentPages(view.recent_pages.clone()),
        Panel::Search(view.search.clone()),
        Panel::Config(view.config.clone()),
        Panel::Connection(view.connection),
        Panel::Today(view.today.clone()),
        Panel::Calendar(view.calendar.clone()),
        Panel::Title(view.title.clone()),
        Panel::History(view.history.clone()),
    ]
}
