using System.Collections.ObjectModel;
using PetaTypes;

namespace PetaBorder.Tests;

/// <summary>
/// The view model that the changes of the core make (ADR 076). That the core sends the changes
/// that make its own view model is tested in Rust, in <c>client-native-bridge</c>. No journal
/// server is needed.
/// </summary>
public sealed class AppliedTests
{
    private static Row Row(string id) => new() { Item = new Item { Id = id } };

    private static Changes Changes(IEnumerable<RowChange> rows, IEnumerable<Panel> panels) =>
        new()
        {
            Rows = [.. rows],
            Headings = [],
            IndexPages = [],
            RecentPages = [],
            Panels = [.. panels],
        };

    private static RowChange Reset(params string[] ids) =>
        new RowChange.Reset(new ObservableCollection<Row>(ids.Select(Row)));

    private static SidebarPage Page(string id) =>
        new()
        {
            Page = new Page
            {
                Id = id,
                ParentId = "",
                DirPath = "",
                Name = id,
                PageType = "doc",
                Properties = [],
            },
            Label = id,
            Path = id,
        };

    private static SearchEntry Result(string key, string snippet) =>
        new()
        {
            Key = key,
            Kind = SearchKind.Item,
            Title = "t",
            Label = "t",
            Snippet = snippet,
        };

    private static Heading Heading(string id, string text) =>
        new()
        {
            Id = id,
            Text = text,
            Level = 1,
        };

    private static List<string> Ids(IEnumerable<Row> rows) => [.. rows.Select(row => row.Item.Id)];

    /// <summary>The outline panel, with <c>SelectionMode</c> to tell one from another.</summary>
    private static OutlinePanel Outline(bool selecting) =>
        new()
        {
            SelectionMode = selecting,
            Wiki = new WikiView { Suggestions = [] },
        };

    private static SearchView Search(string query) => new() { Query = query, Results = [] };

    /// <summary>
    /// Every panel once, as the first changes of a core hold them. Each panel has a value of its
    /// own, so a panel put in the place of another is seen.
    /// </summary>
    private static List<Panel> EveryPanel() =>
        [
            new Panel.Screen(new ScreenView.Settings()),
            new Panel.AtlasCards([]),
            new Panel.AtlasCallouts([]),
            new Panel.AtlasLinks([]),
            new Panel.AtlasLinkSuggestions(["suggested"]),
            new Panel.Outline(Outline(selecting: true)),
            new Panel.IndexPages(
                new IndexPagesPanel
                {
                    Header = "index",
                    CurrentPageId = "current",
                    State = LoadState.Loaded,
                }
            ),
            new Panel.Headings(new HeadingsPanel { State = LoadState.Failed }),
            new Panel.RecentPages(new RecentPagesPanel { State = LoadState.Loading }),
            new Panel.Search(Search("first")),
            new Panel.Config(new ConfigView { TagsDir = "tags" }),
            new Panel.Connection(ConnectionState.Connected),
            new Panel.Today("2026-11-22"),
            new Panel.Calendar(new CalendarView { MonthName = "November" }),
            new Panel.Title("the title"),
            new Panel.History(new HistoryView { CanGoBack = true, Url = "/pages/back" }),
        ];

    // 最初の差分から、すべての欄が、それぞれのパネルの値で作られる。行は、入れ替えで届いた行である。
    [Fact]
    public void The_first_changes_make_every_part_from_its_panel()
    {
        var view = Applied.First(Changes([Reset("a")], EveryPanel()));

        Assert.IsType<ScreenView.Settings>(view.Screen);
        Assert.Equal(["suggested"], view.AtlasLinkSuggestions);
        Assert.True(view.Outline.SelectionMode);
        Assert.Equal("current", view.IndexPages.CurrentPageId);
        Assert.Equal(LoadState.Failed, view.Headings.State);
        Assert.Equal(LoadState.Loading, view.RecentPages.State);
        Assert.Equal("first", view.Search.Query);
        Assert.Equal("tags", view.Config.TagsDir);
        Assert.Equal(ConnectionState.Connected, view.Connection);
        Assert.Equal("2026-11-22", view.Today);
        Assert.Equal("November", view.Calendar.MonthName);
        Assert.Equal("the title", view.Title);
        Assert.Equal("/pages/back", view.History.Url);
        Assert.Equal(["a"], Ids(view.Outline.Rows));
    }

    // 最初の差分に、欠けたパネルや二度あるパネルがあれば、ビューモデルは作られない。境界の両側の版が違うときである。
    [Fact]
    public void The_first_changes_without_every_panel_once_make_no_view_model()
    {
        var missing = EveryPanel().Where(panel => panel is not Panel.Title);
        var twice = EveryPanel().Append(new Panel.Title("again"));
        // As many panels as there should be
        var twiceInPlaceOfAnother = missing.Append(new Panel.Today("again"));

        Assert.Throws<InvalidOperationException>(() => Applied.First(Changes([], missing)));
        Assert.Throws<InvalidOperationException>(() => Applied.First(Changes([], twice)));
        Assert.Throws<InvalidOperationException>(() =>
            Applied.First(Changes([], twiceInPlaceOfAnother))
        );
    }

    // 届いたパネルだけが入れ替わり、ほかの欄は前のままである。
    [Fact]
    public void Only_the_panels_that_came_take_the_place_of_their_parts()
    {
        var view = Applied.First(Changes([], EveryPanel()));

        Applied.To(
            view,
            Changes([], [new Panel.Search(Search("changed")), new Panel.Title("another")])
        );

        Assert.Equal("changed", view.Search.Query);
        Assert.Equal("another", view.Title);
        Assert.Equal("tags", view.Config.TagsDir);
        Assert.Equal("/pages/back", view.History.Url);
    }

    // 後の差分の見出しの操作は、今の見出しに当たる。見出しの状態は前のままである。
    [Fact]
    public void The_headings_of_a_later_change_are_applied_to_the_headings_there()
    {
        var view = Applied.First(
            new Changes
            {
                Rows = [],
                Headings = [new HeadingChange.Reset([Heading("a", "One"), Heading("b", "Two")])],
                IndexPages = [],
                RecentPages = [],
                Panels = [.. EveryPanel()],
            }
        );

        Applied.To(
            view,
            new Changes
            {
                Rows = [],
                Headings = [new HeadingChange.Update(1, Heading("b", "Three"))],
                IndexPages = [],
                RecentPages = [],
                Panels = [],
            }
        );

        Assert.Equal(["One", "Three"], view.Headings.Headings.Select(heading => heading.Text));
        Assert.Equal(LoadState.Failed, view.Headings.State);
    }

    // 後の差分のページの操作は、今のフォルダのページと最近のページに当たる。それぞれのパネルの値は前のままである。
    [Fact]
    public void The_pages_of_a_later_change_are_applied_to_the_lists_there()
    {
        var view = Applied.First(
            new Changes
            {
                Rows = [],
                Headings = [],
                IndexPages = [new PageChange.Reset([Page("a"), Page("b")])],
                RecentPages = [new PageChange.Reset([Page("c")])],
                Panels = [.. EveryPanel()],
            }
        );

        Applied.To(
            view,
            new Changes
            {
                Rows = [],
                Headings = [],
                IndexPages = [new PageChange.Remove(0)],
                RecentPages = [new PageChange.Insert(0, Page("d"))],
                Panels = [],
            }
        );

        Assert.Equal(["b"], view.IndexPages.Pages.Select(page => page.Label));
        Assert.Equal("current", view.IndexPages.CurrentPageId);
        Assert.Equal(["d", "c"], view.RecentPages.Pages.Select(page => page.Label));
        Assert.Equal(LoadState.Loading, view.RecentPages.State);
    }

    // 後の差分の検索の結果の操作は、今の結果に当たる。検索のパネルの値は前のままである。
    [Fact]
    public void The_search_results_of_a_later_change_are_applied_to_the_results_there()
    {
        var first = Changes([], EveryPanel());
        first.SearchResults = [new SearchChange.Reset([Result("item:a", "one"), Result("item:b", "two")])];
        var view = Applied.First(first);
        var later = Changes([], []);
        later.SearchResults = [new SearchChange.Update(1, Result("item:b", "three"))];

        Applied.To(view, later);

        Assert.Equal(["one", "three"], view.Search.Results.Select(result => result.Snippet));
        Assert.Equal("first", view.Search.Query);
    }

    // アウトラインのパネルは行を持たないので、届いても行は前のままである。
    [Fact]
    public void The_outline_panel_keeps_the_rows()
    {
        var view = Applied.First(Changes([Reset("a", "b")], EveryPanel()));

        Applied.To(view, Changes([], [new Panel.Outline(Outline(selecting: false))]));

        Assert.False(view.Outline.SelectionMode);
        Assert.Equal(["a", "b"], Ids(view.Outline.Rows));
    }
}
