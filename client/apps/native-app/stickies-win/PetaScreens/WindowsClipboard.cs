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

    /// <summary>
    /// Writes the image on the clipboard to a file of its own, and gives its path. The core
    /// deletes the file once the server has answered (ADR 071). Null when there is no image,
    /// or when another app holds the clipboard.
    /// </summary>
    public static async Task<string?> Image()
    {
        try
        {
            var content = Clipboard.GetContent();
            if (!content.Contains(StandardDataFormats.Bitmap))
                return null;
            var bitmap = await content.GetBitmapAsync();
            using var source = await bitmap.OpenReadAsync();
            // A folder of its own: two images of the same name do not meet
            var folder = Directory.CreateTempSubdirectory("peta-pasted-").FullName;
            var path = Path.Combine(folder, "image.png");
            await using var file = File.Create(path);
            await source.AsStreamForRead().CopyToAsync(file);
            return path;
        }
        catch (Exception e) when (e is COMException or UnauthorizedAccessException)
        {
            // Another app holds the clipboard
            return null;
        }
    }
}
