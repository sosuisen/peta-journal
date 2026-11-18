using System.Globalization;
using System.Net;

namespace MemoView.Services;

/// <summary>
/// The texts that every page document holds: the style sheet and the two scripts.
/// The app reads them from its embedded resources.
/// </summary>
/// <param name="Css">The style sheet of a page.</param>
/// <param name="RendererScript">The script of flatmarkdown-ast2html. It adds the global <c>FmAst2Html</c>.</param>
/// <param name="PageScript">The script of a page. It runs after the renderer script.</param>
internal sealed record PageAssets(string Css, string RendererScript, string PageScript);

/// <summary>
/// The values of a page document that change at run time.
/// </summary>
/// <param name="BackgroundColor">The background as a CSS color.</param>
/// <param name="ZoomFactor">The zoom, where 1 is 100%.</param>
/// <param name="Scrolls">Whether the page shows scroll bars. A sticky that is not active hides them.</param>
internal sealed record PageLook(string BackgroundColor, double ZoomFactor, bool Scrolls);

/// <summary>
/// Builds the HTML documents that a sticky shows in its WebView.
/// </summary>
internal static class PageHtml
{
    /// <summary>
    /// The document of a page.
    /// </summary>
    /// <param name="itemsHtml">The rows of the items, from <see cref="OutlineHtml"/>.</param>
    /// <param name="look">The values that change at run time.</param>
    /// <param name="links">How the links of the page are made.</param>
    /// <param name="assets">The style sheet and the scripts.</param>
    internal static string ForItems(
        string itemsHtml,
        PageLook look,
        LinkTemplates links,
        PageAssets assets
    )
    {
        ArgumentNullException.ThrowIfNull(itemsHtml);
        ArgumentNullException.ThrowIfNull(look);
        ArgumentNullException.ThrowIfNull(links);
        ArgumentNullException.ThrowIfNull(assets);

        var zoom = look.ZoomFactor.ToString(CultureInfo.InvariantCulture);
        var overflow = look.Scrolls ? "auto" : "hidden";
        var bodyStyle = $"background:{look.BackgroundColor};overflow:{overflow};zoom:{zoom};";
        return $"""
            <!DOCTYPE html>
            <html>
            <head>
            <meta charset="utf-8">
            <style>
            {assets.Css}
            </style>
            </head>
            <body style="{Attr(bodyStyle)}" data-pages="{Attr(
                links.PagesUrl
            )}" data-wikilink-href="{Attr(links.WikilinkHref)}" data-hashtag-href="{Attr(
                links.HashtagHref
            )}" data-fragment-pattern="{Attr(links.WikilinkFragmentPattern)}" data-zoom="{zoom}">
            {itemsHtml}
            <script>
            {assets.RendererScript}
            </script>
            <script>
            {assets.PageScript}
            </script>
            </body>
            </html>
            """;
    }

    /// <summary>
    /// A document that shows one line of text in the middle, in place of a page.
    /// </summary>
    /// <param name="message">The text. It is shown as text, not as HTML.</param>
    /// <param name="textColor">The color of the text as a CSS color.</param>
    /// <param name="backgroundColor">The background as a CSS color.</param>
    internal static string ForMessage(string message, string textColor, string backgroundColor)
    {
        ArgumentNullException.ThrowIfNull(message);
        var bodyStyle =
            "font-family:'Segoe UI',sans-serif;display:flex;justify-content:center;"
            + $"align-items:center;height:100vh;margin:0;background:{backgroundColor};color:{textColor};";
        return $"""
            <!DOCTYPE html>
            <html><head><meta charset="utf-8"></head>
            <body style="{Attr(bodyStyle)}"><p>{WebUtility.HtmlEncode(message)}</p></body></html>
            """;
    }

    private static string Attr(string value)
    {
        return WebUtility.HtmlEncode(value);
    }
}
