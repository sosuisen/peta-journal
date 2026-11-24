namespace PetaBorder.Tests;

public sealed class AreaTests
{
    /// <summary>A screen of 1000 by 800 pixels, right of a screen that starts at 0.</summary>
    private static readonly Area Screen = new(1000, 0, 1000, 800);

    // スクリーンに収まる付箋紙は、その位置のまま置かれる。
    [Fact]
    public void An_area_inside_the_screen_stays_where_it_is()
    {
        var sticky = new Area(1100, 100, 300, 400);

        Assert.Equal(sticky, sticky.Within(Screen));
    }
}
