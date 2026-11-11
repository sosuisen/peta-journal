import Foundation
import PetaTypes

/// The search box. What it found is in `view.search`: the core asks the server a moment after
/// the last key.
extension Shell {
    /// The user changed the text of the search box.
    public func search(_ query: String) {
        send(.searchQueryChanged(query))
    }

    /// The user pressed a key in the search box that goes through what was found.
    public func search(key: SearchKey) {
        send(.searchKey(key))
    }

    /// The user chose an entry of what was found. `index` is the one the entry carries.
    public func pick(_ index: UInt64) {
        send(.searchResultClicked(index: index))
    }
}

/// How an entry of what was found reads on screen.
extension SearchEntry {
    /// The core opens the journal of today for the name "today", and makes no page of it. To
    /// the core it is still a page that is not there, so it comes as an entry that creates.
    private var isToday: Bool {
        kind == .create && title == "today"
    }

    /// The first line: the page, or what choosing the entry does.
    public var label: String {
        kind == .create && !isToday ? "Create \"\(title)\"" : title
    }

    /// The second line, or nothing. The snippet of a page found by its name is the name again.
    public var hint: String {
        if isToday { return "The journal of today" }
        return snippet == title ? "" : snippet
    }
}
