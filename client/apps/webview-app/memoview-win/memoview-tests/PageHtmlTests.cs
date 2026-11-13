using MemoView.Services;

namespace MemoView.Tests;

public class PageHtmlTests
{
    private const string Pages = "http://localhost:8087/pages/";

    private static readonly PageAssets Assets = new(
        Css: "body { color: #333; }",
        RendererScript: "var FmAst2Html = {};",
        PageScript: "console.log('page');"
    );

    private static readonly PageLook Look = new("#FFFDD0", ZoomFactor: 1.0, Scrolls: true);

    private static string BodyTag(string html)
    {
        var start = html.IndexOf("<body", StringComparison.Ordinal);
        return html[start..(html.IndexOf('>', start) + 1)];
    }

    // --- The document of a page ---

    // ページの文書は、項目の行、スタイルシート、2つのスクリプトを持つ。
    [Fact]
    public void The_document_of_a_page_holds_the_rows_the_style_sheet_and_the_two_scripts()
    {
        var html = PageHtml.ForItems("<div>row</div>", Look, Pages, Assets);

        Assert.Contains("<div>row</div>", html);
        Assert.Contains(Assets.Css, html);
        Assert.Contains(Assets.RendererScript, html);
        Assert.Contains(Assets.PageScript, html);
    }

    // ページのスクリプトは、レンダラーのスクリプトと項目の行のあとに置かれる。
    // ページのスクリプトが、行を探してレンダラーを呼ぶためである。
    [Fact]
    public void The_page_script_comes_after_the_rows_and_the_renderer_script()
    {
        var html = PageHtml.ForItems("<div>row</div>", Look, Pages, Assets);

        var rows = html.IndexOf("<div>row</div>", StringComparison.Ordinal);
        var renderer = html.IndexOf(Assets.RendererScript, StringComparison.Ordinal);
        var page = html.IndexOf(Assets.PageScript, StringComparison.Ordinal);
        Assert.True(rows < page, "The page script is before the rows.");
        Assert.True(renderer < page, "The page script is before the renderer script.");
    }

    // 背景色は、bodyに付く。
    [Fact]
    public void The_background_color_is_on_the_body()
    {
        var html = PageHtml.ForItems("", Look with { BackgroundColor = "#D0F0FF" }, Pages, Assets);

        Assert.Contains("background:#D0F0FF;", BodyTag(html));
    }

    // アクティブでない付箋紙は、スクロールバーを出さない。
    [Theory]
    [InlineData(true, "overflow:auto;")]
    [InlineData(false, "overflow:hidden;")]
    public void A_sticky_that_is_not_active_shows_no_scroll_bars(bool scrolls, string expected)
    {
        var html = PageHtml.ForItems("", Look with { Scrolls = scrolls }, Pages, Assets);

        Assert.Contains(expected, BodyTag(html));
    }

    // ズームの倍率は、表示とスクリプトの両方に渡る。小数点は、OSの言語によらずピリオドである。
    [Fact]
    public void The_zoom_factor_goes_to_the_look_and_to_the_script_with_a_period_in_any_language()
    {
        var culture = Thread.CurrentThread.CurrentCulture;
        Thread.CurrentThread.CurrentCulture = new System.Globalization.CultureInfo("de-DE");
        try
        {
            var html = PageHtml.ForItems("", Look with { ZoomFactor = 1.2 }, Pages, Assets);

            Assert.Contains("zoom:1.2;", BodyTag(html));
            Assert.Contains("data-zoom=\"1.2\"", BodyTag(html));
        }
        finally
        {
            Thread.CurrentThread.CurrentCulture = culture;
        }
    }

    // ページの基準アドレスは、スクリプトに渡る。
    [Fact]
    public void The_address_of_the_pages_goes_to_the_script()
    {
        var html = PageHtml.ForItems("", Look, Pages, Assets);

        Assert.Contains($"data-pages=\"{Pages}\"", BodyTag(html));
    }

    // --- The document of a message ---

    // メッセージの文書は、文と、その色と背景色を持つ。
    [Fact]
    public void The_document_of_a_message_holds_the_text_with_its_color_and_the_background()
    {
        var html = PageHtml.ForMessage("Failed to load page", "#cc3333", "#FFFDD0");

        Assert.Contains("<p>Failed to load page</p>", html);
        Assert.Contains("color:#cc3333;", BodyTag(html));
        Assert.Contains("background:#FFFDD0;", BodyTag(html));
    }

    // メッセージの文は、HTMLではなく文字として出る。
    [Fact]
    public void The_text_of_a_message_is_shown_as_text_and_not_as_HTML()
    {
        var html = PageHtml.ForMessage("<b>a & b</b>", "#999", "#FFFDD0");

        Assert.Contains("<p>&lt;b&gt;a &amp; b&lt;/b&gt;</p>", html);
    }
}
