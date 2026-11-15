import PetaTypes

/// A drag of the mouse that starts on the text of an item. Inside the text it selects letters.
/// Once it leaves the text up or down, it selects items, as in the web frontend. The places
/// are in the list, from its top down.
@MainActor
public final class ItemDrag {
    private let send: (Event) -> Void

    /// `send` sends an event to the core.
    public init(send: @escaping (Event) -> Void) {
        self.send = send
    }

    /// Items are selected: the drag left the text.
    public private(set) var selects = false

    /// The mouse went down on the text of the item, from `top` to `bottom`.
    public func pressed(_ itemId: String, top: Double, bottom: Double) {}

    /// The mouse moved to `y`, over `itemUnder`, or over no item.
    public func moved(y: Double, itemUnder: String?) {}
}
