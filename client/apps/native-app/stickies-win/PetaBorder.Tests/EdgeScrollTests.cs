namespace PetaBorder.Tests;

public sealed class EdgeScrollTests
{
    // 付箋の上端か下端から40pxの帯では、深いほど速くスクロールする。端と、その外では1回に24pxである。帯の外ではスクロールしない。
    [Theory]
    [InlineData(150, 0)]
    [InlineData(140, 0)]
    [InlineData(120, -12)]
    [InlineData(100, -24)]
    [InlineData(80, -24)]
    [InlineData(460, 0)]
    [InlineData(480, 12)]
    [InlineData(500, 24)]
    [InlineData(520, 24)]
    public void The_list_scrolls_faster_the_deeper_the_mouse_is_in_the_band_at_an_edge(
        double y,
        double step
    )
    {
        Assert.Equal(step, EdgeScroll.Step(y, top: 100, bottom: 500));
    }
}
