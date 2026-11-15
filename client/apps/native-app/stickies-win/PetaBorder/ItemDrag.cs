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
        if (y < on.Top - 10 || y > on.Bottom + 10)
            send(new Event.DragBeyondBounds(on.ItemId));
    }
}
