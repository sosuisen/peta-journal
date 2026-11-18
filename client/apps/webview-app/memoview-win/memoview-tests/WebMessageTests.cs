using System.Reflection;
using MemoView.Services;

namespace MemoView.Tests;

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

    // ウィキリンクやハッシュタグのクリックは、そのページのURLを持つメッセージになる。
    [Fact]
    public void A_click_on_a_link_to_a_page_is_a_message_with_its_url()
    {
        var message = WebMessage.Parse("""{"openPage":"http://localhost:8087/pages/C%23#20260426035700_QZ998PX"}""");

        Assert.Equal(
            new WebMessage.PageLinkClicked(new Uri("http://localhost:8087/pages/C%23#20260426035700_QZ998PX")),
            message
        );
    }

    // アプリが知らない内容は、メッセージにならない。
    [Theory]
    [InlineData("""{"unknown":1}""")]
    [InlineData("""{"zoom":"big"}""")]
    [InlineData("""{"toggleCollapse":null}""")]
    [InlineData("""{"openPage":""}""")]
    [InlineData("""{"openPage":7}""")]
    [InlineData("""{"openPage":"C#"}""")]
    [InlineData("""{"openPage":"file:///C:/Windows/notepad.exe"}""")]
    [InlineData("\"text\"")]
    [InlineData("not json")]
    public void A_text_the_app_does_not_know_is_not_a_message(string json)
    {
        Assert.Null(WebMessage.Parse(json));
    }

    // ページのスクリプトは、アプリが読む3つの名前でメッセージを送る。
    [Theory]
    [InlineData("zoom")]
    [InlineData("toggleCollapse")]
    [InlineData("openPage")]
    public void The_page_script_sends_its_messages_under_the_names_the_app_reads(string name)
    {
        using var stream = Assembly.GetExecutingAssembly().GetManifestResourceStream("page.js")!;
        using var reader = new StreamReader(stream);
        var script = reader.ReadToEnd();

        Assert.Contains($"{{{name}:", script);
    }
}
