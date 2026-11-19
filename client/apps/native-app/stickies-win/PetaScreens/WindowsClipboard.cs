using System.Runtime.InteropServices;
using Windows.ApplicationModel.DataTransfer;

namespace PetaScreens;

/// <summary>The clipboard of Windows, as the core reads it.</summary>
internal static class WindowsClipboard
{
    /// <summary>
    /// The text on the clipboard, with the line breaks of the core. Null when there is no text,
    /// or when another app holds the clipboard.
    /// </summary>
    public static async Task<string?> Text()
    {
        try
        {
            var content = Clipboard.GetContent();
            if (!content.Contains(StandardDataFormats.Text))
                return null;
            // The other apps of Windows put its line breaks on the clipboard
            return (await content.GetTextAsync()).Replace("\r\n", "\n");
        }
        catch (Exception e) when (e is COMException or UnauthorizedAccessException)
        {
            // Another app holds the clipboard
            return null;
        }
    }
}
