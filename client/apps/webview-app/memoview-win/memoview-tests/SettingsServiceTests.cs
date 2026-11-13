using MemoView.Services;

namespace MemoView.Tests;

public sealed class SettingsServiceTests : IDisposable
{
    private readonly string _path = Path.Combine(
        Path.GetTempPath(),
        $"peta-stickies-test-{Guid.NewGuid()}"
    );

    public void Dispose()
    {
        File.Delete(_path);
    }

    // A second service reads the file, as the next start of the app does.
    private SettingsService Reloaded()
    {
        var settings = new SettingsService(_path);
        settings.Load();
        return settings;
    }

    // 付箋紙を足すと、設定を読み直しても残る。
    [Fact]
    public void A_sticky_that_was_added_is_there_after_the_settings_are_read_again()
    {
        var settings = new SettingsService(_path);

        var sticky = settings.AddWindow();

        Assert.Equal([sticky.Id], Reloaded().Windows.Select(w => w.Id));
    }

    // 付箋紙を消すと、設定を読み直しても出てこない。ほかの付箋紙は残る。
    [Fact]
    public void A_sticky_that_was_removed_is_gone_after_the_settings_are_read_again_and_the_others_stay()
    {
        var settings = new SettingsService(_path);
        var closed = settings.AddWindow();
        var open = settings.AddWindow();

        settings.RemoveWindow(closed.Id);

        Assert.Equal([open.Id], Reloaded().Windows.Select(w => w.Id));
    }

    // 消した付箋紙の設定を保存しても、付箋紙は戻らない。
    // 閉じる途中のウィンドウが、位置やサイズの変化を知らせてくることがあるためである。
    [Fact]
    public void Saving_the_settings_of_a_removed_sticky_does_not_bring_it_back()
    {
        var settings = new SettingsService(_path);
        var closed = settings.AddWindow();
        settings.RemoveWindow(closed.Id);

        closed.X = 300;
        settings.UpdateWindow(closed);

        Assert.Empty(Reloaded().Windows);
    }
}
