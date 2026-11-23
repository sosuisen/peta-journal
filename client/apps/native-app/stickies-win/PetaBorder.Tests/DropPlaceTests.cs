namespace PetaBorder.Tests;

public sealed class DropPlaceTests
{
    /// <summary>The places of "abc" on one line, 10 wide each, 16 high.</summary>
    private static readonly Caret[] OneLine =
    [
        new(0, 0, 16),
        new(10, 0, 16),
        new(20, 0, 16),
        new(30, 0, 16),
    ];

    // 落とした点の位置は、同じ行で最も近い文字の境目である。
    [Theory]
    [InlineData(12, 1)]
    [InlineData(17, 2)]
    public void A_drop_goes_to_the_nearest_place_on_its_line(double x, int place)
    {
        Assert.Equal(place, DropPlace.Nearest(OneLine, x, 8));
    }
}
