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

    // 残しておいた色で開いた付箋紙は、その色になる。色がなければ、黄色になる。
    [Fact]
    public void A_sticky_opens_in_the_color_that_was_kept_or_in_yellow()
    {
        Assert.Equal("#ABCDEF", new StickyLook("#ABCDEF").Color);
        Assert.Equal(PetaCore.PetaNative.StickyYellow(), new StickyLook().Color);
    }

    // 縮小は、拡大と逆の向きに大きさを変える。
    [Fact]
    public void Zooming_out_makes_the_page_smaller()
    {
        var look = new StickyLook();

        look.ZoomOut();

        Assert.True(look.Zoom < 1);
    }
}
