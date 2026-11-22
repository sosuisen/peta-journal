//! What changed on the screen since a shell was last told (ADR 076): the changes to the rows
//! of the outline, and each panel that changed, whole.

use crate::diff::ListChange;
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
pub struct Shown {}

impl Shown {
    /// What changed from what the shell has to `view`. The shell has `view` after this.
    pub fn changes(&mut self, _view: ViewModel<RichText>) -> Changes {
        todo!()
    }
}
