namespace PetaBorder.Tests;

/// <summary>
/// How a sticky looks. The rules of the colors, the zoom and the file are tested in Rust, in
/// <c>client-stickies</c>.
/// </summary>
public sealed class StickyLookTests
{
    // 見た目が変わると、知らされる。同じ値を入れても、知らされない。
    [Fact]
    public void A_change_of_the_look_is_told()
    {
        var look = new StickyLook();
        var told = 0;
        look.Changed += () => told++;

        look.Color = "#FFFFFF";
        look.Color = "#FFFFFF";
        look.AlwaysOnTop = true;
        look.Searching = true;
        look.ZoomIn();
        look.ResetZoom();
        look.ResetZoom();

        Assert.Equal(5, told);
    }

    // 残しておいた色で開いた付箋紙は、その色になる。色がなければ、黄色になる。
    [Fact]
    public void A_sticky_opens_in_the_color_that_was_kept_or_in_yellow()
    {
        Assert.Equal("#ABCDEF", new StickyLook("#ABCDEF").Color);
        Assert.Equal(PetaCore.PetaNative.StickyYellow(), new StickyLook().Color);
    }

    // 色のメニューは、9色のあとに透明を出す。
    [Fact]
    public void The_menu_offers_transparent_after_the_colors()
    {
        Assert.Equal(PetaCore.PetaNative.StickyColors().Length + 1, StickyLook.Colors.Count);
        Assert.Equal(("Transparent", "#FFFFFF00"), StickyLook.Colors[^1]);
    }

    // 透明な付箋紙は、フォーカスがないときだけ、書かれたこと以外を隠す。色のある付箋紙は、何も隠さない。
    [Fact]
    public void Only_a_transparent_sticky_without_the_focus_is_bare()
    {
        var look = new StickyLook(StickyLook.Colors[^1].Hex);
        Assert.True(look.Transparent);
        Assert.True(look.Bare(focused: false));
        Assert.False(look.Bare(focused: true));

        look.Color = PetaCore.PetaNative.StickyYellow();
        Assert.False(look.Transparent);
        Assert.False(look.Bare(focused: false));
    }

    // 縮小は、拡大と逆の向きに大きさを変える。
    [Fact]
    public void Zooming_out_makes_the_page_smaller()
    {
        var look = new StickyLook();

        look.ZoomOut();

        Assert.True(look.Zoom < 1);
    }
}

/// <summary>The file that keeps the stickies for the next run of the app.</summary>
public sealed class StickyStoreTests
{
    // アプリは、付箋紙を、アプリのデータのフォルダのPetaStickies\stickies.jsonに残す。場所が変わると、残した付箋紙が開かない。
    [Fact]
    public void The_stickies_are_kept_in_the_folder_of_the_data_of_apps()
    {
        Assert.Equal(
            Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
                "PetaStickies",
                "stickies.json"
            ),
            StickyStore.StandardFile
        );
    }

    // ファイルに残した付箋紙は、次にアプリを起動したときに、そのまま戻る。
    [Fact]
    public void A_kept_sticky_comes_back_in_the_next_run()
    {
        var folder = Path.Combine(Path.GetTempPath(), $"peta-sticky-store-test-{Guid.NewGuid()}");
        var file = Path.Combine(folder, "stickies.json");
        var sticky = new PetaCore.PetaSticky(
            "sticky-1",
            "/pages/sample",
            10,
            20,
            300,
            400,
            "#ABCDEF",
            true,
            1.2,
            false
        );
        try
        {
            using (var store = new StickyStore(file))
                store.Put(sticky);

            using var next = new StickyStore(file);
            Assert.Equal([sticky], next.Stickies);
        }
        finally
        {
            Directory.Delete(folder, recursive: true);
        }
    }
}
