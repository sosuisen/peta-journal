namespace PetaBorder;

/// <summary>
/// The messages of Windows after which the date may be another one. The core does not count
/// the time while the PC sleeps, and would open the new day only up to a minute later (#1031).
/// </summary>
public static class DayChange
{
    /// <summary>
    /// The date may have changed when Windows told a window <paramref name="message"/> with
    /// <paramref name="wParam"/>.
    /// </summary>
    public static bool MayFollow(uint message, nint wParam) =>
        message == 0x0218 && wParam is 0x0012 or 0x0007 || message == 0x001E;
}
