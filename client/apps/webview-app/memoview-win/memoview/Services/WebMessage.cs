using System.Text.Json;

namespace MemoView.Services;

/// <summary>
/// What the user did in a page, as the script of the page tells the app.
/// The script is <c>Web/page.js</c>. Keep the names the same in both files.
/// </summary>
internal abstract record WebMessage
{
    private WebMessage() { }

    /// <summary>The user changed the zoom. 1 is 100%.</summary>
    internal sealed record ZoomChanged(double Factor) : WebMessage;

    /// <summary>The user clicked the collapse arrow of an item.</summary>
    internal sealed record CollapseToggled(string ItemId) : WebMessage;

    /// <summary>The user clicked a wikilink.</summary>
    internal sealed record WikilinkClicked(string PageName) : WebMessage;

    /// <summary>The user clicked a hashtag. The tag has no hash sign.</summary>
    internal sealed record HashtagClicked(string Tag) : WebMessage;

    /// <summary>
    /// Reads a message of the page script.
    /// </summary>
    /// <param name="json">The message as JSON.</param>
    /// <returns>
    /// The message, or null when the text is not a message the app knows. A page shows text
    /// from the notes of the user, so the app does not trust what comes out of it.
    /// </returns>
    internal static WebMessage? Parse(string json)
    {
        ArgumentNullException.ThrowIfNull(json);
        try
        {
            using var document = JsonDocument.Parse(json);
            var root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object)
                return null;

            if (root.TryGetProperty("zoom", out var zoom))
            {
                return zoom.ValueKind == JsonValueKind.Number
                    ? new ZoomChanged(zoom.GetDouble())
                    : null;
            }
            if (Text(root, "toggleCollapse") is { } itemId)
                return new CollapseToggled(itemId);
            if (Text(root, "openWikilink") is { } pageName)
                return new WikilinkClicked(pageName);
            if (Text(root, "openHashtag") is { } tag)
                return new HashtagClicked(tag);
            return null;
        }
        catch (JsonException)
        {
            return null;
        }
    }

    // The text of a property, or null when the property is missing, not a text, or empty.
    private static string? Text(JsonElement root, string name)
    {
        if (!root.TryGetProperty(name, out var value))
            return null;
        if (value.ValueKind != JsonValueKind.String)
            return null;
        var text = value.GetString();
        return string.IsNullOrEmpty(text) ? null : text;
    }
}
