using System.Text.Json;

namespace PetaBorder;

/// <summary>What is kept of one sticky between two runs of the app.</summary>
public sealed record StickySettings
{
    public const string Yellow = "#FFF8D0";

    public required string Id { get; init; }

    /// <summary>What the sticky shows, as the URL that opens it. <c>null</c> when it shows no page.</summary>
    public string? Url { get; init; }

    /// <summary>
    /// The top left corner of the window, in the pixels of the screens as Windows counts them
    /// over all screens.
    /// </summary>
    public required int X { get; init; }
    public required int Y { get; init; }

    /// <summary>
    /// The size of the window, in pixels as they are at a scale of 100%: a sticky is as large
    /// on a screen with another scale.
    /// </summary>
    public required double Width { get; init; }
    public required double Height { get; init; }

    /// <summary>The color of the paper, as <c>#RRGGBB</c>.</summary>
    public string Color { get; init; } = Yellow;

    /// <summary>The window stays above the windows of other apps.</summary>
    public bool AlwaysOnTop { get; init; }

    /// <summary>How large the page is shown: 1 is the usual size.</summary>
    public double Zoom { get; init; } = 1;

    /// <summary>
    /// Only the title bar of the window is shown. The size is still that of the whole sticky.
    /// </summary>
    public bool RolledUp { get; init; }
}

/// <summary>
/// The stickies as a file keeps them. Every change is written at once: the app may be ended at
/// any time.
/// </summary>
public sealed class StickyStore
{
    // A file of an older version of the app lacks what was added since: such a value is the
    // one of a new sticky
    private static readonly JsonSerializerOptions Format = new(JsonSerializerDefaults.Web)
    {
        WriteIndented = true,
    };

    private readonly List<StickySettings> stickies;
    private readonly string file;

    /// <summary>The file of the app, in the folder of the user for the data of apps.</summary>
    public static string StandardFile =>
        Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            "PetaStickies",
            "stickies.json"
        );

    /// <summary>
    /// Reads the file. A file that is not there, or cannot be read, gives no stickies.
    /// </summary>
    public StickyStore(string file)
    {
        ArgumentNullException.ThrowIfNull(file);
        this.file = file;
        stickies = Read(file);
    }

    public IReadOnlyList<StickySettings> Stickies => stickies;

    /// <summary>Adds the sticky, or changes the one with its id.</summary>
    public void Put(StickySettings sticky)
    {
        ArgumentNullException.ThrowIfNull(sticky);
        var place = stickies.FindIndex(kept => kept.Id == sticky.Id);
        if (place < 0)
            stickies.Add(sticky);
        else if (stickies[place] == sticky)
            return;
        else
            stickies[place] = sticky;
        Write();
    }

    public void Remove(string id)
    {
        stickies.RemoveAll(kept => kept.Id == id);
        Write();
    }

    private static List<StickySettings> Read(string file)
    {
        try
        {
            return JsonSerializer.Deserialize<List<StickySettings>>(File.ReadAllText(file), Format)
                ?? [];
        }
        catch (Exception e) when (e is IOException or JsonException or UnauthorizedAccessException)
        {
            return [];
        }
    }

    /// <summary>
    /// A failure is not told: the stickies on screen go on, and the next change writes again.
    /// </summary>
    private void Write()
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(file))!);
            // Written beside the file and then put in its place: the app may be ended in the
            // middle of the writing, and half a file would lose all the stickies
            var beside = file + ".new";
            File.WriteAllText(beside, JsonSerializer.Serialize(stickies, Format));
            File.Move(beside, file, overwrite: true);
        }
        catch (Exception e) when (e is IOException or UnauthorizedAccessException) { }
    }
}
