namespace Stickies.Services;

/// <summary>
/// Builds the address of a page in the PetaJournal web app.
/// The journal server serves the web app, and a page lives at <c>/pages/&lt;page name&gt;</c>.
/// </summary>
internal static class PageLinks
{
    /// <summary>
    /// The address of the page that a wikilink names.
    /// </summary>
    /// <param name="serverUrl">The address of the journal server, without a slash at the end.</param>
    /// <param name="pageName">The page name. It is not empty, and it may hold folders.</param>
    internal static Uri ForWikilink(string serverUrl, string pageName)
    {
        ArgumentException.ThrowIfNullOrEmpty(serverUrl);
        ArgumentException.ThrowIfNullOrEmpty(pageName);
        return new Uri($"{serverUrl}/pages/{EscapePath(pageName)}");
    }

    /// <summary>
    /// The address of the page of a tag. The tag pages live in a folder of their own.
    /// </summary>
    /// <param name="serverUrl">The address of the journal server, without a slash at the end.</param>
    /// <param name="tagsDir">The folder of the tag pages. It is empty for the pages root and ends with a slash otherwise.</param>
    /// <param name="tag">The tag name without the hash sign. It is not empty.</param>
    internal static Uri ForHashtag(string serverUrl, string tagsDir, string tag)
    {
        ArgumentNullException.ThrowIfNull(tagsDir);
        ArgumentException.ThrowIfNullOrEmpty(tag);
        return ForWikilink(serverUrl, tagsDir + tag);
    }

    // A slash separates folders, so it stays. Any other character must not change the path.
    private static string EscapePath(string path)
    {
        return string.Join("/", path.Split('/').Select(Uri.EscapeDataString));
    }
}
