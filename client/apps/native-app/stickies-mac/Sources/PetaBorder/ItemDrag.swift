import PetaTypes

/// A drag of the mouse that starts on the text of an item. Inside the text it selects letters.
/// Once it leaves the text up or down, it selects items, as in the web frontend. The places
/// are in the list, from its top down.
@MainActor
public final class ItemDrag {
    private let send: (Event) -> Void
    /// The item the mouse went down on, with the top and the bottom of its text.
    private var pressed: (itemId: String, top: Double, bottom: Double)?

    /// `send` sends an event to the core.
    public init(send: @escaping (Event) -> Void) {
        self.send = send
    }

    /// The item the selection was last told to reach. `nil` while no items are selected.
    private var reached: String?

    /// Items are selected: the drag left the text.
    public var selects: Bool {
        reached != nil
    }

    /// The mouse went down on the text of the item, from `top` to `bottom`.
    public func pressed(_ itemId: String, top: Double, bottom: Double) {
        pressed = (itemId, top, bottom)
    }

    /// The mouse moved to `y`, over `itemUnder`, or over no item.
    public func moved(y: Double, itemUnder: String?) {
        guard let on = pressed else { return }
        if reached == nil {
            guard y < on.top - 10 || y > on.bottom + 10 else { return }
            // The edit ends first, which saves what was typed, as for Shift and an arrow
            send(.editFinished(itemId: on.itemId))
            send(.dragBeyondBounds(itemId: on.itemId))
            reached = on.itemId
        }
        guard let itemUnder, itemUnder != reached else { return }
        send(.itemMouseEnter(itemId: itemUnder))
        reached = itemUnder
    }

    /// The mouse button came up: the drag is over.
    public func released() {
        if reached != nil { send(.selectionMouseUp) }
        pressed = nil
        reached = nil
    }
}
