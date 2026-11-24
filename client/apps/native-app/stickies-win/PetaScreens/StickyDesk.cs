using PetaBorder;

namespace PetaScreens;

/// <summary>
/// The stickies that are open. A sticky is added with a button of any sticky, and is gone when
/// its window is closed. With a store, the stickies are kept for the next run of the app.
/// </summary>
public sealed class StickyDesk
{
    private readonly List<StickyWindow> stickies = [];
    private readonly string server;
    private readonly StickyStore? store;

    /// <summary>The app is ending.</summary>
    private bool ending;

    /// <summary>The stickies show pages of the journal server at <paramref name="server"/>.</summary>
    public StickyDesk(string server, StickyStore? store = null)
    {
        this.server = server;
        this.store = store;
    }

    /// <summary>The stickies that are open, in the order they were opened.</summary>
    internal IReadOnlyList<StickyWindow> Stickies => stickies;

    /// <summary>Opens the stickies that were kept, or a first one without a page.</summary>
    public void Open()
    {
        var kept = store?.Stickies.ToList() ?? [];
        if (kept.Count == 0)
            Add();
        foreach (var sticky in kept)
            Show(sticky, null, null);
    }

    /// <summary>Opens a new sticky that shows what the URL names, or no page yet.</summary>
    public void Add(string? url = null) => Show(null, url, stickies.LastOrDefault());

    /// <summary>
    /// Opens a new sticky a little down and to the right of the sticky that the user worked
    /// in, with what the URL names or no page yet.
    /// </summary>
    internal void AddNextTo(StickyWindow from, string? url = null) => Show(null, url, from);

    private void Show(PetaCore.PetaSticky? kept, string? url, StickyWindow? before)
    {
        var window = new StickyWindow(this, server, kept, url, before);
        stickies.Add(window);
        window.Keep();
        window.Activate();
    }

    internal void Keep(PetaCore.PetaSticky sticky)
    {
        if (stickies.Exists(open => open.Id == sticky.Id))
            store?.Put(sticky);
    }

    /// <summary>
    /// A sticky that the user closed with the button of its title bar does not come back.
    /// A window that was closed in any other way was not put away by the user: Alt+F4,
    /// the taskbar, or Windows that ends the session.
    /// </summary>
    internal void Closed(StickyWindow closed, bool byButton)
    {
        stickies.Remove(closed);
        if (byButton && !ending)
            store?.Remove(closed.Id);
        // The app ends with its last window. Nothing is kept after it
        if (stickies.Count == 0)
            store?.Dispose();
    }

    /// <summary>
    /// Ends the app. The windows that are closed here were not closed one by one by the user,
    /// and so their stickies stay kept.
    /// </summary>
    internal void End()
    {
        ending = true;
        foreach (var sticky in stickies.ToList())
            sticky.Close();
    }
}
