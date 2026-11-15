using PetaTypes;

namespace PetaBorder;

/// <summary>
/// A drag of the mouse that starts on the text of an item. Inside the text it selects letters.
/// Once it leaves the text up or down, it selects items, as in the web frontend.
/// </summary>
/// <param name="send">Sends an event to the core.</param>
public sealed class ItemDrag(Action<Event> send)
{
    /// <summary>The item the mouse went down on, with the top and the bottom of its text.</summary>
    private (string ItemId, double Top, double Bottom)? pressed;

    /// <summary>
    /// The item the selection was last told to reach. <c>null</c> while no items are selected.
    /// </summary>
    private string? reached;

    /// <summary>The mouse went down on the text of the item, from <paramref name="top"/> to
    /// <paramref name="bottom"/>.</summary>
    public void Pressed(string itemId, double top, double bottom)
    {
        ArgumentNullException.ThrowIfNull(itemId);
        pressed = (itemId, top, bottom);
    }

    /// <summary>The mouse moved to <paramref name="y"/>, over <paramref name="itemUnder"/>.</summary>
    /// <param name="itemUnder">The item under the mouse. <c>null</c> when there is none.</param>
    public void Moved(double y, string? itemUnder)
    {
        if (pressed is not { } on)
            return;
        if (reached is null)
        {
            if (y >= on.Top - 10 && y <= on.Bottom + 10)
                return;
            send(new Event.DragBeyondBounds(on.ItemId));
            reached = on.ItemId;
        }
        if (itemUnder is null || itemUnder == reached)
            return;
        send(new Event.ItemMouseEnter(itemUnder));
        reached = itemUnder;
    }

    /// <summary>The mouse button came up: the drag is over.</summary>
    public void Released()
    {
        if (reached is not null)
            send(new Event.SelectionMouseUp());
        pressed = null;
        reached = null;
    }
}
