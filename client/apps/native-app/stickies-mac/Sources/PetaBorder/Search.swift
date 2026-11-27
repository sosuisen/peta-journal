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

    /// The user chose an entry of what was found. `index` is its place in the results.
    public func pick(_ index: UInt64) {
        send(.searchResultClicked(position: index))
    }
}
