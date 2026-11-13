using System.Text.Json;
using System.Text.Json.Serialization;

namespace MemoView.Services;

public class StickyWindowSettings
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = Guid.NewGuid().ToString();

    [JsonPropertyName("pageId")]
    public string? PageId { get; set; }

    [JsonPropertyName("x")]
    public int X { get; set; } = 100;

    [JsonPropertyName("y")]
    public int Y { get; set; } = 100;

    [JsonPropertyName("width")]
    public int Width { get; set; } = 500;

    [JsonPropertyName("height")]
    public int Height { get; set; } = 600;

    [JsonPropertyName("backgroundColor")]
    public string BackgroundColor { get; set; } = "#FFFDD0";

    [JsonPropertyName("zoomFactor")]
    public double ZoomFactor { get; set; } = 1.0;

    [JsonPropertyName("alwaysOnTop")]
    public bool AlwaysOnTop { get; set; } = false;
}

public class MemoViewSettings
{
    [JsonPropertyName("windows")]
    public List<StickyWindowSettings> Windows { get; set; } = new();
}

public class SettingsService
{
    private static readonly string DefaultPath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
        ".peta-stickies"
    );

    private readonly string _path;
    private MemoViewSettings _settings = new();

    /// <summary>
    /// The settings in the file of the user, <c>~/.peta-stickies</c>.
    /// </summary>
    public SettingsService()
        : this(DefaultPath) { }

    /// <summary>
    /// The settings in a file of the caller. The tests use it, so that they do not write
    /// the file of the user.
    /// </summary>
    /// <param name="path">The path of the settings file. The file does not have to exist.</param>
    public SettingsService(string path)
    {
        ArgumentException.ThrowIfNullOrEmpty(path);
        _path = path;
    }

    private static readonly JsonSerializerOptions JsonOptions = new() { WriteIndented = true };

    public IReadOnlyList<StickyWindowSettings> Windows => _settings.Windows;

    public void Load()
    {
        try
        {
            if (File.Exists(_path))
            {
                var json = File.ReadAllText(_path);
                _settings =
                    JsonSerializer.Deserialize<MemoViewSettings>(json, JsonOptions)
                    ?? new MemoViewSettings();
            }
        }
        catch
        {
            _settings = new MemoViewSettings();
        }
    }

    public void Save()
    {
        try
        {
            var json = JsonSerializer.Serialize(_settings, JsonOptions);
            File.WriteAllText(_path, json);
        }
        catch
        {
            // Silently ignore save failures
        }
    }

    public StickyWindowSettings AddWindow()
    {
        var settings = new StickyWindowSettings();
        _settings.Windows.Add(settings);
        Save();
        return settings;
    }

    /// <summary>
    /// Saves the changed settings of a sticky.
    /// A sticky that was removed is not added again: a window that is closing can still
    /// report a change of its position or size.
    /// </summary>
    public void UpdateWindow(StickyWindowSettings windowSettings)
    {
        ArgumentNullException.ThrowIfNull(windowSettings);
        var idx = _settings.Windows.FindIndex(w => w.Id == windowSettings.Id);
        if (idx < 0)
            return;
        _settings.Windows[idx] = windowSettings;
        Save();
    }

    public void RemoveWindow(string windowId)
    {
        _settings.Windows.RemoveAll(w => w.Id == windowId);
        Save();
    }
}
