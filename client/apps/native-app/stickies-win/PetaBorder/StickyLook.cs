using System.Globalization;

namespace PetaBorder;

/// <summary>How one sticky looks. The views of the sticky change it, and the desk keeps it.</summary>
public sealed class StickyLook
{
    private string color;
    private bool alwaysOnTop;
    private double zoom;
    private bool searching;

    /// <summary>
    /// The colors the menu of a sticky offers. They are the colors of the cards of Petasti
    /// (sosuisen/petasti, <c>cardColors</c>), without its transparent one: a sticky is a paper
    /// here. The SwiftUI shell offers the same.
    /// </summary>
    public static IReadOnlyList<(string Name, string Hex)> Colors { get; } =
    [
        ("Yellow", StickySettings.Yellow),
        ("Red", "#FFCAC4"),
        ("Green", "#EDFFC4"),
        ("Blue", "#D1E1FF"),
        ("Orange", "#FFDD9E"),
        ("Purple", "#F8D0FF"),
        ("White", "#FFFFFF"),
        ("Gray", "#E0E0E0"),
        ("Light Gray", "#F8F8F8"),
    ];

    public StickyLook(
        string color = StickySettings.Yellow,
        bool alwaysOnTop = false,
        double zoom = 1
    )
    {
        this.color = color;
        this.alwaysOnTop = alwaysOnTop;
        this.zoom = Tenths(zoom);
    }

    /// <summary>Something of the look changed.</summary>
    public event Action? Changed;

    /// <summary>The color of the paper, as <c>#RRGGBB</c>.</summary>
    public string Color
    {
        get => color;
        set => Set(ref color, value);
    }

    /// <summary>The window stays above the windows of other apps.</summary>
    public bool AlwaysOnTop
    {
        get => alwaysOnTop;
        set => Set(ref alwaysOnTop, value);
    }

    /// <summary>How large the page is shown: 1 is the usual size.</summary>
    public double Zoom => zoom;

    /// <summary>
    /// The search box is on screen. This is not kept: the box is closed in the next run.
    /// </summary>
    public bool Searching
    {
        get => searching;
        set => Set(ref searching, value);
    }

    public void ZoomIn() => Set(ref zoom, Tenths(zoom + 0.1));

    public void ZoomOut() => Set(ref zoom, Tenths(zoom - 0.1));

    public void ResetZoom() => Set(ref zoom, 1);

    private void Set<T>(ref T kept, T value)
    {
        if (EqualityComparer<T>.Default.Equals(kept, value))
            return;
        kept = value;
        Changed?.Invoke();
    }

    /// <summary>
    /// The zoom in whole tenths, between three tenths and three, as in the stickies app for
    /// Windows. Whole tenths, as steps of 0.1 do not add up to them.
    /// </summary>
    private static double Tenths(double zoom) => Math.Clamp(Math.Round(zoom * 10), 3, 30) / 10;

    /// <summary>
    /// The red, green and blue of the color that <c>#RRGGBB</c> names. Anything else is the
    /// yellow of a new sticky.
    /// </summary>
    public static (byte Red, byte Green, byte Blue) Rgb(string hex)
    {
        ArgumentNullException.ThrowIfNull(hex);
        if (
            hex.Length != 7
            || hex[0] != '#'
            || !uint.TryParse(
                hex.AsSpan(1),
                NumberStyles.AllowHexSpecifier,
                CultureInfo.InvariantCulture,
                out var value
            )
        )
            return Rgb(StickySettings.Yellow);
        return ((byte)(value >> 16), (byte)(value >> 8), (byte)value);
    }
}
