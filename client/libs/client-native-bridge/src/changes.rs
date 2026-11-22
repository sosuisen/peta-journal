//! What changed on the screen since a shell was last told (ADR 076): the changes to the rows
//! of the outline, and each panel that changed, whole.

use crate::diff::{list_changes, ListChange};
use client_core::app::ConnectionState;
use client_core::outline::{FocusOrder, FocusRequest};
use client_core::screens::atlas::{Callout, LinkCard};
use client_core::screens::calendar::CalendarView;
use client_core::view::{
    AtlasCard, ConfigView, HistoryView, Row, ScreenView, SearchView, SidebarView, WikiView,
};
use client_core::{RichText, ViewModel};
use serde::Serialize;

/// What changed since the shell was last told.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Changes {
    /// The changes to the rows of the outline, to apply in order.
    pub rows: Vec<ListChange<Row<RichText>>>,
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
    Sidebar(SidebarView),
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
#[derive(Default)]
pub struct Shown {
    /// The rows and the panels the shell has. None before it was first told.
    had: Option<(Vec<Row<RichText>>, Vec<Panel>)>,
}

impl Shown {
    /// What changed from what the shell has to `view`. The shell has `view` after this.
    pub fn changes(&mut self, view: ViewModel<RichText>) -> Changes {
        let (rows, panels) = split(view);
        let changes = match &self.had {
            None => Changes {
                rows: vec![ListChange::Reset {
                    items: rows.clone(),
                }],
                panels: panels.clone(),
            },
            Some((had_rows, had_panels)) => Changes {
                rows: list_changes(had_rows, &rows, |row| &row.item.id),
                panels: panels
                    .iter()
                    .zip(had_panels)
                    .filter(|(panel, had)| panel != had)
                    .map(|(panel, _)| panel.clone())
                    .collect(),
            },
        };
        self.had = Some((rows, panels));
        changes
    }
}

/// The rows of the outline, and the panels of the rest of `view`.
fn split(view: ViewModel<RichText>) -> (Vec<Row<RichText>>, Vec<Panel>) {
    let outline = view.outline;
    let panels = vec![
        Panel::Screen(view.screen),
        Panel::AtlasCards(view.atlas_cards),
        Panel::AtlasCallouts(view.atlas_callouts),
        Panel::AtlasLinks(view.atlas_links),
        Panel::AtlasLinkSuggestions(view.atlas_link_suggestions),
        Panel::Outline(OutlinePanel {
            selection_mode: outline.selection_mode,
            focus_request: outline.focus_request,
            focus_order: outline.focus_order,
            focused_item_id: outline.focused_item_id,
            key_relay_holding: outline.key_relay_holding,
            wiki: outline.wiki,
            scrolled_from_top: outline.scrolled_from_top,
        }),
        Panel::Sidebar(view.sidebar),
        Panel::Search(view.search),
        Panel::Config(view.config),
        Panel::Connection(view.connection),
        Panel::Today(view.today),
        Panel::Calendar(view.calendar),
        Panel::Title(view.title),
        Panel::History(view.history),
    ];
    (outline.rows, panels)
}
