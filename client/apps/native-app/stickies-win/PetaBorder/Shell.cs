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
        View = border.View();
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
    public event Action? TitleChanged;
    public event Action? UrlChanged;
    public event Action? QuestionChanged;
    public event Action? FocusChanged;

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
    /// The user clicked a link of a shown item. Every link opens in the browser: a page opens
    /// in the web frontend, and a file of the assets folder as the journal server serves it.
    /// The sticky keeps its page.
    /// </summary>
    public void Follow(RichLink link)
    {
        ArgumentNullException.ThrowIfNull(link);
        var address = link switch
        {
            RichLink.Page page => PetaCore.PetaNative.PageAddress(server, page.Url),
            RichLink.Web web => web.Url,
            RichLink.File file => AssetAddress(file.Path)?.AbsoluteUri,
        };
        if (Uri.TryCreate(address, UriKind.Absolute, out var there))
            browse(there);
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
    /// The user clicked the mark of a task: its state goes one step on, as with the key.
    /// </summary>
    public void ClickedTask(string itemId) => Send(new Event.TaskCycle(itemId));

    /// <summary>
    /// The user touched the shown text of the item after <paramref name="letter"/> of its
    /// letters, as <c>RichParagraph.Start</c> counts them. The item is edited with the cursor
    /// at that place in its Markdown (ADR 043).
    /// </summary>
    public void Touched(string itemId, int letter) =>
        Send(new Event.ShownTextTouched(itemId, (ulong)letter));

    /// <summary>The user put the cursor in the item.</summary>
    public void StartEditing(string itemId) => Send(new Event.EditRequested(itemId, 0));

    /// <summary>
    /// The cursor left the item. The core saves what was typed, and the item is shown with its
    /// decoration again.
    /// </summary>
    public void FinishEditing(string itemId) => Send(new Event.EditFinished(itemId));

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
    public void Typed(string body, string itemId) =>
        Send(new Event.BodyChanged(itemId, body, NowMs()));

    /// <summary>
    /// The user typed into the item, and the cursor is <paramref name="cursor"/> UTF-16 units into
    /// <paramref name="body"/>. An opening just typed gets its closing (ADR 050).
    /// </summary>
    public void Typed(string body, string itemId, int cursor) =>
        Typed(PetaCore.PetaNative.ClosedAsTyped(body, (ulong)cursor) ?? body, itemId);

    /// <summary>Answers the confirmation on screen.</summary>
    public void Answer(bool yes) => Answer(new UiOutput.Bool(yes));

    /// <summary>
    /// Answers the prompt on screen. <c>null</c> is the answer of a user who closed it.
    /// </summary>
    public void Answer(string? text) => Answer(new UiOutput.Text(text));

    private void Answer(UiOutput output)
    {
        if (Question is not { } asked)
            return;
        Question = null;
        border.Answer(asked.Id, output);
        Show(border.View());
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
        Show(border.View());
    }

    private void Show(ViewModel view)
    {
        var before = Title;
        View = view;
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
                    Show(border.View());
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
            Show(border.View());
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
            case UiOp.Prompt { Value: var message }:
                Ask(new Question(id, message, QuestionKind.Prompt));
                break;
            case UiOp.CopyText { Value: var text }:
                copy(text, null);
                break;
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
        }
    }

    private static ulong NowMs() => (ulong)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();

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

    /// <summary>A line of text.</summary>
    Prompt,
}
