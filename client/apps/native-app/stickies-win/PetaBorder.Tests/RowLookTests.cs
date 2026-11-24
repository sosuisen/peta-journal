using PetaTypes;

namespace PetaBorder.Tests;

/// <summary>How a row is drawn, as far as the view model decides it.</summary>
public sealed class RowLookTests
{
    // 編集中の項目の1行目は、本文全体が1行の見出しのときだけ、見出しの大きさになる。編集していない項目は、見出しの大きさで描く。
    [Fact]
    public void An_edited_item_is_large_only_when_it_is_one_heading_line()
    {
        var oneHeadingLine = new Row
        {
            HeadingLevel = 1,
            HeadingLine = true,
            IsEditing = true,
        };
        var headingAndMore = new Row
        {
            HeadingLevel = 1,
            HeadingLine = false,
            IsEditing = true,
        };
        var shown = new Row { HeadingLevel = 1, HeadingLine = false };

        Assert.Equal(1.6, RowLook.FirstLineScale(oneHeadingLine));
        Assert.Equal(1, RowLook.FirstLineScale(headingAndMore));
        Assert.Equal(1.6, RowLook.FirstLineScale(shown));
    }

    // 行頭の点は、コアが出すと言う項目にだけ出る。編集中でない空の項目には出ない。ほかのシェルと同じである（#1042）。
    [Theory]
    [InlineData(true, true)]
    [InlineData(false, false)]
    public void The_bullet_shows_only_when_the_core_says_so(bool showBullet, bool shown)
    {
        var row = new Row { ShowBullet = showBullet };

        Assert.Equal(shown, RowLook.ShowsBullet(row, bare: false));
    }
}
