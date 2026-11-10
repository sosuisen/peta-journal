using Stickies.Services;

namespace Stickies.Tests;

public class PageLinksTests
{
    private const string Server = "http://localhost:8087";

    // ウィキリンクは、Webアプリのページに通じる。
    [Fact]
    public void A_wikilink_leads_to_a_page_of_the_web_app()
    {
        var uri = PageLinks.ForWikilink(Server, "Quarkus");

        Assert.Equal("http://localhost:8087/pages/Quarkus", uri.AbsoluteUri);
    }

    // ページ名のフォルダの区切りは、アドレスでも区切りのままである。
    [Fact]
    public void The_folders_of_a_page_name_stay_folders_in_the_address()
    {
        var uri = PageLinks.ForWikilink(Server, "KCG/2026/syllabus");

        Assert.Equal("http://localhost:8087/pages/KCG/2026/syllabus", uri.AbsoluteUri);
    }

    // ページ名の空白、#、日本語は、アドレスの意味を変えないようにエスケープされる。
    [Fact]
    public void Spaces_hash_signs_and_Japanese_in_a_page_name_are_escaped()
    {
        var uri = PageLinks.ForWikilink(Server, "C#/0.2 メモ");

        Assert.Equal(
            "http://localhost:8087/pages/C%23/0.2%20%E3%83%A1%E3%83%A2",
            uri.AbsoluteUri);
    }

    // ハッシュタグは、タグ用フォルダにあるタグのページに通じる。
    [Fact]
    public void A_hashtag_leads_to_a_page_in_the_folder_of_the_tag_pages()
    {
        var uri = PageLinks.ForHashtag(Server, "tags/", "DailyReport/PixiJS");

        Assert.Equal("http://localhost:8087/pages/tags/DailyReport/PixiJS", uri.AbsoluteUri);
    }

    // タグ用フォルダがページのルートのとき、ハッシュタグはルートのページに通じる。
    [Fact]
    public void A_hashtag_leads_to_a_page_in_the_root_when_the_tag_folder_is_the_root()
    {
        var uri = PageLinks.ForHashtag(Server, "", "topic");

        Assert.Equal("http://localhost:8087/pages/topic", uri.AbsoluteUri);
    }

    // ページからの相対パスで書かれた画像は、ページのアドレスを基準にすると、サーバーのassetsに通じる。
    [Fact]
    public void An_image_path_relative_to_a_page_leads_to_the_assets_of_the_server()
    {
        var pages = new Uri(PageLinks.PagesUrl(Server));

        var image = new Uri(pages, "../assets/20260225055930_BSPKWC7.png");

        Assert.Equal("http://localhost:8087/assets/20260225055930_BSPKWC7.png", image.AbsoluteUri);
    }
}
