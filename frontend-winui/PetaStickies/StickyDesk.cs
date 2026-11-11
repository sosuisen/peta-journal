using PetaBorder;

namespace PetaStickies;

/// <summary>
/// The stickies that are open. A sticky is added with a button of any sticky, and is gone when
/// its window is closed. With a store, the stickies are kept for the next run of the app.
/// </summary>
internal sealed class StickyDesk
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

    /// <summary>Opens the stickies that were kept, or a first one without a page.</summary>
    public void Open()
    {
        var kept = store?.Stickies.ToList() ?? [];
        if (kept.Count == 0)
            Add();
        foreach (var sticky in kept)
            Show(sticky, null);
    }

    /// <summary>Opens a new sticky that shows what the URL names, or no page yet.</summary>
    public void Add(string? url = null) => Show(null, url);

    private void Show(StickySettings? kept, string? url)
    {
        var window = new StickyWindow(this, server, kept, url, stickies.LastOrDefault());
        stickies.Add(window);
        window.Keep();
        window.Activate();
    }

    public void Keep(StickySettings sticky)
    {
        if (stickies.Exists(open => open.Id == sticky.Id))
            store?.Put(sticky);
    }

    /// <summary>A sticky that the user closed does not come back.</summary>
    public void Closed(StickyWindow closed)
    {
        stickies.Remove(closed);
        if (!ending)
            store?.Remove(closed.Id);
    }

    /// <summary>
    /// Ends the app. The windows that are closed here were not closed one by one by the user,
    /// and so their stickies stay kept.
    /// </summary>
    public void End()
    {
        ending = true;
        foreach (var sticky in stickies.ToList())
            sticky.Close();
    }
}
