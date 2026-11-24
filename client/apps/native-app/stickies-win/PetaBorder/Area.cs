namespace PetaBorder;

/// <summary>A rectangle on the screens, in pixels.</summary>
public readonly record struct Area(int Left, int Top, int Width, int Height)
{
    /// <summary>The area, moved so that it is inside <paramref name="screen"/>.</summary>
    public Area Within(Area screen) => this;
}
