namespace PetaBorder.Tests;

/// <summary>The file that keeps the stickies between two runs of the app.</summary>
public sealed class StickyStoreTests : IDisposable
{
    private readonly string folder = Path.Combine(
        Path.GetTempPath(),
        $"peta-stickies-{Guid.NewGuid()}"
    );

    private string File => Path.Combine(folder, "kept", "stickies.json");

    public void Dispose()
    {
        if (Directory.Exists(folder))
            Directory.Delete(folder, recursive: true);
    }

    private static StickySettings Sticky(string id, string? url = null) =>
        new()
        {
            Id = id,
            Url = url,
            X = 10,
            Y = 20,
            Width = 480,
            Height = 600,
        };

    /// <summary>Writes a file as an older version of the app wrote it.</summary>
    private void Keep(string json)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(File)!);
        System.IO.File.WriteAllText(File, json);
    }

    // ファイルがまだないときは、付箋紙は1つもない。
    [Fact]
    public void There_are_no_stickies_before_anything_is_kept()
    {
        Assert.Empty(new StickyStore(File).Stickies);
    }

    // 足した付箋紙は、読み直しても残る。順番も同じである。
    [Fact]
    public void Stickies_that_were_put_are_there_after_reading()
    {
        var store = new StickyStore(File);
        store.Put(Sticky("a", "/pages/notes"));
        store.Put(Sticky("b"));

        Assert.Equal([Sticky("a", "/pages/notes"), Sticky("b")], new StickyStore(File).Stickies);
    }

    // 同じ付箋紙をもう一度入れると、増えずに、中身が変わる。
    [Fact]
    public void A_sticky_that_is_put_again_is_changed_and_not_added()
    {
        var store = new StickyStore(File);
        store.Put(Sticky("a", "/pages/notes"));
        store.Put(Sticky("b"));

        store.Put(Sticky("a", "/today") with { Width = 300, RolledUp = true });

        Assert.Equal(
            [Sticky("a", "/today") with { Width = 300, RolledUp = true }, Sticky("b")],
            new StickyStore(File).Stickies
        );
    }

    // 消した付箋紙は、読み直しても出てこない。ほかの付箋紙は残る。
    [Fact]
    public void A_sticky_that_is_removed_is_not_there_after_reading()
    {
        var store = new StickyStore(File);
        store.Put(Sticky("a"));
        store.Put(Sticky("b"));

        store.Remove("a");

        Assert.Equal([Sticky("b")], new StickyStore(File).Stickies);
    }

    // 読めないファイルは、付箋紙がないものとして扱う。次に入れた付箋紙は、保存される。
    [Fact]
    public void A_file_that_cannot_be_read_counts_as_no_stickies()
    {
        Keep("not JSON");
        var store = new StickyStore(File);
        Assert.Empty(store.Stickies);

        store.Put(Sticky("a"));

        Assert.Equal([Sticky("a")], new StickyStore(File).Stickies);
    }

    // 色、常に手前、ズーム、巻き上げは、保存される。
    [Fact]
    public void The_look_of_a_sticky_is_kept()
    {
        var sticky = Sticky("a") with
        {
            Color = "#D0E8FF",
            AlwaysOnTop = true,
            Zoom = 1.5,
            RolledUp = true,
        };
        new StickyStore(File).Put(sticky);

        Assert.Equal([sticky], new StickyStore(File).Stickies);
    }

    // 古いファイルの付箋紙には、あとから足した設定がない。黄色で、手前に表示せず、等倍で、巻き上げていない。
    [Fact]
    public void A_sticky_of_an_older_file_looks_like_a_new_one()
    {
        Keep("""[{"id": "a", "x": 10, "y": 20, "width": 480, "height": 600}]""");

        var kept = Assert.Single(new StickyStore(File).Stickies);

        Assert.Equal(Sticky("a"), kept);
        Assert.Equal(StickySettings.Yellow, kept.Color);
        Assert.False(kept.AlwaysOnTop);
        Assert.Equal(1, kept.Zoom);
        Assert.False(kept.RolledUp);
    }
}

/// <summary>How a sticky looks.</summary>
public sealed class StickyLookTests
{
    // ズームは、1割ずつ変わる。0.1を足し続けても、端数は出ない。
    [Fact]
    public void The_zoom_changes_by_a_tenth()
    {
        var look = new StickyLook();

        look.ZoomIn();
        look.ZoomIn();
        look.ZoomIn();
        Assert.Equal(1.3, look.Zoom);
        look.ZoomOut();
        Assert.Equal(1.2, look.Zoom);
        look.ResetZoom();
        Assert.Equal(1, look.Zoom);
    }

    // ズームは、30%から300%の間に収まる。保存されていた値も同じである。
    [Fact]
    public void The_zoom_stays_between_three_tenths_and_three()
    {
        var small = new StickyLook(zoom: 0.3);
        small.ZoomOut();
        Assert.Equal(0.3, small.Zoom);

        var large = new StickyLook(zoom: 3);
        large.ZoomIn();
        Assert.Equal(3, large.Zoom);

        Assert.Equal(3, new StickyLook(zoom: 50).Zoom);
    }

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

    // `#RRGGBB`の色が読める。読めない色は、新しい付箋紙の黄色になる。
    [Fact]
    public void A_color_is_read_and_anything_else_is_yellow()
    {
        Assert.Equal(((byte)0xD0, (byte)0xE8, (byte)0xFF), StickyLook.Rgb("#D0E8FF"));
        Assert.Equal(((byte)0xFF, (byte)0xFD, (byte)0xD0), StickyLook.Rgb("blue"));
        Assert.Equal(((byte)0xFF, (byte)0xFD, (byte)0xD0), StickyLook.Rgb("#12345"));
    }
}
