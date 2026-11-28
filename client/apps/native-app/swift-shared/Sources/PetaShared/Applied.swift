@preconcurrency import PetaTypes

// The view model that the changes of the core make (ADR 076). The core sends only what changed;
// the shell keeps the whole of it for the screen to read.

extension ViewModel {
    /// The view model of the first changes of a core. They hold every panel and a reset of the
    /// rows, the headings, the pages of the sidebar and the results of the search.
    public init(first changes: Changes) {
        let panels = changes.panels
        func the<T>(_ value: (Panel) -> T?) -> T {
            let found = panels.compactMap(value)
            precondition(found.count == 1, "the first changes have each panel once")
            return found[0]
        }
        let outline = the { if case .outline(let it) = $0 { it } else { nil } }
        self.init(
            screen: the { if case .screen(let it) = $0 { it } else { nil } },
            atlasCards: the { if case .atlasCards(let it) = $0 { it } else { nil } },
            atlasCallouts: the { if case .atlasCallouts(let it) = $0 { it } else { nil } },
            atlasLinks: the { if case .atlasLinks(let it) = $0 { it } else { nil } },
            atlasUnplaced: the { if case .atlasUnplaced(let it) = $0 { it } else { nil } },
            outline: OutlineView(outline, rows: [Row]().applying(changes.rows)),
            indexPages: IndexPagesView(
                the { if case .indexPages(let it) = $0 { it } else { nil } },
                pages: [SidebarPage]().after(changes.indexPages.map(\.change))),
            headings: HeadingsView(
                headings: [Heading]().after(changes.headings.map(\.change)),
                state: the { if case .headings(let it) = $0 { it } else { nil } }.state),
            recentPages: RecentPagesView(
                pages: [SidebarPage]().after(changes.recentPages.map(\.change)),
                state: the { if case .recentPages(let it) = $0 { it } else { nil } }.state),
            search: SearchView(
                the { if case .search(let it) = $0 { it } else { nil } },
                results: [SearchEntry]().after(changes.searchResults.map(\.change))),
            config: the { if case .config(let it) = $0 { it } else { nil } },
            connection: the { if case .connection(let it) = $0 { it } else { nil } },
            today: the { if case .today(let it) = $0 { it } else { nil } },
            calendar: the { if case .calendar(let it) = $0 { it } else { nil } },
            title: the { if case .title(let it) = $0 { it } else { nil } },
            history: the { if case .history(let it) = $0 { it } else { nil } })
    }

    /// Applies the changes: each panel that came takes the place of its part, and the changes
    /// to the rows, the headings, the pages and the results of the search are applied in order.
    public mutating func apply(_ changes: Changes) {
        for panel in changes.panels { apply(panel) }
        if !changes.rows.isEmpty { outline.rows = outline.rows.applying(changes.rows) }
        if !changes.headings.isEmpty { headings.headings = headings.headings.after(changes.headings.map(\.change)) }
        if !changes.indexPages.isEmpty { indexPages.pages = indexPages.pages.after(changes.indexPages.map(\.change)) }
        if !changes.recentPages.isEmpty {
            recentPages.pages = recentPages.pages.after(changes.recentPages.map(\.change))
        }
        if !changes.searchResults.isEmpty {
            search.results = search.results.after(changes.searchResults.map(\.change))
        }
    }

    private mutating func apply(_ panel: Panel) {
        switch panel {
        case .screen(let value): screen = value
        case .atlasCards(let value): atlasCards = value
        case .atlasCallouts(let value): atlasCallouts = value
        case .atlasLinks(let value): atlasLinks = value
        case .atlasUnplaced(let value): atlasUnplaced = value
        case .outline(let value): outline = OutlineView(value, rows: outline.rows)
        case .indexPages(let value): indexPages = IndexPagesView(value, pages: indexPages.pages)
        case .headings(let value): headings.state = value.state
        case .recentPages(let value): recentPages.state = value.state
        case .search(let value): search = SearchView(value, results: search.results)
        case .config(let value): config = value
        case .connection(let value): connection = value
        case .today(let value): today = value
        case .calendar(let value): calendar = value
        case .title(let value): title = value
        case .history(let value): history = value
        }
    }
}

extension Changes {
    /// True when nothing changed: no change to any list, and no panel. A shell may skip these
    /// changes; any others it must apply, or its lists go apart from those of the core.
    public var isEmpty: Bool {
        rows.isEmpty && headings.isEmpty && indexPages.isEmpty && recentPages.isEmpty && searchResults.isEmpty
            && panels.isEmpty
    }
}

extension Array where Element == Row {
    /// The rows after the changes, applied one after another. A place counts in the rows as the
    /// changes before it left them.
    public func applying(_ changes: [RowChange]) -> [Row] {
        after(changes.map(\.change))
    }
}

/// A change of a list of any kind (ADR 088). The border has a type of its own for the changes of
/// each list, all of the same shape; each is made into this, and `after` applies them all.
public enum Change<Element> {
    case insert(at: Int, item: Element)
    case remove(at: Int)
    case move(from: Int, to: Int)
    case update(at: Int, item: Element)
    case reset(items: [Element])
}

extension Array {
    /// The list after the changes, applied one after another. A place counts in the list as the
    /// changes before it left it.
    public func after(_ changes: [Change<Element>]) -> [Element] {
        var items = self
        for change in changes {
            switch change {
            case .insert(let at, let item): items.insert(item, at: at)
            case .remove(let at): items.remove(at: at)
            case .move(let from, let to): items.insert(items.remove(at: from), at: to)
            case .update(let at, let item): items[at] = item
            case .reset(let all): items = all
            }
        }
        return items
    }
}

extension RowChange {
    fileprivate var change: Change<Row> {
        switch self {
        case .insert(let at, let item): .insert(at: Int(at), item: item)
        case .remove(let at): .remove(at: Int(at))
        case .move(let from, let to): .move(from: Int(from), to: Int(to))
        case .update(let at, let item): .update(at: Int(at), item: item)
        case .reset(let items): .reset(items: items)
        }
    }
}

extension HeadingChange {
    fileprivate var change: Change<Heading> {
        switch self {
        case .insert(let at, let item): .insert(at: Int(at), item: item)
        case .remove(let at): .remove(at: Int(at))
        case .move(let from, let to): .move(from: Int(from), to: Int(to))
        case .update(let at, let item): .update(at: Int(at), item: item)
        case .reset(let items): .reset(items: items)
        }
    }
}

extension PageChange {
    fileprivate var change: Change<SidebarPage> {
        switch self {
        case .insert(let at, let item): .insert(at: Int(at), item: item)
        case .remove(let at): .remove(at: Int(at))
        case .move(let from, let to): .move(from: Int(from), to: Int(to))
        case .update(let at, let item): .update(at: Int(at), item: item)
        case .reset(let items): .reset(items: items)
        }
    }
}

extension SearchChange {
    fileprivate var change: Change<SearchEntry> {
        switch self {
        case .insert(let at, let item): .insert(at: Int(at), item: item)
        case .remove(let at): .remove(at: Int(at))
        case .move(let from, let to): .move(from: Int(from), to: Int(to))
        case .update(let at, let item): .update(at: Int(at), item: item)
        case .reset(let items): .reset(items: items)
        }
    }
}

extension SearchView {
    fileprivate init(_ panel: SearchPanel, results: [SearchEntry]) {
        self.init(
            query: panel.query, results: results, showDropdown: panel.showDropdown,
            selectedIndex: panel.selectedIndex, searching: panel.searching)
    }
}

extension IndexPagesView {
    fileprivate init(_ panel: IndexPagesPanel, pages: [SidebarPage]) {
        self.init(
            header: panel.header, path: panel.path, sort: panel.sort, parentPath: panel.parentPath, pages: pages,
            currentPageId: panel.currentPageId, state: panel.state)
    }
}

extension OutlineView {
    fileprivate init(_ panel: OutlinePanel, rows: [Row]) {
        self.init(
            rows: rows, selectionMode: panel.selectionMode, focusRequest: panel.focusRequest,
            focusOrder: panel.focusOrder, focusedItemId: panel.focusedItemId,
            keyRelayHolding: panel.keyRelayHolding, wiki: panel.wiki, scrolledFromTop: panel.scrolledFromTop)
    }
}
