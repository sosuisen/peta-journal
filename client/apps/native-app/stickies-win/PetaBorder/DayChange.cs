namespace PetaBorder;

/// <summary>
/// The messages of Windows after which the date may be another one. The core does not count
/// the time while the PC sleeps, and would open the new day only up to a minute later (#1031).
/// </summary>
public static class DayChange
{
    private const uint PowerBroadcast = 0x0218;
    private const uint TimeChange = 0x001E;

    /// <summary>Windows woke from sleep, as it does with no user, as for an alarm.</summary>
    private const nint ResumeAutomatic = 0x0012;

    /// <summary>Windows woke from sleep for the user.</summary>
    private const nint ResumeSuspend = 0x0007;

    /// <summary>
    /// The date may have changed when Windows told a window <paramref name="message"/> with
    /// <paramref name="wParam"/>.
    /// </summary>
    public static bool MayFollow(uint message, nint wParam) =>
        message switch
        {
            PowerBroadcast => wParam is ResumeAutomatic or ResumeSuspend,
            TimeChange => true,
            _ => false,
        };
}
