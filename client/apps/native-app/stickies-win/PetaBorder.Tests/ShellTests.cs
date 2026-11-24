using PetaTypes;

namespace PetaBorder.Tests;

/// <summary>
/// The shell against a real journal server. These are skipped without <c>PETA_E2E_SERVER</c>.
/// </summary>
public sealed class ShellTests : IDisposable
{
    /// <summary>
    /// An extension of the server that asks the clients, as an extension of the user may. It
    /// asks when a page gets a tag that starts with "ask", and adds an item after a yes.
    /// </summary>
    private const string AskingExtension = """
        globalThis.onPageTagAdd = (page, tags) => {
            if (!tags.some(tag => tag.startsWith('ask'))) return;
            if (peta.confirm('Insert into ' + page.fileName + '?')) {
                peta.createItem({ position: 'last', level: 1, body: 'confirmed' });
            }
        };
        """;

    /// <summary>
    /// Skips a test of a key with Control on a system other than Windows. The core reads the keys
    /// of the system it was built for, and a Mac has Command where Windows has Control. The tests
    /// run on a Mac as well, for those who work there (README).
    /// </summary>
    private static void SkipWithoutTheKeysOfWindows() =>
        SkipTest.When(
            !OperatingSystem.IsWindows(),
            "the core built for this system reads Command where Windows has Control"
        );

    private readonly JournalServer server = new(
        18111,
        new Dictionary<string, string> { ["asking"] = AskingExtension }
    );
    private readonly MainThread main = new();
    private readonly List<Shell> shells = [];

    /// <summary>What the clipboard holds, for the core to read.</summary>
    private string? onClipboard;

    /// <summary>What the shell put on the clipboard.</summary>
    private string? copied;

    /// <summary>The HTML that the shell put on the clipboard with the text.</summary>
    private string? copiedHtml;

    /// <summary>The last address that the shell gave to the browser.</summary>
    private Uri? browsed;

    public void Dispose()
    {
        shells.ForEach(shell => shell.Dispose());
        server.Dispose();
    }

    private Shell NewShell(string? url)
    {
        var shell = new Shell(
            server.Address,
            url,
            main.Post,
            reply => reply(onClipboard),
            (text, html) => (copied, copiedHtml) = (text, html),
            uri => browsed = uri
        );
        shells.Add(shell);
        return shell;
    }

    /// <summary>A shell with a page of its own on screen.</summary>
    private Shell ShellWithPage()
    {
        var shell = NewShell("/pages/winui-notes");
        main.WaitUntil("the page is shown", () => shell.View.Outline.Rows.Count > 0);
        return shell;
    }

    /// <summary>
    /// Types into the first item and presses Enter at the end of it, which makes a second item.
    /// </summary>
    private (string First, string Second) TwoItems(Shell shell)
    {
        var first = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(first);
        shell.Typed("first", first);
        Assert.True(Press(shell, Enter, first, Editor("first", 5)));
        main.WaitUntil("there are two items", () => shell.View.Outline.Rows.Count == 2);
        return (first, shell.View.Outline.Rows[1].Item.Id);
    }

    /// <summary>
    /// Waits until the shell hears what the server tells its clients. A change of another
    /// client before that would not reach it.
    /// </summary>
    private void HearsTheServer(Shell shell) =>
        main.WaitUntil("the shell hears the server", () =>
            shell.View.Connection == ConnectionState.Connected
        );

    /// <summary>Gives the page the tag, as another client does.</summary>
    private void Tag(string pageId, string tag) =>
        Assert.True(
            server.Send(
                HttpMethod.Put,
                $"/api/pages/{pageId}/properties/tags",
                $$"""{"valueType":"tags","value":"{{tag}}"}"""
            )
        );

    /// <summary>The page on screen, once it is there.</summary>
    private static PageView? Page(Shell shell) => (shell.View.Screen as ScreenView.Page)?.Value;

    /// <summary>The journal on screen, once it is there.</summary>
    private static JournalView? Journal(Shell shell) =>
        (shell.View.Screen as ScreenView.Journal)?.Value;

    private static List<string> Bodies(Shell shell) =>
        [.. shell.View.Outline.Rows.Select(row => row.Item.Body)];

    private static List<string> Selected(Shell shell) =>
        [.. shell.View.Outline.Rows.Where(row => row.IsSelected).Select(row => row.Item.Body)];

    /// <summary>An editor of one line with the cursor at <paramref name="cursor"/>.</summary>
    private static EditorState Editor(string text, ulong cursor) =>
        new()
        {
            Text = text,
            SelectionStart = cursor,
            SelectionEnd = cursor,
            OnFirstLine = true,
            OnLastLine = true,
        };

    private static readonly Key Enter = new Key.Enter();
    private static readonly Key Tab = new Key.Tab();
    private static readonly Key Up = new Key.Up();
    private static readonly Key Down = new Key.Down();
    private static readonly Key Left = new Key.Left();
    private static readonly Key Right = new Key.Right();
    private static readonly Key Backspace = new Key.Backspace();
    private static readonly Key Delete = new Key.Delete();
    private static readonly Key Escape = new Key.Escape();

    private static Key Letter(string letter) => new Key.Letter(letter);

    /// <summary>
    /// Presses a key in the editor of the item, as the app does: it hands the chord to the
    /// core, which decides what it means (ADR 052).
    /// </summary>
    private static bool Press(
        Shell shell,
        Key key,
        string itemId,
        EditorState editor,
        Held held = Held.None
    ) => shell.Pressed(new KeyPress.InEditor(itemId, Keyboard.Chord(key, held), editor));

    /// <summary>Presses a key while items are selected, as the app does.</summary>
    private static void Press(Shell shell, Key key, Held held = Held.None) =>
        _ = shell.Pressed(new KeyPress.InSelection(Keyboard.Chord(key, held)));

    // ページを開くと、その行が画面に出る。新しいページには、空の項目が1つある。
    [Fact]
    public void A_page_is_opened_and_its_rows_are_shown()
    {
        var shell = NewShell("/pages/winui-notes");

        main.WaitUntil("the page is shown", () => shell.View.Outline.Rows.Count > 0);

        Assert.Equal("winui-notes", Page(shell)?.Page.Name);
        Assert.Equal([""], Bodies(shell));
        main.WaitUntil("the title is that of the page", () => shell.Title == "winui-notes");
        Assert.Equal("/pages/winui-notes", shell.Url);
    }

    // 行の変化は、操作の列として知らされる。知らされた操作を順に当てると、シェルの行と同じになる（ADR 076）。
    [Fact]
    public void The_rows_are_told_as_changes_that_make_the_rows_of_the_shell()
    {
        var shell = NewShell("/pages/winui-notes");
        List<Row> told = [];
        shell.RowsChanged += changes => Apply(told, changes);
        main.WaitUntil("the page is shown", () => shell.View.Outline.Rows.Count > 0);

        var (first, _) = TwoItems(shell);
        shell.Typed("first, typed again", first);

        Assert.Equal(
            shell.View.Outline.Rows.Select(row => (row.Item.Id, row.Item.Body)),
            told.Select(row => (row.Item.Id, row.Item.Body))
        );
    }

    /// <summary>The rows after the changes, applied one after another as a list does.</summary>
    private static void Apply(List<Row> rows, IReadOnlyList<ListChange> changes)
    {
        foreach (var change in changes)
        {
            switch (change)
            {
                case ListChange.Insert insert:
                    rows.Insert((int)insert.At, insert.Item);
                    break;
                case ListChange.Remove remove:
                    rows.RemoveAt((int)remove.At);
                    break;
                case ListChange.Move move:
                    var moved = rows[(int)move.From];
                    rows.RemoveAt((int)move.From);
                    rows.Insert((int)move.To, moved);
                    break;
                case ListChange.Update update:
                    rows[(int)update.At] = update.Item;
                    break;
                case ListChange.Reset reset:
                    rows.Clear();
                    rows.AddRange(reset.Items);
                    break;
            }
        }
    }

    // 編集中にウィンドウを閉じると、閉じたあとでエディタがカーソルを失う。そのときの編集の終わりは、何もしない。
    [Fact]
    public void The_end_of_an_edit_after_the_window_is_closed_does_nothing()
    {
        var shell = ShellWithPage();
        var item = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(item);

        shell.Dispose();

        shell.FinishEditing(item);
    }

    // ウィンドウを閉じると、保存を待っていた本文が、閉じる前にサーバーに保存される（#766）。
    [Fact]
    public void Closing_the_window_saves_the_text_still_waiting_to_be_saved()
    {
        var shell = ShellWithPage();
        var item = shell.View.Outline.Rows[0].Item.Id;
        var pageId = Page(shell)!.Page.Id;
        shell.StartEditing(item);
        shell.Typed("typed before closing", item);

        shell.Dispose();

        Assert.Contains("typed before closing", server.Get($"/api/pages/{pageId}") ?? "");
    }

    // 閉じたウィンドウに、待っていたコアの要求が届いても、何もしない。
    [Fact]
    public void The_requests_that_reach_a_closed_window_do_nothing()
    {
        var shell = ShellWithPage();
        shell.Open("/pages/winui-other");
        main.WaitForWork();

        shell.Dispose();

        main.Turn();
    }

    // 何も指さないURLを開いても、何も変わらない。
    [Fact]
    public void A_url_that_names_nothing_changes_nothing()
    {
        var shell = ShellWithPage();
        var told = 0;
        shell.UrlChanged += () => told++;

        shell.Open("nothing");

        Assert.Equal("/pages/winui-notes", shell.Url);
        Assert.Equal(0, told);
    }

    // 戻ると、前に開いたページがまた開き、付箋紙が覚えるURLもそのページになる。進むと、戻る前のページに戻る（ADR 035）。
    [Fact]
    public void Back_and_forward_follow_the_pages_opened_and_the_url_follows()
    {
        var shell = NewShell("/pages/winui-first");
        main.WaitUntil("the first page is shown", () => Page(shell)?.Page.Name == "winui-first");
        Assert.False(shell.View.History.CanGoBack);
        shell.Open("/pages/winui-second");
        main.WaitUntil("the second page is shown", () => Page(shell)?.Page.Name == "winui-second");

        shell.GoBack();

        main.WaitUntil("the first page is shown again", () =>
            Page(shell)?.Page.Name == "winui-first"
        );
        Assert.Equal("/pages/winui-first", shell.Url);
        Assert.True(shell.View.History.CanGoForward);

        shell.GoForward();

        main.WaitUntil("the second page is shown again", () =>
            Page(shell)?.Page.Name == "winui-second"
        );
        Assert.Equal("/pages/winui-second", shell.Url);
    }

    // 項目に入力すると、少しあとにサーバーに保存される。
    [Fact]
    public void Typing_into_an_item_is_saved_on_the_server()
    {
        var shell = ShellWithPage();
        var item = shell.View.Outline.Rows[0].Item.Id;
        var pageId = Page(shell)!.Page.Id;

        shell.StartEditing(item);
        Assert.Equal(item, shell.View.Outline.FocusedItemId);
        shell.Typed("typed in WinUI 日本語", item);

        Assert.Equal("typed in WinUI 日本語", shell.View.Outline.Rows[0].Item.Body);
        main.WaitUntil("the server has the text", () =>
            server.Get($"/api/pages/{pageId}")?.Contains("typed in WinUI 日本語") ?? false
        );
    }

    // 開始記号を打ち終えると、コアが閉じる記号を補う（ADR 050、ADR 057）。
    [Fact]
    public void Typing_an_opening_closes_it()
    {
        var shell = ShellWithPage();
        var item = shell.View.Outline.Rows[0].Item.Id;

        shell.StartEditing(item);
        shell.Typed(">>", item, 2);
        shell.Typed(">>>", item, 3);

        Assert.Equal(">>>\n>>>", shell.View.Outline.Rows[0].Item.Body);
    }

    // サーバーの拡張が確認を求めると、質問が出る。「いいえ」では何も起きず、「はい」で拡張が先へ進む。
    [Fact]
    public void A_confirmation_of_an_extension_is_answered_with_yes_or_no()
    {
        var shell = ShellWithPage();
        var pageId = Page(shell)!.Page.Id;
        HearsTheServer(shell);

        Tag(pageId, "ask-first");
        main.WaitUntil("the shell is asked to confirm", () => shell.Question is not null);
        Assert.Equal("Insert into winui-notes?", shell.Question?.Message);
        Assert.Equal(QuestionKind.Confirm, shell.Question?.Kind);
        shell.Answer(false);
        Assert.Null(shell.Question);
        Tag(pageId, "ask-second");
        main.WaitUntil("the shell is asked again", () => shell.Question is not null);
        shell.Answer(true);

        // Only the yes added an item: the no added none before it. The item of the extension
        // takes the place of the empty item of the new page, as the server does with an item
        // placed last on a page whose only item is empty
        main.WaitUntil("the item of the extension is shown", () =>
            Bodies(shell).Contains("confirmed")
        );
        Assert.Equal(["confirmed"], Bodies(shell));
    }

    // Enterで項目を分けると、コアが新しい項目にカーソルを置くように頼む。
    [Fact]
    public void Enter_asks_the_cursor_into_the_new_item()
    {
        var shell = ShellWithPage();

        var (_, second) = TwoItems(shell);

        main.WaitUntil("the cursor is asked into the new item", () =>
            shell.Focus?.ItemId == second
        );
    }

    // 項目のメニューの「Paste as Transclusion」では、コアがクリップボードを求め、シェルがその文字列で答える。項目のIDなら、その項目はトランスクルージョンになる。
    [Fact]
    public void Paste_as_transclusion_makes_the_item_show_the_item_whose_id_is_on_the_clipboard()
    {
        var shell = ShellWithPage();
        var (first, second) = TwoItems(shell);
        onClipboard = first;

        shell.PasteAsTransclusion(second);

        main.WaitUntil("the second item shows the first", () =>
            shell.View.Outline.Rows.Any(row =>
                row.Item.Id == second && row.Item.TranscludeItemId == first
            )
        );
    }

    // ほかで表示されている項目を選んで消すときは、元に戻しにくい操作として確認を求められる。
    [Fact]
    public void Deleting_a_transcluded_item_is_asked_as_hard_to_undo()
    {
        var shell = ShellWithPage();
        HearsTheServer(shell);
        var (first, second) = TwoItems(shell);
        // Enter left the cursor in the second item. While any item is edited, the core does not
        // read again a page that another client changed. It reads it once the edit ends
        shell.FinishEditing(second);
        // Another client, such as the web frontend, shows the first item in the second
        Assert.True(
            server.Send(
                HttpMethod.Post,
                $"/api/items/{second}/transclude",
                $$"""{"sourceItemId":"{{first}}"}"""
            )
        );
        main.WaitUntil("the second item shows the first", () =>
            shell.View.Outline.Rows.Any(row =>
                row.Item.Id == second && row.Item.TranscludeItemId == first
            )
        );

        Assert.True(Press(shell, Up, first, Editor("first", 0), Held.Shift));
        Press(shell, Delete);

        main.WaitUntil("the shell is asked to confirm", () => shell.Question is not null);
        Assert.Equal(QuestionKind.ConfirmDestructive, shell.Question?.Kind);
    }

    // 表示した文字に触れると、その項目の編集に入り、カーソルは触れた文字のMarkdownでの位置に置かれる（ADR 043）。
    [Fact]
    public void A_touch_on_the_shown_text_puts_the_cursor_at_its_place_in_the_markdown()
    {
        var shell = ShellWithPage();
        var first = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(first);
        shell.Typed("**bold** end", first);
        shell.FinishEditing(first);
        main.WaitUntil("the edit is over", () => !shell.View.Outline.Rows[0].IsEditing);

        // The shown text is "bold end": the touch is after "bol"
        shell.Touched(first, 3);

        main.WaitUntil("the item is edited", () => shell.View.Outline.Rows[0].IsEditing);
        Assert.Equal(first, shell.View.Outline.FocusRequest?.ItemId);
        Assert.Equal(5ul, shell.View.Outline.FocusRequest?.Cursor);
    }

    // アウトラインにとって意味のないキーは、エディタに任される。
    [Fact]
    public void A_key_that_means_nothing_to_the_outline_is_left_to_the_editor()
    {
        var shell = ShellWithPage();
        var item = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(item);

        Assert.False(Press(shell, Letter("a"), item, Editor("", 0)));
    }

    // Enterは、カーソルの位置で項目を分ける。カーソルは、新しい項目の先頭に移る。
    [Fact]
    public void Enter_splits_the_item_at_the_cursor()
    {
        var shell = ShellWithPage();
        var first = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(first);
        shell.Typed("日本語second", first);

        Assert.True(Press(shell, Enter, first, Editor("日本語second", 3)));

        main.WaitUntil("the item is split", () => Bodies(shell) is ["日本語", "second"]);
        var second = shell.View.Outline.Rows[1].Item.Id;
        main.WaitUntil("the cursor is asked into the new item", () =>
            shell.Focus?.ItemId == second
        );
    }

    // Tabは項目を字下げし、Shift+Tabは字上げする。
    [Fact]
    public void Tab_indents_and_Shift_Tab_outdents()
    {
        var shell = ShellWithPage();
        var (_, second) = TwoItems(shell);
        var level = shell.View.Outline.Rows[1].Item.Level;
        Assert.Equal(shell.View.Outline.Rows[0].Item.Level, level);

        Assert.True(Press(shell, Tab, second, Editor("", 0)));
        Assert.Equal(level + 1, shell.View.Outline.Rows[1].Item.Level);
        Assert.True(Press(shell, Tab, second, Editor("", 0), Held.Shift));
        Assert.Equal(level, shell.View.Outline.Rows[1].Item.Level);
    }

    // 最初の行での上矢印は、前の項目へ移る。カーソルの位置は保たれる。
    [Fact]
    public void Up_on_the_first_line_goes_to_the_item_before()
    {
        var shell = ShellWithPage();
        var (first, second) = TwoItems(shell);
        shell.Typed("second", second);

        Assert.True(Press(shell, Up, second, Editor("second", 2)));

        main.WaitUntil("the cursor is asked into the first item", () =>
            shell.View.Outline.FocusRequest?.ItemId == first
        );
        Assert.Equal(2UL, shell.View.Outline.FocusRequest?.Cursor);
    }

    // 最後の行での下矢印は、次の項目へ移る。
    [Fact]
    public void Down_on_the_last_line_goes_to_the_item_after()
    {
        var shell = ShellWithPage();
        var (first, second) = TwoItems(shell);
        shell.StartEditing(first);

        Assert.True(Press(shell, Down, first, Editor("first", 1)));

        main.WaitUntil("the cursor is asked into the second item", () =>
            shell.View.Outline.FocusRequest?.ItemId == second
        );
    }

    // 先頭での左矢印は、前の項目の末尾へ移る。末尾での右矢印は、次の項目の先頭へ移る。
    [Fact]
    public void Left_at_the_start_and_Right_at_the_end_leave_the_item()
    {
        var shell = ShellWithPage();
        var (first, second) = TwoItems(shell);

        Assert.True(Press(shell, Left, second, Editor("", 0)));
        main.WaitUntil("the cursor is asked to the end of the first item", () =>
            shell.View.Outline.FocusRequest?.ItemId == first
        );
        var asked = shell.View.Outline.FocusRequest?.Cursor;
        Assert.NotNull(asked);
        // The core asks for the end, which the editor finds in its text
        Assert.Equal(5UL, PetaCore.PetaNative.Utf16OfChars("first", asked.Value));

        Assert.True(Press(shell, Right, first, Editor("first", 5)));
        main.WaitUntil("the cursor is asked to the start of the second item", () =>
            shell.View.Outline.FocusRequest?.ItemId == second
        );
        Assert.Equal(0UL, shell.View.Outline.FocusRequest?.Cursor);
    }

    // 先頭でのBackspaceは、項目を前の項目につなぐ。
    [Fact]
    public void Backspace_at_the_start_joins_the_item_to_the_one_before()
    {
        var shell = ShellWithPage();
        var (_, second) = TwoItems(shell);
        shell.Typed("second", second);

        Assert.True(Press(shell, Backspace, second, Editor("second", 0)));

        main.WaitUntil("the items are joined", () => Bodies(shell) is ["firstsecond"]);
    }

    // 末尾でのDeleteは、次の項目をつなぐ。
    [Fact]
    public void Delete_at_the_end_joins_the_next_item()
    {
        var shell = ShellWithPage();
        var (first, second) = TwoItems(shell);
        shell.Typed("second", second);
        shell.StartEditing(first);

        Assert.True(Press(shell, Delete, first, Editor("first", 5)));

        main.WaitUntil("the items are joined", () => Bodies(shell) is ["firstsecond"]);
    }

    // タスクの切り替えは、項目をタスクにし、押すたびに状態を進める。
    [Fact]
    public void The_task_key_makes_the_item_a_task_and_steps_its_state()
    {
        var shell = ShellWithPage();
        var first = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(first);
        shell.Typed("to do", first);
        Assert.Null(shell.View.Outline.Rows[0].Task);

        Assert.True(Press(shell, Enter, first, Editor("to do", 0), Held.Control));
        main.WaitUntil("the item is a task", () =>
            shell.View.Outline.Rows[0].Task == TaskState.NotStarted
        );
        Assert.True(Press(shell, Enter, first, Editor("to do", 0), Held.Control));
        main.WaitUntil("the task is running", () =>
            shell.View.Outline.Rows[0].Task == TaskState.Running
        );
    }

    // 後に回した編集の終わりは、呼んだその場では画面を変えず、メインスレッドの次の番に編集を終える。
    // 一覧が行を並べ替えている最中に、新しい画面で割り込まないため。
    [Fact]
    public void An_edit_finished_later_does_not_change_the_view_during_the_call()
    {
        var shell = ShellWithPage();
        var first = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(first);
        main.WaitUntil("the item is edited", () => shell.View.Outline.Rows[0].IsEditing);
        var changed = false;
        shell.ViewChanged += () => changed = true;

        shell.FinishEditingLater(first);

        Assert.False(changed);
        main.WaitUntil("the edit is over", () => !shell.View.Outline.Rows[0].IsEditing);
    }

    // タスクの印をクリックすると、タスクの状態が進む。項目を編集していなくてもよい。
    [Fact]
    public void A_click_on_the_mark_of_a_task_steps_its_state()
    {
        var shell = ShellWithPage();
        var first = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(first);
        shell.Typed("to do", first);
        Assert.True(Press(shell, Enter, first, Editor("to do", 0), Held.Control));
        main.WaitUntil("the item is a task", () =>
            shell.View.Outline.Rows[0].Task == TaskState.NotStarted
        );
        shell.FinishEditing(first);

        shell.ClickedTask(first);
        main.WaitUntil("the task is running", () =>
            shell.View.Outline.Rows[0].Task == TaskState.Running
        );
        shell.ClickedTask(first);
        main.WaitUntil("the task is done", () => shell.View.Outline.Rows[0].Task == TaskState.Done);
    }

    // 折りたたむキーは子の項目を隠し、開くキーは子の項目を表示する。切り替えではない。
    [Fact]
    public void The_fold_key_hides_the_children_and_the_unfold_key_shows_them()
    {
        SkipWithoutTheKeysOfWindows();
        var shell = ShellWithPage();
        var (first, second) = TwoItems(shell);
        Assert.True(Press(shell, Tab, second, Editor("", 0)));

        // The children are shown already
        Assert.True(Press(shell, Down, first, Editor("first", 0), Held.Control));
        Assert.False(shell.View.Outline.Rows[0].IsCollapsed);
        Assert.True(Press(shell, Up, first, Editor("first", 0), Held.Control));
        main.WaitUntil("the child is hidden", () => shell.View.Outline.Rows.Count == 1);
        Assert.True(shell.View.Outline.Rows[0].IsCollapsed);
        // The children are hidden already
        Assert.True(Press(shell, Up, first, Editor("first", 0), Held.Control));
        Assert.True(shell.View.Outline.Rows[0].IsCollapsed);
        Assert.True(Press(shell, Down, first, Editor("first", 0), Held.Control));
        main.WaitUntil("the child is shown again", () => shell.View.Outline.Rows.Count == 2);
        Assert.False(shell.View.Outline.Rows[0].IsCollapsed);
    }

    // 子のない項目には、折りたたむものがない。
    [Fact]
    public void An_item_without_children_is_not_folded()
    {
        SkipWithoutTheKeysOfWindows();
        var shell = ShellWithPage();
        var first = shell.View.Outline.Rows[0].Item.Id;

        Assert.True(Press(shell, Up, first, Editor("", 0), Held.Control));
        Assert.False(shell.View.Outline.Rows[0].IsCollapsed);
    }

    // 項目を下へ動かすと、次の項目と入れ替わる。上へ動かすと、戻る。
    [Fact]
    public void An_item_is_moved_down_and_up()
    {
        var shell = ShellWithPage();
        var (first, second) = TwoItems(shell);
        shell.Typed("second", second);
        shell.StartEditing(first);

        Assert.True(Press(shell, Down, first, Editor("first", 0), Held.Shift | Held.Alt));
        main.WaitUntil("the first item is below", () => Bodies(shell) is ["second", "first"]);
        Assert.True(Press(shell, Up, first, Editor("first", 0), Held.Shift | Held.Alt));
        main.WaitUntil("the first item is on top again", () =>
            Bodies(shell) is ["first", "second"]
        );
    }

    // 項目を右へ動かすと字下げになり、左へ動かすと字上げになる。
    [Fact]
    public void An_item_is_moved_in_and_out()
    {
        var shell = ShellWithPage();
        var (_, second) = TwoItems(shell);
        var level = shell.View.Outline.Rows[1].Item.Level;

        Assert.True(Press(shell, Right, second, Editor("", 0), Held.Shift | Held.Alt));
        Assert.Equal(level + 1, shell.View.Outline.Rows[1].Item.Level);
        Assert.True(Press(shell, Left, second, Editor("", 0), Held.Shift | Held.Alt));
        Assert.Equal(level, shell.View.Outline.Rows[1].Item.Level);
    }

    // 元に戻すと、直前の操作が取り消される。やり直すと、もう一度行われる。
    [Fact]
    public void Undo_takes_back_the_last_change_and_redo_makes_it_again()
    {
        var shell = ShellWithPage();
        var (_, second) = TwoItems(shell);
        var level = shell.View.Outline.Rows[1].Item.Level;
        Assert.True(Press(shell, Tab, second, Editor("", 0)));
        Assert.Equal(level + 1, shell.View.Outline.Rows[1].Item.Level);

        Assert.True(Press(shell, Letter("z"), second, Editor("", 0), Held.Control));
        main.WaitUntil("the indent is taken back", () =>
            shell.View.Outline.Rows[1].Item.Level == level
        );
        Assert.True(Press(shell, Letter("z"), second, Editor("", 0), Held.Control | Held.Shift));
        main.WaitUntil("the indent is made again", () =>
            shell.View.Outline.Rows[1].Item.Level == level + 1
        );
    }

    /// <summary>Two items, "first" and "second", with the second one selected from its editor.</summary>
    private void SecondOfTwoSelected(Shell shell)
    {
        var (_, second) = TwoItems(shell);
        shell.Typed("second", second);
        Assert.True(Press(shell, Down, second, Editor("second", 0), Held.Shift));
    }

    // 項目の文字から次の項目までマウスでドラッグすると、2つの項目が選ばれる。
    [Fact]
    public void A_drag_from_an_item_to_the_next_selects_both()
    {
        var shell = ShellWithPage();
        var (first, second) = TwoItems(shell);

        shell.StartEditing(first);
        shell.Dragged(new DragMove.Pressed(first, 0, 20));
        shell.Dragged(new DragMove.Moved(200, second));

        main.WaitUntil("both items are selected", () => Selected(shell).Count == 2);
        Assert.True(shell.View.Outline.SelectionMode);
    }

    // 最初の行でのShift+上は、その項目を選ぶ。
    [Fact]
    public void Shift_with_an_arrow_on_the_edge_line_selects_the_item()
    {
        var shell = ShellWithPage();
        var (first, _) = TwoItems(shell);

        Assert.True(Press(shell, Up, first, Editor("first", 0), Held.Shift));

        Assert.True(shell.View.Outline.SelectionMode);
        Assert.Equal(["first"], Selected(shell));
    }

    // 選んでいるあいだのShift+上下は、選択を広げる。Escapeは、選択をやめる。
    [Fact]
    public void The_selection_grows_and_Escape_ends_it()
    {
        var shell = ShellWithPage();
        SecondOfTwoSelected(shell);

        Press(shell, Up, Held.Shift);
        Assert.Equal(["first", "second"], Selected(shell));
        Press(shell, Escape);

        Assert.False(shell.View.Outline.SelectionMode);
        Assert.Empty(Selected(shell));
        Assert.Equal(["first", "second"], Bodies(shell));
    }

    // 選んでいるあいだに項目の外をクリックすると、選択が消える。項目は消えない。
    [Fact]
    public void A_click_outside_the_items_ends_the_selection()
    {
        var shell = ShellWithPage();
        SecondOfTwoSelected(shell);

        shell.ClickedOutsideItems();

        Assert.False(shell.View.Outline.SelectionMode);
        Assert.Empty(Selected(shell));
        Assert.Equal(["first", "second"], Bodies(shell));
    }

    // 選んだ項目を消せる。
    [Fact]
    public void The_selected_items_are_deleted()
    {
        var shell = ShellWithPage();
        SecondOfTwoSelected(shell);

        Press(shell, Delete);

        main.WaitUntil("the second item is gone", () => Bodies(shell) is ["first"]);
        Assert.False(shell.View.Outline.SelectionMode);
    }

    // 選んだ項目を上へ動かせる。
    [Fact]
    public void The_selected_items_are_moved()
    {
        var shell = ShellWithPage();
        SecondOfTwoSelected(shell);

        Press(shell, Up, Held.Shift | Held.Alt);

        main.WaitUntil("the second item is on top", () => Bodies(shell) is ["second", "first"]);
        Assert.Equal(["second"], Selected(shell));
    }

    // 選んだ項目をコピーすると、Markdownがクリップボードに載る。同じ内容のHTMLも載る。
    [Fact]
    public void Copy_puts_the_selected_items_on_the_clipboard()
    {
        SkipWithoutTheKeysOfWindows();
        var shell = ShellWithPage();
        SecondOfTwoSelected(shell);
        Press(shell, Up, Held.Shift);

        Press(shell, Letter("c"), Held.Control);

        main.WaitUntil("the clipboard has the items", () => copied is not null);
        Assert.Equal("- first\n- second", copied);
        Assert.Contains("<li>first</li>", copiedHtml);
        Assert.Contains("<li>second</li>", copiedHtml);
        Assert.Equal(["first", "second"], Bodies(shell));
    }

    // Markdownのリストを貼り付けると、項目として貼り付く。
    [Fact]
    public void A_pasted_list_becomes_items()
    {
        var shell = ShellWithPage();
        var item = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(item);

        shell.Paste(item, Editor("", 0), "- a\n- b", "", raw: false);

        main.WaitUntil("the list is two items", () => Bodies(shell) is ["a", "b"]);
    }

    // そのままの貼り付け（Ctrl+Shift+V）では、リストも1つの項目にそのまま入る。
    [Fact]
    public void A_raw_paste_keeps_a_list_in_one_item()
    {
        var shell = ShellWithPage();
        var item = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(item);

        shell.Paste(item, Editor("", 0), "- a\n- b", "", raw: true);

        main.WaitUntil("the list is in the item", () => Bodies(shell) is ["- a\n- b"]);
    }

    // 画像のファイルを貼ると、サーバーのassetsに保存され、項目にそのリンクが入る。ファイルは保存のあとに消える（ADR 071）。
    [Fact]
    public void A_pasted_image_file_is_saved_in_the_assets_and_linked_in_the_item()
    {
        var shell = ShellWithPage();
        var item = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(item);
        var image = Path.Combine(Directory.CreateTempSubdirectory().FullName, "sample.png");
        File.WriteAllBytes(image, [1, 2, 3]);

        shell.PasteImage(item, Editor("", 0), image);

        main.WaitUntil("the item links the image", () =>
            Bodies(shell) is [var body]
            && body.StartsWith("![sample.png](../assets/", StringComparison.Ordinal)
        );
        main.WaitUntil("the file is deleted", () => !File.Exists(image));
    }

    // 描かれた項目に落とした画像は、本文の末尾にリンクされる。
    [Fact]
    public void An_image_dropped_on_a_drawn_item_is_linked_at_the_end_of_its_body()
    {
        var shell = ShellWithPage();
        var item = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(item);
        shell.Typed("abc", item);
        shell.FinishEditing(item);
        main.WaitUntil("the item has its text", () => Bodies(shell) is ["abc"]);
        var image = Path.Combine(Directory.CreateTempSubdirectory().FullName, "sample.png");
        File.WriteAllBytes(image, [1, 2, 3]);

        shell.DropImage(item, [image]);

        main.WaitUntil("the image is linked after the text", () =>
            Bodies(shell) is [var body]
            && body.StartsWith("abc![sample.png](../assets/", StringComparison.Ordinal)
        );
    }

    // 編集中のエディタに落とした画像は、落とした文字の位置にリンクされる。
    [Fact]
    public void An_image_dropped_on_an_editor_is_linked_where_it_was_dropped()
    {
        var shell = ShellWithPage();
        var item = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(item);
        shell.Typed("abcd", item);
        var image = Path.Combine(Directory.CreateTempSubdirectory().FullName, "sample.png");
        File.WriteAllBytes(image, [1, 2, 3]);

        shell.DropImage(item, Editor("abcd", 2), [image]);

        main.WaitUntil("the image is linked between b and c", () =>
            Bodies(shell) is [var body]
            && body.StartsWith("ab![sample.png](../assets/", StringComparison.Ordinal)
            && body.EndsWith(")cd", StringComparison.Ordinal)
        );
    }

    // 落としたファイルの中で、最初の画像だけがリンクされる。画像でないファイルは飛ばされる。
    [Fact]
    public void Only_the_first_image_of_the_dropped_files_is_linked()
    {
        var shell = ShellWithPage();
        var item = shell.View.Outline.Rows[0].Item.Id;
        var folder = Directory.CreateTempSubdirectory().FullName;
        string[] files = ["notes.txt", "first.png", "second.jpg"];
        foreach (var file in files)
            File.WriteAllBytes(Path.Combine(folder, file), [1, 2, 3]);

        var taken = shell.DropImage(item, [.. files.Select(file => Path.Combine(folder, file))]);

        Assert.True(taken);
        main.WaitUntil("the first image is linked", () =>
            Bodies(shell) is [var body]
            && body.StartsWith("![first.png](../assets/", StringComparison.Ordinal)
            && !body.Contains("second", StringComparison.Ordinal)
        );
    }

    // 落としたファイルに画像がなければ、何もリンクされない。
    [Fact]
    public void Dropped_files_without_an_image_are_not_taken()
    {
        var shell = ShellWithPage();
        var item = shell.View.Outline.Rows[0].Item.Id;
        var notes = Path.Combine(Directory.CreateTempSubdirectory().FullName, "notes.txt");
        File.WriteAllBytes(notes, [1, 2, 3]);

        var taken = shell.DropImage(item, [notes]);

        Assert.False(taken);
        Assert.True(File.Exists(notes));
    }

    // 落とした元のファイルは、リンクされたあとも消えずに残る。コアが消すのは、シェルが作った写しである。
    [Fact]
    public void A_dropped_file_stays_where_it_was()
    {
        var shell = ShellWithPage();
        var item = shell.View.Outline.Rows[0].Item.Id;
        var image = Path.Combine(Directory.CreateTempSubdirectory().FullName, "sample.png");
        File.WriteAllBytes(image, [1, 2, 3]);

        shell.DropImage(item, [image]);

        main.WaitUntil("the image is linked", () => Bodies(shell) is [var body] && body.Length > 0);
        Assert.Equal([1, 2, 3], File.ReadAllBytes(image));
    }

    /// <summary>The level and the body of each item, as "level body".</summary>
    private static List<string> LevelsAndBodies(Shell shell) =>
        [.. shell.View.Outline.Rows.Select(row => $"{row.Item.Level} {row.Item.Body}")];

    /// <summary>
    /// Three items: "parent", its child "child", and "third" after them, with the cursor in
    /// "third". Returns the id of "third".
    /// </summary>
    private string ParentChildAndThird(Shell shell)
    {
        var parent = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(parent);
        shell.Typed("parent", parent);
        Assert.True(Press(shell, Enter, parent, Editor("parent", 6)));
        main.WaitUntil("there are two items", () => shell.View.Outline.Rows.Count == 2);
        var child = shell.View.Outline.Rows[1].Item.Id;
        shell.Typed("child", child);
        Assert.True(Press(shell, Tab, child, Editor("child", 5)));
        Assert.True(Press(shell, Enter, child, Editor("child", 5)));
        main.WaitUntil("there are three items", () => shell.View.Outline.Rows.Count == 3);
        var third = shell.View.Outline.Rows[2].Item.Id;
        Assert.True(Press(shell, Tab, third, Editor("", 0), Held.Shift));
        shell.Typed("third", third);
        Assert.Equal(["1 parent", "2 child", "1 third"], LevelsAndBodies(shell));
        return third;
    }

    // 親と子を選んでコピーし、別の項目の終わりに貼り付けると、親と子の形のまま戻る。
    // OutlineScreenTests checks the same with the real keys and the real clipboard. This one
    // stays: the screen tests send real input, so they run only when the developer is at the
    // PC. This one runs with "dotnet test" at any time.
    [Fact]
    public void A_parent_and_its_child_copied_and_pasted_at_the_end_of_another_item_come_back_in_their_shape()
    {
        SkipWithoutTheKeysOfWindows();
        var shell = ShellWithPage();
        var third = ParentChildAndThird(shell);
        var parent = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(parent);
        Assert.True(Press(shell, Up, parent, Editor("parent", 0), Held.Shift));
        Assert.Equal(["parent", "child"], Selected(shell));
        Press(shell, Letter("c"), Held.Control);
        main.WaitUntil("the clipboard has the items", () => copied is not null);
        shell.StartEditing(third, "third".Length);

        shell.Paste(third, Editor("third", 5), copied!, "", raw: false);

        main.WaitUntil("the parent and its child are pasted", () =>
            LevelsAndBodies(shell) is ["1 parent", "2 child", "1 thirdparent", "2 child"]
        );
    }

    // 絵文字のあとのカーソルに貼る。カーソルは、UTF-16では2つ分進んでいる。
    [Fact]
    public void A_paste_behind_an_emoji_goes_in_there()
    {
        var shell = ShellWithPage();
        var item = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(item);
        shell.Typed("a😀b", item);

        shell.Paste(item, Editor("a😀b", 3), "X", "", raw: false);

        main.WaitUntil("the text is behind the emoji", () => Bodies(shell) is ["a😀Xb"]);
    }

    /// <summary>A shell on the page "winui-notes", with a second page "apple-pie" on the server.</summary>
    private Shell ShellWithTwoPages()
    {
        var shell = ShellWithPage();
        shell.Open("/pages/apple-pie");
        main.WaitUntil("the second page is shown", () => shell.Title == "apple-pie");
        shell.Open("/pages/winui-notes");
        main.WaitUntil("the first page is shown again", () => shell.Title == "winui-notes");
        return shell;
    }

    private static List<string> PagesFound(Shell shell) =>
        [
            .. shell
                .View.Search.Results.Where(entry => entry.Kind == SearchKind.Page)
                .Select(entry => entry.Title),
        ];

    // 検索欄に入力すると、少しあとにページの候補が出る。候補を選ぶと、そのページが開き、検索欄は空になる。
    [Fact]
    public void A_search_finds_a_page_and_picking_it_opens_the_page()
    {
        var shell = ShellWithTwoPages();

        shell.Search("apple");
        Assert.Equal("apple", shell.View.Search.Query);
        main.WaitUntil("the page is found", () => PagesFound(shell) is ["apple-pie"]);
        Assert.True(shell.View.Search.ShowDropdown);
        shell.Pick(shell.View.Search.Results[0].Position);

        main.WaitUntil("the page found is shown", () => shell.Title == "apple-pie");
        Assert.Equal("", shell.View.Search.Query);
        Assert.False(shell.View.Search.ShowDropdown);
        Assert.Equal("/pages/apple-pie", shell.Url);
    }

    // 検索の候補は、下矢印で選び、Enterで開く。
    [Fact]
    public void A_found_page_is_chosen_with_Down_and_opened_with_Enter()
    {
        var shell = ShellWithTwoPages();
        shell.Search("apple");
        main.WaitUntil("the page is found", () => PagesFound(shell) is ["apple-pie"]);

        shell.Search(SearchKey.Down);
        Assert.Equal(0UL, shell.View.Search.SelectedIndex);
        shell.Search(SearchKey.Enter);

        main.WaitUntil("the page found is shown", () => shell.Title == "apple-pie");
    }

    // Escapeは、検索の候補を隠す。
    [Fact]
    public void Escape_hides_what_was_found()
    {
        var shell = ShellWithTwoPages();
        shell.Search("apple");
        main.WaitUntil("the page is found", () => PagesFound(shell) is ["apple-pie"]);

        shell.Search(SearchKey.Escape);

        Assert.False(shell.View.Search.ShowDropdown);
        Assert.Equal("winui-notes", shell.Title);
    }

    // ページを指定しないシェルは、何も開かない。検索はできて、選んだページが開く。開いた場所は、シェルが覚えている。
    [Fact]
    public void A_shell_without_a_page_opens_nothing_until_a_page_is_found()
    {
        ShellWithPage();
        var shell = NewShell(null);
        Assert.Null(shell.Url);
        Assert.Empty(shell.View.Outline.Rows);

        shell.Search("winui");
        main.WaitUntil("the page is found", () => PagesFound(shell) is ["winui-notes"]);
        shell.Pick(shell.View.Search.Results[0].Position);

        main.WaitUntil("the page found is shown", () => shell.Title == "winui-notes");
        Assert.Equal("/pages/winui-notes", shell.Url);
    }

    // ページへのリンクをキーを押さずにクリックすると、その付箋紙でページが開く。Altだけを押しても同じである。ブラウザには渡らない（ADR 083）。
    [Theory]
    [InlineData(Held.None)]
    [InlineData(Held.Alt)]
    public void A_link_to_a_page_clicked_without_a_key_opens_the_page_in_the_sticky(Held held)
    {
        var shell = ShellWithPage();

        shell.Follow(new RichLink.Page("/pages/apple-pie"), held);

        main.WaitUntil("the linked page is shown", () => shell.Title == "apple-pie");
        Assert.Equal("/pages/apple-pie", shell.Url);
        Assert.Null(browsed);
    }

    // ページへのリンクをCtrlクリックすると、そのページを開く新しい付箋紙を頼む。この付箋紙のページは変わらず、ブラウザにも渡らない。
    [Fact]
    public void A_link_to_a_page_clicked_with_control_asks_for_a_new_sticky_with_the_page()
    {
        var shell = ShellWithPage();
        string? asked = null;
        shell.NewStickyAsked += url => asked = url;

        shell.Follow(new RichLink.Page("/pages/apple-pie"), Held.Control);

        main.WaitUntil("a new sticky is asked for", () => asked is not null);
        Assert.Equal("/pages/apple-pie", asked);
        Assert.Equal("/pages/winui-notes", shell.Url);
        Assert.Null(browsed);
    }

    // Shiftクリックしたページへのリンクと、資産のファイルとWebへのリンクは、ブラウザに渡る。ページはWebのフロントエンドで、資産のファイルはジャーナルサーバーが出すもので開く。付箋紙のページは変わらない。
    [Fact]
    public void A_link_that_goes_to_the_browser_leaves_the_sticky_with_its_page()
    {
        var shell = ShellWithPage();
        void Opens(RichLink link, Held held, string address)
        {
            shell.Follow(link, held);
            main.WaitUntil($"{address} opens in the browser", () =>
                browsed?.AbsoluteUri == address
            );
        }

        Opens(
            new RichLink.Page("/pages/linked/idea"),
            Held.Shift,
            $"{server.Address}/pages/linked/idea"
        );
        Opens(
            new RichLink.Page("/pages/メモ/案 1#item"),
            Held.Shift | Held.Control,
            $"{server.Address}/pages/%E3%83%A1%E3%83%A2/%E6%A1%88%201#item"
        );
        Opens(
            new RichLink.File("assets/report a.pdf"),
            Held.None,
            $"{server.Address}/assets/report%20a.pdf"
        );
        Opens(new RichLink.Web("https://example.com/a"), Held.Control, "https://example.com/a");
        Assert.Equal("/pages/winui-notes", shell.Url);
    }

    /// <summary>
    /// Makes the journal of a day long ago, with one item, and waits until the server has it.
    /// </summary>
    private void JournalOfADayLongAgo()
    {
        var old = NewShell("/pages/2020-01-02");
        main.WaitUntil("the old journal is shown", () => old.View.Outline.Rows.Count > 0);
        var pageId = Journal(old)!.PageId;
        var item = old.View.Outline.Rows[0].Item.Id;
        old.StartEditing(item);
        old.Typed("written long ago", item);
        main.WaitUntil("the old journal is saved", () =>
            server.Get($"/api/pages/{pageId}")?.Contains("written long ago") ?? false
        );
    }

    // 今日のジャーナルのタイトルは、今日の日付に「(Today)」を付けたものである。
    [Fact]
    public void The_title_of_the_journal_of_today_is_its_date_marked_as_today()
    {
        var shell = NewShell("/today");
        main.WaitUntil("the journal is shown", () => shell.View.Outline.Rows.Count > 0);

        var date = Journal(shell)!.DisplayDate;
        Assert.NotEqual("", date);
        Assert.Equal($"{date} (Today)", shell.Title);
    }

    // 日付が変わったかもしれないと知らせても、同じ日のうちは、今日のジャーナルは開き直されない。編集中の項目は、編集のまま残る。
    [Fact]
    public void Telling_that_the_day_may_have_changed_on_the_same_day_keeps_the_journal_of_today()
    {
        var shell = NewShell("/today");
        main.WaitUntil("the journal is shown", () => shell.View.Outline.Rows.Count > 0);
        var item = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(item);
        shell.Typed("draft", item);
        var title = shell.Title;

        shell.DayMayHaveChanged();

        main.WaitUntil("the item is still edited", () =>
            shell.View.Outline.Rows[0].IsEditing && shell.View.Outline.Rows[0].Item.Body == "draft"
        );
        Assert.Equal(title, shell.Title);
    }

    // 今日でない日のジャーナルのタイトルは、その日付だけである。
    [Fact]
    public void The_title_of_the_journal_of_another_day_is_its_date_alone()
    {
        var shell = NewShell("/pages/2020-01-02");
        main.WaitUntil("the journal is shown", () => shell.View.Outline.Rows.Count > 0);

        Assert.Equal("2020年1月2日 木曜日", shell.Title);
    }

    // 今日のジャーナルを前の日までスクロールすると、タイトルはその日の日付になり、「(Today)」は付かない。今日まで戻ると、また付く。
    [Fact]
    public void Scrolling_the_journal_to_an_earlier_day_shows_that_date_in_the_title()
    {
        JournalOfADayLongAgo();
        var shell = NewShell("/today");
        main.WaitUntil("the earlier day is shown under today", () =>
            shell.View.Outline.Rows.Any(row => row.DateHeader is not null)
        );
        var earlier = shell.View.Outline.Rows.First(row => row.DateHeader is not null);
        var today = Journal(shell)!.DisplayDate;

        shell.Scrolled((int)earlier.Position, 0, (int)earlier.Position);

        Assert.Equal("2020年1月2日 木曜日", earlier.DateHeader);
        Assert.Equal("2020年1月2日 木曜日", shell.Title);

        shell.Scrolled(0, 0, (int)earlier.Position);

        Assert.Equal($"{today} (Today)", shell.Title);
    }

    // タイトルが変わると知らされる。同じタイトルのままなら、知らされない。
    [Fact]
    public void A_new_title_is_told_once()
    {
        var shell = NewShell(null);
        var told = 0;
        shell.TitleChanged += () => told++;

        shell.Open("/pages/winui-notes");
        main.WaitUntil("the page is shown", () => shell.View.Outline.Rows.Count > 0);
        main.WaitUntil("the title is that of the page", () => shell.Title == "winui-notes");
        var first = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(first);
        shell.Typed("text", first);

        Assert.Equal(1, told);
    }
}
