using PetaTypes;

namespace PetaBorder.Tests;

public sealed class ItemDragTests
{
    private readonly List<Event> sent = [];
    private readonly ItemDrag drag;

    public ItemDragTests() => drag = new ItemDrag(sent.Add);

    // 本文の上か下へ10pxより大きく出ると、押した項目から選択が始まる。
    [Theory]
    [InlineData(89)]
    [InlineData(141)]
    public void Leaving_the_body_by_more_than_10px_starts_a_selection_of_the_pressed_item(
        double y
    )
    {
        drag.Pressed("a", top: 100, bottom: 130);

        drag.Moved(y, itemUnder: "a");

        Assert.Equal([new Event.DragBeyondBounds("a")], sent);
    }
}
