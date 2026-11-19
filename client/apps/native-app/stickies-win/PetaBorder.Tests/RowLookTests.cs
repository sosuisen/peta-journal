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
}
