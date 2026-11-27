import PetaTypes
import XCTest

@testable import PetaShared

/// The view model that the changes of the core make (ADR 076). That the core sends the changes
/// that make its own view model is tested in Rust, in `client-native-bridge`.
final class AppliedTests: XCTestCase {
    private func row(_ id: String, _ body: String = "") -> Row {
        Row(
            item: Item(
                id: id, pageId: "page", dirPath: "", name: "", itemType: "", createdDate: 0, modifiedDate: 0,
                idx: "", level: 1, body: body, collapsed: false, types: [], properties: [:], transcludeItemId: nil),
            bodyModel: RichText(blocks: []), position: 0, indentPx: 0, isEditing: false, isSelected: false,
            isCollapsed: false, hasChildren: false, headingLevel: 0, isHeading: false, headingLine: false,
            firstLineScale: 1, task: nil, hasOwnZoom: false, isTransclusion: false, isBrokenTransclusion: false,
            showBullet: true,
            effectiveZoom: 1, dateHeader: nil, suggestionsVisible: false)
    }

    /// The changes, each list without a change unless one is given.
    private func changes(
        rows: [RowChange] = [], headings: [HeadingChange] = [], indexPages: [PageChange] = [],
        recentPages: [PageChange] = [], searchResults: [SearchChange] = [], panels: [Panel] = []
    ) -> Changes {
        Changes(
            rows: rows, headings: headings, indexPages: indexPages, recentPages: recentPages,
            searchResults: searchResults, panels: panels)
    }

    private func ids(_ rows: [Row]) -> [String] {
        rows.map(\.item.id)
    }

    /// The outline panel, with `wiki` to tell one from another.
    private func outline(selecting: Bool) -> OutlinePanel {
        OutlinePanel(
            selectionMode: selecting, focusRequest: nil, focusOrder: nil, focusedItemId: nil, keyRelayHolding: false,
            wiki: WikiView(itemId: nil, suggestions: [], selectedIndex: 0, x: 0, y: 0), scrolledFromTop: false)
    }

    private func search(_ query: String) -> SearchPanel {
        SearchPanel(query: query, showDropdown: false, selectedIndex: nil, searching: false)
    }

    /// Every panel once, as the first changes of a core hold them. Each panel has a value of
    /// its own, so a panel put in the place of another is seen.
    private func everyPanel() -> [Panel] {
        [
            .screen(.settings),
            .atlasCards([]),
            .atlasCallouts([]),
            .atlasLinks([]),
            .atlasLinkSuggestions(["suggested"]),
            .outline(outline(selecting: true)),
            .indexPages(
                IndexPagesPanel(
                    header: "index", path: "index", sort: IndexSort(key: .name, order: .asc), parentPath: nil,
                    currentPageId: "current", state: .loaded)),
            .headings(HeadingsPanel(state: .failed)),
            .recentPages(RecentPagesPanel(state: .loading)),
            .search(search("first")),
            .config(
                ConfigView(
                    markdownDir: "markdown", weekNumbering: .iso, logicalOutdent: true, tagsDir: "tags",
                    homeLink: "home", propertyDefinitions: [], definitionError: nil, definitionsAdded: 0)),
            .connection(.connected),
            .today("2026-11-22"),
            .calendar(
                CalendarView(
                    year: 2026, month: 11, monthName: "November", monthPage: "", yearPage: "", dayLabels: [],
                    weeks: [])),
            .title("the title"),
            .history(HistoryView(canGoBack: true, canGoForward: false, url: "/pages/back")),
        ]
    }

    // 最初の差分から、すべての欄が、それぞれのパネルの値で作られる。行は、入れ替えで届いた行である。
    func testTheFirstChangesMakeEveryPartFromItsPanel() {
        let view = ViewModel(first: changes(rows: [.reset(items: [row("a")])], panels: everyPanel()))

        XCTAssertEqual(view.screen, .settings)
        XCTAssertEqual(view.atlasLinkSuggestions, ["suggested"])
        XCTAssertEqual(view.outline.selectionMode, true)
        XCTAssertEqual(view.indexPages.currentPageId, "current")
        XCTAssertEqual(view.headings.state, .failed)
        XCTAssertEqual(view.recentPages.state, .loading)
        XCTAssertEqual(view.search.query, "first")
        XCTAssertEqual(view.config.tagsDir, "tags")
        XCTAssertEqual(view.connection, .connected)
        XCTAssertEqual(view.today, "2026-11-22")
        XCTAssertEqual(view.calendar.monthName, "November")
        XCTAssertEqual(view.title, "the title")
        XCTAssertEqual(view.history.url, "/pages/back")
        XCTAssertEqual(ids(view.outline.rows), ["a"])
    }

    // 届いたパネルだけが入れ替わり、ほかの欄は前のままである。
    func testOnlyThePanelsThatCameTakeThePlaceOfTheirParts() {
        var view = ViewModel(first: changes(panels: everyPanel()))

        view.apply(changes(panels: [.search(search("changed")), .title("another")]))

        XCTAssertEqual(view.search.query, "changed")
        XCTAssertEqual(view.title, "another")
        XCTAssertEqual(view.config.tagsDir, "tags")
        XCTAssertEqual(view.history.url, "/pages/back")
    }

    // 後の差分の行の操作は、今の行に当たる。
    func testTheRowsOfALaterChangeAreAppliedToTheRowsThere() {
        var view = ViewModel(first: changes(rows: [.reset(items: [row("a")])], panels: everyPanel()))

        view.apply(changes(rows: [.insert(at: 1, item: row("b"))]))

        XCTAssertEqual(ids(view.outline.rows), ["a", "b"])
    }

    // 後の差分の見出しの操作は、今の見出しに当たる。見出しの状態は前のままである。
    func testTheHeadingsOfALaterChangeAreAppliedToTheHeadingsThere() {
        let one = Heading(id: "a", text: "One", level: 1)
        let two = Heading(id: "b", text: "Two", level: 2)
        var view = ViewModel(first: changes(headings: [.reset(items: [one, two])], panels: everyPanel()))

        let three = Heading(id: "b", text: "Three", level: 2)
        view.apply(changes(headings: [.update(at: 1, item: three)]))

        XCTAssertEqual(view.headings.headings.map(\.text), ["One", "Three"])
        XCTAssertEqual(view.headings.state, .failed)
    }

    // 後の差分のページの操作は、今のフォルダのページと最近のページに当たる。それぞれのパネルの値は前のままである。
    func testThePagesOfALaterChangeAreAppliedToTheListsThere() {
        func page(_ id: String) -> SidebarPage {
            SidebarPage(
                page: Page(
                    id: id, parentId: "", dirPath: "", name: id, pageType: "doc", createdDate: 0, modifiedDate: 0,
                    properties: [:]),
                label: id, published: false, isDir: false, dirLabel: nil, path: id)
        }
        var view = ViewModel(
            first: changes(
                indexPages: [.reset(items: [page("a"), page("b")])], recentPages: [.reset(items: [page("c")])],
                panels: everyPanel()))

        view.apply(changes(indexPages: [.remove(at: 0)], recentPages: [.insert(at: 0, item: page("d"))]))

        XCTAssertEqual(view.indexPages.pages.map(\.label), ["b"])
        XCTAssertEqual(view.indexPages.currentPageId, "current")
        XCTAssertEqual(view.recentPages.pages.map(\.label), ["d", "c"])
        XCTAssertEqual(view.recentPages.state, .loading)
    }

    // 見出しやページの差分だけの変更も、空ではない。シェルは、空の変更だけを当てずに捨ててよい。
    func testChangesWithOnlyAListOfTheSidebarAreNotEmpty() {
        let heading = Heading(id: "a", text: "One", level: 1)

        XCTAssertTrue(changes().isEmpty)
        XCTAssertFalse(changes(headings: [.reset(items: [heading])]).isEmpty)
        XCTAssertFalse(changes(indexPages: [.remove(at: 0)]).isEmpty)
        XCTAssertFalse(changes(recentPages: [.remove(at: 0)]).isEmpty)
    }

    // 後の差分の検索の結果の操作は、今の結果に当たる。検索のパネルの値は前のままである。
    func testTheSearchResultsOfALaterChangeAreAppliedToTheResultsThere() {
        func result(_ key: String, _ snippet: String) -> SearchEntry {
            SearchEntry(key: key, kind: .item, title: "t", label: "t", snippet: snippet)
        }
        var view = ViewModel(
            first: changes(
                searchResults: [.reset(items: [result("item:a", "one"), result("item:b", "two")])],
                panels: everyPanel()))

        view.apply(changes(searchResults: [.update(at: 1, item: result("item:b", "three"))]))

        XCTAssertEqual(view.search.results.map(\.snippet), ["one", "three"])
        XCTAssertEqual(view.search.query, "first")
        XCTAssertFalse(changes(searchResults: [.remove(at: 0)]).isEmpty)
    }

    // アウトラインのパネルは行を持たないので、届いても行は前のままである。
    func testTheOutlinePanelKeepsTheRows() {
        var view = ViewModel(
            first: changes(rows: [.reset(items: [row("a"), row("b")])], panels: everyPanel()))

        view.apply(changes(panels: [.outline(outline(selecting: false))]))

        XCTAssertEqual(view.outline.selectionMode, false)
        XCTAssertEqual(ids(view.outline.rows), ["a", "b"])
    }

    // 行の操作は順に当たる。挿入、削除、移動、更新の位置は、その前の操作を当てたあとの行で数える。
    func testTheChangesAreAppliedInOrderAndEachPlaceCountsAfterTheChangesBefore() {
        let rows = [row("a"), row("b"), row("c")]

        let applied = rows.applying([
            .insert(at: 1, item: row("x")),
            .remove(at: 0),
            .move(from: 2, to: 0),
            .update(at: 1, item: row("x", "changed")),
        ])

        XCTAssertEqual(ids(applied), ["c", "x", "b"])
        XCTAssertEqual(applied[1].item.body, "changed")
    }

    // 入れ替えは、前の行をすべて捨てて、届いた行にする。
    func testAResetPutsTheRowsThatCameInPlaceOfAllBefore() {
        let rows = [row("a"), row("b")]

        let applied = rows.applying([.reset(items: [row("y"), row("z")])])

        XCTAssertEqual(ids(applied), ["y", "z"])
    }
}
