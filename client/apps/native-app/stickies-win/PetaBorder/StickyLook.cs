
namespace PetaBorder;

/// <summary>How one sticky looks. The views of the sticky change it, and the desk keeps it.</summary>
public sealed class StickyLook
{
    private string color;
    private bool alwaysOnTop;
    private double zoom;
    private bool searching;

    /// <summary>The colors the menu of a sticky offers, as <c>client-stickies</c> has them.</summary>
    public static IReadOnlyList<(string Name, string Hex)> Colors { get; } =
        PetaCore.PetaNative.StickyColors().Select(color => (color.Name, color.Hex)).ToList();

    public StickyLook(
        string? color = null,
        bool alwaysOnTop = false,
        double zoom = 1
    )
    {
        this.color = color ?? PetaCore.PetaNative.StickyYellow();
        this.alwaysOnTop = alwaysOnTop;
        this.zoom = PetaCore.PetaNative.StickyZoom(zoom, 0);
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
