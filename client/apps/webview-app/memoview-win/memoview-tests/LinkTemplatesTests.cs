using MemoView.Services;

namespace MemoView.Tests;

public class LinkTemplatesTests
{
    private const string Server = "http://localhost:8087";

    private static LinkTemplates Templates() =>
        LinkTemplates.Of(
            Server,
            wikilinkHref: "/pages/{}",
            hashtagHref: "/pages/tags/{}",
            pagesBase: "/pages/",
            wikilinkFragmentPattern: "[0-9]{14}_[0-9A-Z]{7}"
        );

    // ウィキリンクとハッシュタグのテンプレートは、サーバーの設定の前にサーバーのアドレスを付けたものである。
    [Fact]
    public void The_templates_of_the_links_are_those_of_the_server_at_its_address()
    {
        var templates = Templates();

        Assert.Equal("http://localhost:8087/pages/{}", templates.WikilinkHref);
        Assert.Equal("http://localhost:8087/pages/tags/{}", templates.HashtagHref);
    }

    // ページの基準アドレスは、サーバーのアドレスとページの置き場所である。
    [Fact]
    public void The_pages_url_is_the_base_of_the_pages_at_the_server()
    {
        Assert.Equal("http://localhost:8087/pages/", Templates().PagesUrl);
    }

    // 項目のidの形は、サーバーの設定のままである。
    [Fact]
    public void The_pattern_of_an_item_id_is_that_of_the_server()
    {
        Assert.Equal("[0-9]{14}_[0-9A-Z]{7}", Templates().WikilinkFragmentPattern);
    }
}
