using Windows.System;

namespace PetaScreens;

/// <summary>
/// What hands a link to another app of Windows. It is left out of the coverage, as the
/// developer agreed: a test would open the real browser. See PetaBorder.Tests.csproj.
/// </summary>
internal static class Launch
{
    /// <summary>
    /// Opens the address with the app that Windows has for its scheme, whatever the scheme is
    /// (#1071). A path of a drive (<c>file:</c>) is not for <see cref="Launcher"/>, so the shell
    /// of Windows opens it as a double click would.
    /// </summary>
    public static void Browse(Uri address)
    {
        if (!address.IsFile)
        {
            _ = Launcher.LaunchUriAsync(address);
            return;
        }
        try
        {
            System
                .Diagnostics.Process.Start(
                    new System.Diagnostics.ProcessStartInfo(address.LocalPath)
                    {
                        UseShellExecute = true,
                    }
                )
                ?.Dispose();
        }
        catch (System.ComponentModel.Win32Exception)
        {
            // No file there, or no app for it: the click does nothing, as for a link of no page
        }
    }
}
