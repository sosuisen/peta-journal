using PetaTypes;

namespace PetaBorder;

/// <summary>
/// What one window shows, kept up to date with the core. The views draw <see cref="View"/> and
/// tell this what the user did. All of it runs on the main thread.
/// </summary>
public sealed partial class Shell : IDisposable
{
    private readonly Border border;
    private readonly Action<Action> onMainThread;
    private readonly Action<Action<string?>> clipboard;
    private readonly Action<string, string?> copy;
    private readonly Action<Uri> browse;

    /// <summary>
    /// The address of the journal server. It serves the web frontend too, so a link that
    /// another app is to open starts with it.
    /// </summary>
    private readonly string server;

    private bool disposed;

    /// <summary>
    /// Starts a core for the journal server at <paramref name="server"/> and opens what
    /// <paramref name="url"/> names, or nothing when there is no URL: a page is then to be
    /// searched for.
    /// </summary>
    /// <param name="onMainThread">
    /// Runs what it is given on the main thread, later and in order. The core calls from its
    /// own threads.
    /// </param>
    /// <param name="clipboard">
    /// Reads the text on the clipboard when the core asks for it, and hands it over on the
    /// main thread. No text is <c>null</c>.
    /// </param>
    /// <param name="copy">
    /// Puts text on the clipboard, with the same as HTML when there is any.
    /// </param>
    /// <param name="browse">Opens an address outside the journal.</param>
    public Shell(
        string server,
        string? url,
        Action<Action> onMainThread,
        Action<Action<string?>> clipboard,
        Action<string, string?> copy,
        Action<Uri> browse
    )
    {
        ArgumentNullException.ThrowIfNull(onMainThread);
        ArgumentNullException.ThrowIfNull(clipboard);
        ArgumentNullException.ThrowIfNull(copy);
        ArgumentNullException.ThrowIfNull(browse);
        this.onMainThread = onMainThread;
        this.clipboard = clipboard;
        this.copy = copy;
        this.browse = browse;
        this.server = server;
        // The main thread keeps the requests in order. The first ones may come before the
        // border is there, and are still handled after this constructor
        border = new Border(server, requests => onMainThread(() => Handle(requests)));
        View = Applied.First(border.Changes());
        if (url is not null)
            Open(url);
    }

    public ViewModel View { get; private set; }

    /// <summary>
    /// The name of what is open, as the core gives it in the view model. For a journal it is
    /// the date of the day at the top of the window, with "(Today)" behind the date of today.
    /// </summary>
    public string Title => View.Title;

    /// <summary>
    /// What is open, as the URL that opens it again. <c>null</c> when nothing was opened yet.
    /// </summary>
    public string? Url { get; private set; }

    /// <summary>What the core asks the user now. The core waits until it is answered.</summary>
    public Question? Question { get; private set; }

    /// <summary>
    /// The last order of the core to put the cursor somewhere, as the view model gives it. The
    /// first item is meant when it names none.
    /// </summary>
    public FocusOrder? Focus => View.Outline.FocusOrder;

    public event Action? ViewChanged;

    /// <summary>
    /// The rows of the outline changed: the changes, applied in order to the rows as they were,
    /// make them the rows of <see cref="View"/> now. It comes before <see cref="ViewChanged"/>.
    /// </summary>
    public event Action<IReadOnlyList<ListChange>>? RowsChanged;
    public event Action? TitleChanged;
    public event Action? UrlChanged;
    public event Action? QuestionChanged;
    public event Action? FocusChanged;

    /// <summary>The user asked for a new sticky that opens the URL it is given.</summary>
    public event Action<string>? NewStickyAsked;

    /// <summary>Opens what the URL names. A URL that names nothing changes nothing.</summary>
    public void Open(string url)
    {
        try
        {
            border.Open(url);
        }
        catch (PetaCore.BoltException)
        {
            return;
        }
        Url = url;
        UrlChanged?.Invoke();
    }

    /// <summary>
    /// Back to the page that was open before, as the core keeps the history (ADR 035). Nothing
    /// happens when <c>View.History.CanGoBack</c> is false.
    /// </summary>
    public void GoBack() => Turn(new Event.Back(Today()));

    /// <summary>Forward to the page that was open before <see cref="GoBack"/>.</summary>
    public void GoForward() => Turn(new Event.Forward(Today()));

    private void Turn(Event turned)
    {
        Send(turned);
        // The URL of the page that the core opened from its history
        if (View.History.Url is not { } shown || shown == Url)
            return;
        Url = shown;
        UrlChanged?.Invoke();
    }

    /// <summary>The local date, as <c>YYYY-MM-DD</c>.</summary>
    private static string Today() =>
        DateTime.Now.ToString("yyyy-MM-dd", System.Globalization.CultureInfo.InvariantCulture);

    /// <summary>
    /// The user clicked a link of a shown item, with the keys in <paramref name="held"/>. The
    /// core decides how it opens (ADR 083): here, in a new sticky, or in the browser. Alt means
    /// nothing to it.
    /// </summary>
    public void Follow(RichLink link, Held held)
    {
        ArgumentNullException.ThrowIfNull(link);
        var keys = new PetaTypes.Held
        {
            Shift = held.HasFlag(Held.Shift),
            Ctrl = held.HasFlag(Held.Control),
        };
        Send(new Event.LinkClicked(link, keys));
    }

    /// <summary>
    /// The address of a file of the assets folder, such as <c>assets/picture.png</c>, on the
    /// journal server.
    /// </summary>
    public Uri? AssetAddress(string path)
    {
        ArgumentNullException.ThrowIfNull(path);
        var address = PetaCore.PetaNative.AssetAddress(server, path);
        return Uri.TryCreate(address, UriKind.Absolute, out var uri) ? uri : null;
    }

    /// <summary>
    /// The window was scrolled: the row at <paramref name="top"/> is the first that can be
    /// seen, and the rows from <paramref name="first"/> to <paramref name="last"/> are drawn.
    /// A journal tells the day at the top by this, and loads the days before when the end of
    /// what it has comes near.
    /// </summary>
    public void Scrolled(int top, int first, int last) =>
        Send(new Event.OutlineScrolled(top > 0, (ulong)top, (ulong)first, (ulong)(last + 1)));

    /// <summary>
    /// The day may have changed while the core could not look at the clock, as when the PC
    /// woke: the journal of today opens the new day at once (#1031).
    /// </summary>
    public void DayMayHaveChanged() => Send(new Event.DateMayHaveChanged());

    /// <summary>
    /// The user clicked the mark of a task: its state goes one step on, as with the key.
    /// </summary>
    public void ClickedTask(string itemId) => Send(new Event.TaskCycle(itemId));

    /// <summary>
    /// The user chose "Paste as Transclusion" in the menu of the item. The core asks for the
    /// text on the clipboard. When it is the id of an item, the item shows that item.
    /// </summary>
    public void PasteAsTransclusion(string itemId) => Send(new Event.PasteAsTransclusion(itemId));

    /// <summary>
    /// The user touched the shown text of the item after <paramref name="letter"/> of its
    /// letters, as <c>RichParagraph.Start</c> counts them. The item is edited with the cursor
    /// at that place in its Markdown (ADR 043). The core ends the edit of another item and a
    /// selection (ADR 057).
    /// </summary>
    public void Touched(string itemId, int letter) =>
        Send(new Event.EditRequested(itemId, new EditPlace.ShownLetter((ulong)letter)));

    /// <summary>
    /// The user clicked something in the sticky that is no item, such as the title bar or a
    /// button. The core ends a selection, unless the items are being selected by a drag.
    /// </summary>
    public void ClickedOutsideItems() => Send(new Event.SelectionClickOutside());

    /// <summary>
    /// The user put the cursor in the item, <paramref name="cursor"/> characters into its
    /// Markdown. The core ends the edit of another item and a selection (ADR 057).
    /// </summary>
    public void StartEditing(string itemId, int cursor = 0) =>
        Send(new Event.EditRequested(itemId, new EditPlace.Markdown((ulong)cursor)));

    /// <summary>
    /// The cursor left the item. The core saves what was typed, and the item is shown with its
    /// decoration again.
    /// </summary>
    public void FinishEditing(string itemId)
    {
        // The window was closed while an item was edited. Its editor loses the cursor after
        // the core is gone, and there is nothing to tell
        if (!disposed)
            Send(new Event.EditFinished(itemId));
    }

    /// <summary>
    /// The editor of the item went away while the list changes its rows. The core is told on a
    /// later turn of the main thread. A new view now would change the rows in the middle of
    /// the change, and the list of WinUI breaks then.
    /// </summary>
    public void FinishEditingLater(string itemId) =>
        onMainThread(() =>
        {
            // The window was closed while this waited for the main thread
            if (!disposed)
                FinishEditing(itemId);
        });

    /// <summary>The user changed the text of the item. The core saves it a moment later.</summary>
    public void Typed(string body, string itemId) => Typed(new Typing.Changed(itemId, body));

    /// <summary>
    /// The user typed into the item, and the cursor is <paramref name="cursor"/> UTF-16 units into
    /// <paramref name="body"/>. The core closes an opening just typed (ADR 057), and saves the
    /// body a moment later.
    /// </summary>
    public void Typed(string body, string itemId, int cursor) =>
        Typed(new Typing.Typed(itemId, body, (ulong)Math.Max(cursor, 0)));

    /// <summary>The core adds the time, and counts the cursor in its own characters.</summary>
    private void Typed(Typing typing)
    {
        border.Typed(typing);
        // As after Send: an editor that reads its old text back after a key loses the key
        Show(border.Changes());
    }

    /// <summary>Answers the confirmation on screen.</summary>
    public void Answer(bool yes)
    {
        if (Question is not { } asked)
            return;
        Question = null;
        border.Answer(asked.Id, new UiOutput.Bool(yes));
        Show(border.Changes());
        QuestionChanged?.Invoke();
    }

    private void Ask(Question asked)
    {
        Question = asked;
        QuestionChanged?.Invoke();
    }

    /// <summary>Sends what the user did to the core.</summary>
    public void Send(Event sent)
    {
        border.Send(sent);
        // Not left to the render request. It comes a turn of the main thread later, and an
        // editor that reads its old text back after a key loses the key. Some events, such as
        // the start of an edit, send no render request at all
        Show(border.Changes());
    }

    /// <summary>
    /// Puts what changed on the screen into <see cref="View"/>, and tells the views. The core
    /// sends only what changed since it was last asked (ADR 076).
    /// </summary>
    private void Show(Changes changes)
    {
        var before = Title;
        Applied.To(View, changes);
        if (changes.Rows.Count > 0)
            RowsChanged?.Invoke(changes.Rows);
        // Not told when it is the same: everything that watches the title would run
        if (Title != before)
            TitleChanged?.Invoke();
        ViewChanged?.Invoke();
    }

    private void Handle(IReadOnlyList<Request> requests)
    {
        // The window was closed while the requests waited for the main thread
        if (disposed)
            return;
        var cursors = new List<uint>();
        foreach (var request in requests)
        {
            switch (request.Effect)
            {
                case ShellEffect.Render:
                    Show(border.Changes());
                    break;
                case ShellEffect.Navigate { Value: var op }:
                    Open(op.Url);
                    break;
                case ShellEffect.Ui { Value: UiOp.FocusEditor }:
                    cursors.Add(request.Id);
                    break;
                case ShellEffect.Ui { Value: var op }:
                    Show(op, request.Id);
                    break;
            }
        }
        // After the render: the item to put the cursor in may have come with these very requests
        // The order itself is in the view model. It is read again here: no render request has
        // to come with the order
        if (cursors.Count > 0)
            Show(border.Changes());
        foreach (var id in cursors)
        {
            border.Answer(id, new UiOutput.Done());
            FocusChanged?.Invoke();
        }
    }

    /// <summary>
    /// Only some requests wait for an answer. The core takes an answer to any other as an error.
    /// </summary>
    private void Show(UiOp op, uint id)
    {
        switch (op)
        {
            case UiOp.Confirm { Value: var message }:
                Ask(new Question(id, message, QuestionKind.Confirm));
                break;
            case UiOp.ConfirmDestructive { Value: var message }:
                Ask(new Question(id, message, QuestionKind.ConfirmDestructive));
                break;
            // No prompt is shown. The core asks one only for a new page without a name, and
            // this app has no way to make one: its search box makes a page with the typed name
            case UiOp.CopyMarkdown { Markdown: var markdown, Ast: var ast }:
                // The HTML is for other apps (ADR 027). Its links to pages open the web
                // frontend, as those of a copy made there: this app has no address that
                // another app opens
                var html = PetaCore.PetaNative.CopiedHtml(ast, server, View.Config.TagsDir);
                copy(markdown, html.Length == 0 ? null : html);
                break;
            case UiOp.ReadClipboardText:
                clipboard(text =>
                {
                    if (!disposed)
                        border.Answer(id, new UiOutput.Text(text));
                });
                break;
            // A new view of the app is a new sticky (ADR 083)
            case UiOp.OpenNewView { Url: var url }:
                NewStickyAsked?.Invoke(url);
                break;
            // The bridge has put the address of the server before an address of the server
            case UiOp.OpenInBrowser { Address: var address }:
                if (Uri.TryCreate(address, UriKind.Absolute, out var there))
                    browse(there);
                break;
        }
    }

    public void Dispose()
    {
        disposed = true;
        border.Dispose();
    }
}

/// <summary>Something the core asks the user and waits for.</summary>
/// <param name="Id">The number of the request of the core.</param>
public sealed record Question(uint Id, string Message, QuestionKind Kind);

public enum QuestionKind
{
    /// <summary>Yes or no.</summary>
    Confirm,

    /// <summary>Yes or no to something hard to undo: the dialog must answer no by default.</summary>
    ConfirmDestructive,
}
