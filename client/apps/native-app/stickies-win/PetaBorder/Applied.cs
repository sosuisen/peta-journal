using PetaTypes;

namespace PetaBorder;

/// <summary>
/// The view model that the changes of the core make (ADR 076). The core sends only what
/// changed; this keeps the whole of it for the views to read.
/// </summary>
internal static class Applied
{
    /// <summary>
    /// The view model of the first changes of a core. They hold every panel and a reset of
    /// the rows.
    /// </summary>
    public static ViewModel First(Changes changes)
    {
        var view = new ViewModel { Outline = new OutlineView { Rows = [] } };
        To(view, changes);
        return view;
    }

    /// <summary>Puts the changes into the view model.</summary>
    public static void To(ViewModel view, Changes changes)
    {
        foreach (var panel in changes.Panels)
            Put(view, panel);
        var rows = view.Outline.Rows;
        ListChanges.Apply(rows, changes.Rows, row => row, (at, row) => rows[at] = row);
    }

    private static void Put(ViewModel view, Panel panel)
    {
        switch (panel)
        {
            case Panel.Screen screen:
                view.Screen = screen.Value;
                break;
            case Panel.AtlasCards cards:
                view.AtlasCards = cards.Value;
                break;
            case Panel.AtlasCallouts callouts:
                view.AtlasCallouts = callouts.Value;
                break;
            case Panel.AtlasLinks links:
                view.AtlasLinks = links.Value;
                break;
            case Panel.AtlasLinkSuggestions suggestions:
                view.AtlasLinkSuggestions = suggestions.Value;
                break;
            case Panel.Outline { Value: var outline }:
                view.Outline.SelectionMode = outline.SelectionMode;
                view.Outline.FocusRequest = outline.FocusRequest;
                view.Outline.FocusOrder = outline.FocusOrder;
                view.Outline.FocusedItemId = outline.FocusedItemId;
                view.Outline.KeyRelayHolding = outline.KeyRelayHolding;
                view.Outline.Wiki = outline.Wiki;
                view.Outline.ScrolledFromTop = outline.ScrolledFromTop;
                break;
            case Panel.Sidebar sidebar:
                view.Sidebar = sidebar.Value;
                break;
            case Panel.Search search:
                view.Search = search.Value;
                break;
            case Panel.Config config:
                view.Config = config.Value;
                break;
            case Panel.Connection connection:
                view.Connection = connection.Value;
                break;
            case Panel.Today today:
                view.Today = today.Value;
                break;
            case Panel.Calendar calendar:
                view.Calendar = calendar.Value;
                break;
            case Panel.Title title:
                view.Title = title.Value;
                break;
            case Panel.History history:
                view.History = history.Value;
                break;
        }
    }
}
