namespace PetaBorder;

/// <summary>How one sticky looks. The views of the sticky change it, and the desk keeps it.</summary>
public sealed class StickyLook
{
    private string color;
    private bool alwaysOnTop;
    private double zoom;
    private bool searching;

    /// <summary>The color of a sticky without paper, as <c>client-stickies</c> has it.</summary>
    private static readonly PetaCore.PetaStickyColor Clear =
        PetaCore.PetaNative.StickyTransparent();

    /// <summary>
    /// The colors the menu of a sticky offers, as <c>client-stickies</c> has them, and then
    /// no paper at all.
    /// </summary>
    public static IReadOnlyList<(string Name, string Hex)> Colors { get; } =
        PetaCore
            .PetaNative.StickyColors()
            .Append(Clear)
            .Select(color => (color.Name, color.Hex))
            .ToList();

    public StickyLook(string? color = null, bool alwaysOnTop = false, double zoom = 1)
    {
        this.color = color ?? PetaCore.PetaNative.StickyYellow();
        this.alwaysOnTop = alwaysOnTop;
        this.zoom = PetaCore.PetaNative.StickyZoom(zoom, 0);
    }

    /// <summary>Something of the look changed.</summary>
    public event Action? Changed;

    /// <summary>The color of the paper, as <c>#RRGGBB</c>, or the hex of no paper.</summary>
    public string Color
    {
        get => color;
        set => Set(ref color, value);
    }

    /// <summary>The sticky has no paper: what is behind it shows through.</summary>
    public bool Transparent => color == Clear.Hex;

    /// <summary>
    /// The sticky shows only what is written on it: a sticky without paper that is not worked
    /// on. Its bullets, the name and the button in its title bar, and its scroll bar would
    /// only cover what is behind it.
    /// </summary>
    public bool Bare(bool focused) => Transparent && !focused;

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

    public void ZoomIn() => Set(ref zoom, PetaCore.PetaNative.StickyZoom(zoom, 1));

    public void ZoomOut() => Set(ref zoom, PetaCore.PetaNative.StickyZoom(zoom, -1));

    public void ResetZoom() => Set(ref zoom, 1);

    private void Set<T>(ref T kept, T value)
    {
        if (EqualityComparer<T>.Default.Equals(kept, value))
            return;
        kept = value;
        Changed?.Invoke();
    }
}
