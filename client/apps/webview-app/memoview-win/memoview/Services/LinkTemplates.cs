namespace MemoView.Services;

/// <summary>
/// How the links of a page are made, as the journal server gives it in its configuration.
/// The server gives the same values to every app, so that a link leads to the same page in
/// all of them (ADR 061). A template has <c>{}</c> where the name of a page goes.
/// </summary>
/// <param name="WikilinkHref">The template of the address of the page of a wikilink.</param>
/// <param name="HashtagHref">The template of the address of the page of a hashtag.</param>
/// <param name="PagesUrl">
/// The address that all pages live under. A path that is relative to a page, such as the path
/// of an image, is resolved against it.
/// </param>
/// <param name="WikilinkFragmentPattern">The form of the id of an item, after a <c>#</c> in a wikilink.</param>
internal sealed record LinkTemplates(
    string WikilinkHref,
    string HashtagHref,
    string PagesUrl,
    string WikilinkFragmentPattern
)
{
    /// <summary>
    /// The templates of the server at <paramref name="serverUrl"/>. The values of its
    /// configuration start at its root, and the notes have no address of their own, so the
    /// address of the server goes before them.
    /// </summary>
    /// <param name="serverUrl">The address of the journal server, without a slash at the end.</param>
    internal static LinkTemplates Of(
        string serverUrl,
        string wikilinkHref,
        string hashtagHref,
        string pagesBase,
        string wikilinkFragmentPattern
    )
    {
        ArgumentException.ThrowIfNullOrEmpty(serverUrl);
        return new LinkTemplates(
            serverUrl + wikilinkHref,
            serverUrl + hashtagHref,
            serverUrl + pagesBase,
            wikilinkFragmentPattern
        );
    }
}
