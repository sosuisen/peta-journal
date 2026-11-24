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

    // スクリーンからはみ出る付箋紙は、全体がスクリーンに収まる位置まで戻る。
    [Theory]
    [InlineData(1900, 100, 1700, 100)]
    [InlineData(1100, 700, 1100, 400)]
    [InlineData(1900, 700, 1700, 400)]
    [InlineData(900, -50, 1000, 0)]
    public void An_area_off_the_screen_comes_back_until_all_of_it_is_inside(
        int left,
        int top,
        int keptLeft,
        int keptTop
    )
    {
        var sticky = new Area(left, top, 300, 400);

        Assert.Equal(new Area(keptLeft, keptTop, 300, 400), sticky.Within(Screen));
    }
}
