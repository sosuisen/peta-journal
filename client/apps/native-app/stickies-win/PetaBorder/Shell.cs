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

    /// <summary>The questions that wait behind the one on screen.</summary>
    private readonly Queue<Question> later = new();

    /// <summary>The name that the core gave to what is open.</summary>
    private string named = "";
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
    /// The name of what is open. For a journal it is the date of the day at the top of the
    /// window, with "(Today)" behind the date of today.
    /// </summary>
    public string Title { get; private set; } = "";

    /// <summary>
    /// What is open, as the URL that opens it again. <c>null</c> when nothing was opened yet.
    /// </summary>
    public string? Url { get; private set; }

    /// <summary>What the core asks the user now. The core waits until it is answered.</summary>
    public Question? Question { get; private set; }

    /// <summary>The last order of the core to put the cursor somewhere.</summary>
    public FocusOrder? Focus { get; private set; }

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
    /// The user clicked a link of a shown item. Every link opens in the browser: a wikilink
    /// and a hashtag open their page in the web frontend, which the journal server serves.
    /// The sticky keeps its page.
    /// </summary>
    public void Follow(LinkTarget link)
    {
        switch (link)
        {
            case LinkTarget.Page page:
                Browse(PetaCore.PetaNative.WikilinkUrl(server, page.Path));
                break;
            case LinkTarget.Tag tag:
                Browse(PetaCore.PetaNative.HashtagUrl(server, View.Config.TagsDir, tag.Name));
                break;
            case LinkTarget.Out outside:
                browse(outside.Address);
                break;
        }
    }

    private void Browse(string address)
    {
        if (Uri.TryCreate(address, UriKind.Absolute, out var there))
            browse(there);
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

    /// <summary>The user put the cursor in the item.</summary>
    public void StartEditing(string itemId) => Send(new Event.EditRequested(itemId, 0));

    /// <summary>
    /// The cursor left the item. The core saves what was typed, and the item is shown with its
    /// decoration again.
    /// </summary>
    public void FinishEditing(string itemId) => Send(new Event.EditFinished(itemId));

    /// <summary>The user changed the text of the item. The core saves it a moment later.</summary>
    public void Typed(string body, string itemId) =>
        Send(new Event.BodyChanged(itemId, body, NowMs()));

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
        Question = later.Count == 0 ? null : later.Dequeue();
        border.Answer(asked.Id, output);
        Show(border.View());
        QuestionChanged?.Invoke();
    }

    private void Ask(Question asked)
    {
        if (Question is null)
        {
            Question = asked;
            QuestionChanged?.Invoke();
        }
        else
        {
            later.Enqueue(asked);
        }
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
        View = view;
        Name();
        ViewChanged?.Invoke();
    }

    /// <summary>
    /// Sets the title. The core names every journal by the day it was opened for; the day at
    /// the top of the window is only in the view model.
    /// </summary>
    private void Name()
    {
        var title = named;
        if (
            View.Screen is ScreenView.Journal { Value: var journal }
            && journal.DisplayDate.Length > 0
        )
        {
            // The days below the one that was opened have their date above their first item
            var top = View.Outline.Rows.FirstOrDefault(row =>
                row.DateHeader == journal.DisplayDate
            );
            var day = top?.Item.Name ?? journal.CurrentDate;
            title = day == View.Today ? $"{journal.DisplayDate} (Today)" : journal.DisplayDate;
        }
        // Not told again when it is the same: everything that watches the title would run
        if (Title == title)
            return;
        Title = title;
        TitleChanged?.Invoke();
    }

    private void Handle(IReadOnlyList<Request> requests)
    {
        // The window was closed while the requests waited for the main thread
        if (disposed)
            return;
        var cursors = new List<(uint Id, string? ItemId)>();
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
                case ShellEffect.Ui { Value: UiOp.FocusEditor focus }:
                    cursors.Add((request.Id, focus.ItemId));
                    break;
                case ShellEffect.Ui { Value: var op }:
                    Show(op, request.Id);
                    break;
            }
        }
        // After the render: the item to put the cursor in may have come with these very requests
        foreach (var (id, itemId) in cursors)
        {
            Focus = new FocusOrder((Focus?.Number ?? 0) + 1, itemId);
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
            case UiOp.SetTitle { Value: var title }:
                named = title;
                Name();
                break;
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

/// <summary>
/// An order to put the cursor in the editor of an item, or of the first item when
/// <paramref name="ItemId"/> is <c>null</c>. <paramref name="Number"/> tells two orders for the
/// same item apart.
/// </summary>
public sealed record FocusOrder(int Number, string? ItemId);
