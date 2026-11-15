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
    public void Leaving_the_body_by_more_than_10px_starts_a_selection_of_the_pressed_item(double y)
    {
        drag.Pressed("a", top: 100, bottom: 130);

        drag.Moved(y, itemUnder: "a");

        Assert.Equal([new Event.DragBeyondBounds("a")], sent);
    }

    // 選択中に別の項目へ入ると、選択がそこまで伸びる。同じ項目へは2度送らない。
    [Fact]
    public void Entering_another_item_during_the_selection_extends_it_there_once()
    {
        drag.Pressed("a", top: 100, bottom: 130);
        drag.Moved(141, itemUnder: "b");
        sent.Clear();

        drag.Moved(150, itemUnder: "b");
        drag.Moved(180, itemUnder: "c");
        drag.Moved(185, itemUnder: "c");

        Assert.Equal([new Event.ItemMouseEnter("c")], sent);
    }

    // ボタンを離すと、ドラッグが終わる。そのあとの動きでは、何も送らない。
    [Fact]
    public void Releasing_the_button_ends_the_drag_and_later_moves_send_nothing()
    {
        drag.Pressed("a", top: 100, bottom: 130);
        drag.Moved(141, itemUnder: "b");
        sent.Clear();

        drag.Released();
        drag.Moved(180, itemUnder: "c");

        Assert.Equal([new Event.SelectionMouseUp()], sent);
    }
}
