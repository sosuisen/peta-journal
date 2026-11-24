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

    /// <summary>
    /// The places of "ab\ncd": "ab" and the line break on the first line, "cd" on the second.
    /// The second line ends at 20, where the first line has its break.
    /// </summary>
    private static readonly Caret[] TwoLines =
    [
        new(0, 0, 16),
        new(10, 0, 16),
        new(20, 0, 16),
        new(0, 16, 32),
        new(10, 16, 32),
        new(20, 16, 32),
    ];

    // 行の右に落とすと、その行の末尾になる。下の行の末尾は、同じ横の位置でも選ばれない。
    [Fact]
    public void A_drop_right_of_a_line_goes_to_its_end()
    {
        Assert.Equal(2, DropPlace.Nearest(TwoLines, 200, 8));
    }

    // 文字のない所（行の上や下）に落とすと、最も近い行の中で決まる。
    [Theory]
    [InlineData(-30, 1)]
    [InlineData(100, 4)]
    public void A_drop_off_the_lines_goes_to_the_nearest_line(double y, int place)
    {
        Assert.Equal(place, DropPlace.Nearest(TwoLines, 12, y));
    }
}
