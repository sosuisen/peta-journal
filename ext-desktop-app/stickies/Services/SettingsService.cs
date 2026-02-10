using System.Text.Json;
using System.Text.Json.Serialization;

namespace Stickies.Services;

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
}

public class StickiesSettings
{
    [JsonPropertyName("windows")]
    public List<StickyWindowSettings> Windows { get; set; } = new();
}

public class SettingsService
{
    private static readonly string SettingsPath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
        ".peta-stickies");

    private StickiesSettings _settings = new();
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true
    };

    public IReadOnlyList<StickyWindowSettings> Windows => _settings.Windows;

    public void Load()
    {
        try
        {
            if (File.Exists(SettingsPath))
            {
                var json = File.ReadAllText(SettingsPath);
                _settings = JsonSerializer.Deserialize<StickiesSettings>(json, JsonOptions)
                            ?? new StickiesSettings();
            }
        }
        catch
        {
            _settings = new StickiesSettings();
        }
    }

    public void Save()
    {
        try
        {
            var json = JsonSerializer.Serialize(_settings, JsonOptions);
            File.WriteAllText(SettingsPath, json);
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

    public void UpdateWindow(StickyWindowSettings windowSettings)
    {
        var existing = _settings.Windows.FirstOrDefault(w => w.Id == windowSettings.Id);
        if (existing != null)
        {
            var idx = _settings.Windows.IndexOf(existing);
            _settings.Windows[idx] = windowSettings;
        }
        else
        {
            _settings.Windows.Add(windowSettings);
        }
        Save();
    }

    public void RemoveWindow(string windowId)
    {
        _settings.Windows.RemoveAll(w => w.Id == windowId);
        Save();
    }
}
