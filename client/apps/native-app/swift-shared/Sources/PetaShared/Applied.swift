@preconcurrency import PetaTypes

// The view model that the changes of the core make (ADR 076). The core sends only what changed;
// the shell keeps the whole of it for the screen to read.

extension ViewModel {
    /// The view model of the first changes of a core. They hold every panel and a reset of the
    /// rows.
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
            atlasLinkSuggestions: the { if case .atlasLinkSuggestions(let it) = $0 { it } else { nil } },
            outline: OutlineView(outline, rows: [Row]().applying(changes.rows)),
            indexPages: the { if case .indexPages(let it) = $0 { it } else { nil } },
            headings: the { if case .headings(let it) = $0 { it } else { nil } },
            recentPages: the { if case .recentPages(let it) = $0 { it } else { nil } },
            search: the { if case .search(let it) = $0 { it } else { nil } },
            config: the { if case .config(let it) = $0 { it } else { nil } },
            connection: the { if case .connection(let it) = $0 { it } else { nil } },
            today: the { if case .today(let it) = $0 { it } else { nil } },
            calendar: the { if case .calendar(let it) = $0 { it } else { nil } },
            title: the { if case .title(let it) = $0 { it } else { nil } },
            history: the { if case .history(let it) = $0 { it } else { nil } })
    }

    /// Applies the changes: each panel that came takes the place of its part, and the changes
    /// to the rows are applied in order.
    public mutating func apply(_ changes: Changes) {
        for panel in changes.panels { apply(panel) }
        if !changes.rows.isEmpty { outline.rows = outline.rows.applying(changes.rows) }
    }

    private mutating func apply(_ panel: Panel) {
        switch panel {
        case .screen(let value): screen = value
        case .atlasCards(let value): atlasCards = value
        case .atlasCallouts(let value): atlasCallouts = value
        case .atlasLinks(let value): atlasLinks = value
        case .atlasLinkSuggestions(let value): atlasLinkSuggestions = value
        case .outline(let value): outline = OutlineView(value, rows: outline.rows)
        case .indexPages(let value): indexPages = value
        case .headings(let value): headings = value
        case .recentPages(let value): recentPages = value
        case .search(let value): search = value
        case .config(let value): config = value
        case .connection(let value): connection = value
        case .today(let value): today = value
        case .calendar(let value): calendar = value
        case .title(let value): title = value
        case .history(let value): history = value
        }
    }
}

extension Array where Element == Row {
    /// The rows after the changes, applied one after another. A place counts in the rows as the
    /// changes before it left them.
    public func applying(_ changes: [ListChange]) -> [Row] {
        var rows = self
        for change in changes {
            switch change {
            case .insert(let at, let item): rows.insert(item, at: Int(at))
            case .remove(let at): rows.remove(at: Int(at))
            case .move(let from, let to): rows.insert(rows.remove(at: Int(from)), at: Int(to))
            case .update(let at, let item): rows[Int(at)] = item
            case .reset(let items): rows = items
            }
        }
        return rows
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
