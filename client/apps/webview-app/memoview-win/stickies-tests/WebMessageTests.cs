using System.Reflection;
using Stickies.Services;

namespace Stickies.Tests;

public class WebMessageTests
{
    // ズームの変更は、倍率を持つメッセージになる。
    [Fact]
    public void A_zoom_change_is_a_message_with_the_factor()
    {
        var message = WebMessage.Parse("""{"zoom":1.2}""");

        Assert.Equal(new WebMessage.ZoomChanged(1.2), message);
    }

    // 折りたたみの矢印のクリックは、項目のIDを持つメッセージになる。
    [Fact]
    public void A_click_on_a_collapse_arrow_is_a_message_with_the_item_id()
    {
        var message = WebMessage.Parse("""{"toggleCollapse":"2026-11-10-14-28-03-2STDJG0"}""");

        Assert.Equal(new WebMessage.CollapseToggled("2026-11-10-14-28-03-2STDJG0"), message);
    }

    // ウィキリンクのクリックは、ページ名を持つメッセージになる。
    [Fact]
    public void A_click_on_a_wikilink_is_a_message_with_the_page_name()
    {
        var message = WebMessage.Parse("""{"openWikilink":"KCG/プロジェクト演習/2026"}""");

        Assert.Equal(new WebMessage.WikilinkClicked("KCG/プロジェクト演習/2026"), message);
    }

    // ハッシュタグのクリックは、タグ名を持つメッセージになる。
    [Fact]
    public void A_click_on_a_hashtag_is_a_message_with_the_tag()
    {
        var message = WebMessage.Parse("""{"openHashtag":"DailyReport/PixiJS"}""");

        Assert.Equal(new WebMessage.HashtagClicked("DailyReport/PixiJS"), message);
    }

    // アプリが知らない内容は、メッセージにならない。
    [Theory]
    [InlineData("""{"unknown":1}""")]
    [InlineData("""{"zoom":"big"}""")]
    [InlineData("""{"toggleCollapse":null}""")]
    [InlineData("""{"openWikilink":""}""")]
    [InlineData("""{"openHashtag":7}""")]
    [InlineData("\"text\"")]
    [InlineData("not json")]
    public void A_text_the_app_does_not_know_is_not_a_message(string json)
    {
        Assert.Null(WebMessage.Parse(json));
    }

    // ページのスクリプトは、アプリが読む4つの名前でメッセージを送る。
    [Theory]
    [InlineData("zoom")]
    [InlineData("toggleCollapse")]
    [InlineData("openWikilink")]
    [InlineData("openHashtag")]
    public void The_page_script_sends_its_messages_under_the_names_the_app_reads(string name)
    {
        using var stream = Assembly.GetExecutingAssembly().GetManifestResourceStream("page.js")!;
        using var reader = new StreamReader(stream);
        var script = reader.ReadToEnd();

        Assert.Contains($"{{{name}:", script);
    }
}
