namespace PetaBorder.Tests;

/// <summary>
/// How a sticky looks. The rules of the colors, the zoom and the file are tested in Rust, in
/// <c>client-stickies</c>.
/// </summary>
public sealed class StickyLookTests
{
    // 見た目が変わると、知らされる。同じ値を入れても、知らされない。
    [Fact]
    public void A_change_of_the_look_is_told()
    {
        var look = new StickyLook();
        var told = 0;
        look.Changed += () => told++;

        look.Color = "#FFFFFF";
        look.Color = "#FFFFFF";
        look.AlwaysOnTop = true;
        look.Searching = true;
        look.ZoomIn();
        look.ResetZoom();
        look.ResetZoom();

        Assert.Equal(5, told);
    }
}
