using Stickies.Services;

namespace Stickies.Tests;

public sealed class SettingsServiceTests : IDisposable
{
    private readonly string _path = Path.Combine(Path.GetTempPath(), $"peta-stickies-test-{Guid.NewGuid()}");

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
}
