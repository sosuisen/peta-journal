namespace PetaBorder;

/// <summary>A rectangle on the screens, in pixels.</summary>
public readonly record struct Area(int Left, int Top, int Width, int Height)
{
    /// <summary>The area, moved so that it is inside <paramref name="screen"/>.</summary>
    public Area Within(Area screen) =>
        this with
        {
            Left = Math.Max(Math.Min(Left, screen.Left + screen.Width - Width), screen.Left),
            Top = Math.Max(Math.Min(Top, screen.Top + screen.Height - Height), screen.Top),
        };
}
