using PetaTypes;

namespace PetaBorder.Tests;

/// <summary>How a row is drawn, as far as the view model decides it.</summary>
public sealed class RowLookTests
{
    // 編集欄の文字の倍率は、本文全体が1行の見出しのときだけ見出しの倍率である。コアがまだ編集中と数えていなくても同じである（#795）。
    [Fact]
    public void The_editor_is_large_only_when_the_body_is_one_heading_line()
    {
        var oneHeadingLine = new Row { HeadingLine = true, FirstLineScale = 1.4f };
        var headingAndMore = new Row { HeadingLine = false, FirstLineScale = 1.4f };
        var edited = new Row
        {
            HeadingLine = false,
            IsEditing = true,
            FirstLineScale = 1,
        };

        Assert.Equal(1.4, RowLook.EditedScale(oneHeadingLine), 5);
        Assert.Equal(1, RowLook.EditedScale(headingAndMore));
        Assert.Equal(1, RowLook.EditedScale(edited));
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

    // 文字だけを見せる付箋紙では、コアが出すと言う項目にも、行頭の点を出さない。
    [Fact]
    public void A_bare_sticky_shows_no_bullet()
    {
        var row = new Row { ShowBullet = true };

        Assert.False(RowLook.ShowsBullet(row, bare: true));
    }
}
