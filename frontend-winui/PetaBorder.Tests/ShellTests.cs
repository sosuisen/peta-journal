using PetaTypes;

namespace PetaBorder.Tests;

/// <summary>
/// The shell against a real journal server. These are skipped without <c>PETA_E2E_SERVER</c>.
/// </summary>
public sealed class ShellTests : IDisposable
{
    private readonly JournalServer server = new(18111);
    private readonly MainThread main = new();
    private readonly List<Shell> shells = [];

    /// <summary>What the clipboard holds, for the core to read.</summary>
    private string? onClipboard;

    /// <summary>What the shell put on the clipboard.</summary>
    private string? copied;

    /// <summary>The HTML that the shell put on the clipboard with the text.</summary>
    private string? copiedHtml;

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
            _ => { }
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
        shell.Send(new Event.EnterPressed(first, 5, 0));
        main.WaitUntil("there are two items", () => shell.View.Outline.Rows.Count == 2);
        return (first, shell.View.Outline.Rows[1].Item.Id);
    }

    /// <summary>The page on screen, once it is there.</summary>
    private static PageView? Page(Shell shell) => (shell.View.Screen as ScreenView.Page)?.Value;

    /// <summary>The journal on screen, once it is there.</summary>
    private static JournalView? Journal(Shell shell) =>
        (shell.View.Screen as ScreenView.Journal)?.Value;

    private static List<string> Bodies(Shell shell) =>
        [.. shell.View.Outline.Rows.Select(row => row.Item.Body)];

    private static List<string> Selected(Shell shell) =>
        [.. shell.View.Outline.Rows.Where(row => row.IsSelected).Select(row => row.Item.Body)];

    /// <summary>The editor of an item as a key finds it: its text and where the cursor is in it.</summary>
    private static EditorState Editor(
        string text,
        int cursor,
        bool firstLine = true,
        bool lastLine = true
    ) => new(text, cursor, 0, firstLine, lastLine);

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
        main.WaitUntil(
            "the server has the text",
            () => server.Get($"/api/pages/{pageId}")?.Contains("typed in WinUI 日本語") ?? false
        );
    }

    // コアが入力を求めると、質問が出る。答えた文字列で、コアが先へ進む。
    [Fact]
    public void A_prompt_is_answered_with_text()
    {
        var shell = ShellWithPage();

        shell.Send(new Event.CreateNewPage(""));
        main.WaitUntil("the shell is asked for a title", () => shell.Question is not null);
        Assert.Equal("Page title:", shell.Question?.Message);
        Assert.Equal(QuestionKind.Prompt, shell.Question?.Kind);
        shell.Answer("made-by-prompt");

        Assert.Null(shell.Question);
        main.WaitUntil("the new page is shown", () => shell.Title == "made-by-prompt");
    }

    // コアが確認を求めると、質問が出る。「いいえ」では何も起きず、「はい」でコアが先へ進む。
    [Fact]
    public void A_confirmation_is_answered_with_yes_or_no()
    {
        var shell = ShellWithPage();
        var pageId = Page(shell)!.Page.Id;

        shell.Send(new Event.DeleteCurrentPage());
        main.WaitUntil("the shell is asked to confirm", () => shell.Question is not null);
        Assert.Equal("Delete \"winui-notes\"?", shell.Question?.Message);
        Assert.Equal(QuestionKind.Confirm, shell.Question?.Kind);
        shell.Answer(false);
        Assert.Null(shell.Question);
        shell.Send(new Event.DeleteCurrentPage());
        main.WaitUntil("the shell is asked again", () => shell.Question is not null);
        Assert.NotNull(server.Get($"/api/pages/{pageId}"));
        shell.Answer(true);

        main.WaitUntil(
            "the page is gone from the server",
            () => server.Get($"/api/pages/{pageId}") is null
        );
    }

    // Enterで項目を分けると、コアが新しい項目にカーソルを置くように頼む。
    [Fact]
    public void Enter_asks_the_cursor_into_the_new_item()
    {
        var shell = ShellWithPage();

        var (_, second) = TwoItems(shell);

        main.WaitUntil(
            "the cursor is asked into the new item",
            () => shell.Focus?.ItemId == second
        );
    }

    // コアがクリップボードを求めると、その文字列で答える。項目のIDなら、トランスクルージョンになる。
    [Fact]
    public void The_clipboard_is_read_when_the_core_asks()
    {
        var shell = ShellWithPage();
        var (first, second) = TwoItems(shell);
        onClipboard = first;

        shell.Send(new Event.PasteAsTransclusion(second));

        main.WaitUntil(
            "the second item shows the first",
            () =>
                shell.View.Outline.Rows.Any(row =>
                    row.Item.Id == second && row.Item.TranscludeItemId == first
                )
        );
    }

    // ほかで表示されている項目を消すときは、元に戻しにくい操作として確認を求められる。
    [Fact]
    public void Deleting_a_transcluded_item_is_asked_as_hard_to_undo()
    {
        var shell = ShellWithPage();
        var (first, second) = TwoItems(shell);
        onClipboard = first;
        shell.Send(new Event.PasteAsTransclusion(second));
        main.WaitUntil(
            "the second item shows the first",
            () =>
                shell.View.Outline.Rows.Any(row =>
                    row.Item.Id == second && row.Item.TranscludeItemId == first
                )
        );

        shell.Send(new Event.DeleteItem(first));

        main.WaitUntil("the shell is asked to confirm", () => shell.Question is not null);
        Assert.Equal(QuestionKind.ConfirmDestructive, shell.Question?.Kind);
    }

    // Enterは、カーソルの位置で項目を分ける。カーソルは、新しい項目の先頭に移る。
    [Fact]
    public void Enter_splits_the_item_at_the_cursor()
    {
        var shell = ShellWithPage();
        var first = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(first);
        shell.Typed("日本語second", first);

        Assert.True(shell.Pressed(EditorKey.Enter, first, Editor("日本語second", 3)));

        main.WaitUntil("the item is split", () => Bodies(shell) is ["日本語", "second"]);
        var second = shell.View.Outline.Rows[1].Item.Id;
        main.WaitUntil(
            "the cursor is asked into the new item",
            () => shell.Focus?.ItemId == second
        );
    }

    // 絵文字のあとでのEnterは、絵文字を壊さずに項目を分ける。カーソルは、UTF-16では2つ分進んでいる。
    [Fact]
    public void Enter_behind_an_emoji_splits_the_item_there()
    {
        var shell = ShellWithPage();
        var first = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(first);
        shell.Typed("a😀b", first);

        Assert.True(shell.Pressed(EditorKey.Enter, first, Editor("a😀b", 3)));

        main.WaitUntil("the item is split", () => Bodies(shell) is ["a😀", "b"]);
    }

    // Tabは項目を字下げし、Shift+Tabは字上げする。
    [Fact]
    public void Tab_indents_and_Shift_Tab_outdents()
    {
        var shell = ShellWithPage();
        var (_, second) = TwoItems(shell);
        var level = shell.View.Outline.Rows[1].Item.Level;
        Assert.Equal(shell.View.Outline.Rows[0].Item.Level, level);

        Assert.True(shell.Pressed(EditorKey.Tab, second, Editor("", 0)));
        Assert.Equal(level + 1, shell.View.Outline.Rows[1].Item.Level);
        Assert.True(shell.Pressed(EditorKey.BackTab, second, Editor("", 0)));
        Assert.Equal(level, shell.View.Outline.Rows[1].Item.Level);
    }

    // 最初の行での上矢印は、前の項目へ移る。カーソルの位置は保たれる。
    [Fact]
    public void Up_on_the_first_line_goes_to_the_item_before()
    {
        var shell = ShellWithPage();
        var (first, second) = TwoItems(shell);
        shell.Typed("second", second);

        Assert.True(shell.Pressed(EditorKey.Up, second, Editor("second", 2)));

        main.WaitUntil(
            "the cursor is asked into the first item",
            () => shell.View.Outline.FocusRequest?.ItemId == first
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

        Assert.True(shell.Pressed(EditorKey.Down, first, Editor("first", 1)));

        main.WaitUntil(
            "the cursor is asked into the second item",
            () => shell.View.Outline.FocusRequest?.ItemId == second
        );
    }

    // 項目の中で行を移る矢印は、テキストの中での移動のままである。
    [Fact]
    public void An_arrow_inside_the_text_is_left_to_the_editor()
    {
        var shell = ShellWithPage();
        var (_, second) = TwoItems(shell);
        var inside = Editor("two\nlines", 5, firstLine: false, lastLine: false);

        Assert.False(shell.Pressed(EditorKey.Up, second, inside));
        Assert.False(shell.Pressed(EditorKey.Down, second, inside));
        Assert.False(shell.Pressed(EditorKey.Left, second, inside));
        Assert.False(shell.Pressed(EditorKey.Right, second, inside));
        Assert.False(shell.Pressed(EditorKey.Backspace, second, inside));
    }

    // 先頭での左矢印は、前の項目の末尾へ移る。末尾での右矢印は、次の項目の先頭へ移る。
    [Fact]
    public void Left_at_the_start_and_Right_at_the_end_leave_the_item()
    {
        var shell = ShellWithPage();
        var (first, second) = TwoItems(shell);

        Assert.True(shell.Pressed(EditorKey.Left, second, Editor("", 0)));
        main.WaitUntil(
            "the cursor is asked to the end of the first item",
            () => shell.View.Outline.FocusRequest?.ItemId == first
        );
        var asked = shell.View.Outline.FocusRequest?.Cursor;
        Assert.NotNull(asked);
        Assert.Equal(5, Cursor.Utf16(asked.Value, "first"));

        Assert.True(shell.Pressed(EditorKey.Right, first, Editor("first", 5)));
        main.WaitUntil(
            "the cursor is asked to the start of the second item",
            () => shell.View.Outline.FocusRequest?.ItemId == second
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

        Assert.True(shell.Pressed(EditorKey.Backspace, second, Editor("second", 0)));

        main.WaitUntil("the items are joined", () => Bodies(shell) is ["firstsecond"]);
    }

    // 文字を選んでいるときのBackspaceは、選んだ文字を消すだけである。
    [Fact]
    public void Backspace_with_a_selection_is_left_to_the_editor()
    {
        var shell = ShellWithPage();
        var (_, second) = TwoItems(shell);
        var selected = new EditorState("second", 0, 3, true, true);

        Assert.False(shell.Pressed(EditorKey.Backspace, second, selected));
    }

    // 末尾でのDeleteは、次の項目をつなぐ。途中でのDeleteは、文字を消すだけである。
    [Fact]
    public void Delete_at_the_end_joins_the_next_item()
    {
        var shell = ShellWithPage();
        var (first, second) = TwoItems(shell);
        shell.Typed("second", second);
        shell.StartEditing(first);

        Assert.False(shell.Pressed(EditorKey.Delete, first, Editor("first", 2)));
        Assert.True(shell.Pressed(EditorKey.Delete, first, Editor("first", 5)));

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

        Assert.True(shell.Pressed(EditorKey.TaskCycle, first, Editor("to do", 0)));
        main.WaitUntil(
            "the item is a task",
            () => shell.View.Outline.Rows[0].Task == TaskState.NotStarted
        );
        Assert.True(shell.Pressed(EditorKey.TaskCycle, first, Editor("to do", 0)));
        main.WaitUntil(
            "the task is running",
            () => shell.View.Outline.Rows[0].Task == TaskState.Running
        );
    }

    // タスクの印をクリックすると、タスクの状態が進む。項目を編集していなくてもよい。
    [Fact]
    public void A_click_on_the_mark_of_a_task_steps_its_state()
    {
        var shell = ShellWithPage();
        var first = shell.View.Outline.Rows[0].Item.Id;
        shell.StartEditing(first);
        shell.Typed("to do", first);
        Assert.True(shell.Pressed(EditorKey.TaskCycle, first, Editor("to do", 0)));
        main.WaitUntil(
            "the item is a task",
            () => shell.View.Outline.Rows[0].Task == TaskState.NotStarted
        );
        shell.FinishEditing(first);

        shell.ClickedTask(first);
        main.WaitUntil(
            "the task is running",
            () => shell.View.Outline.Rows[0].Task == TaskState.Running
        );
        shell.ClickedTask(first);
        main.WaitUntil("the task is done", () => shell.View.Outline.Rows[0].Task == TaskState.Done);
    }

    // 折りたたむキーは子の項目を隠し、開くキーは子の項目を表示する。切り替えではない。
    [Fact]
    public void The_fold_key_hides_the_children_and_the_unfold_key_shows_them()
    {
        var shell = ShellWithPage();
        var (first, second) = TwoItems(shell);
        Assert.True(shell.Pressed(EditorKey.Tab, second, Editor("", 0)));

        // The children are shown already
        Assert.True(shell.Pressed(EditorKey.Unfold, first, Editor("first", 0)));
        Assert.False(shell.View.Outline.Rows[0].IsCollapsed);
        Assert.True(shell.Pressed(EditorKey.Fold, first, Editor("first", 0)));
        main.WaitUntil("the child is hidden", () => shell.View.Outline.Rows.Count == 1);
        Assert.True(shell.View.Outline.Rows[0].IsCollapsed);
        // The children are hidden already
        Assert.True(shell.Pressed(EditorKey.Fold, first, Editor("first", 0)));
        Assert.True(shell.View.Outline.Rows[0].IsCollapsed);
        Assert.True(shell.Pressed(EditorKey.Unfold, first, Editor("first", 0)));
        main.WaitUntil("the child is shown again", () => shell.View.Outline.Rows.Count == 2);
        Assert.False(shell.View.Outline.Rows[0].IsCollapsed);
    }

    // 子のない項目には、折りたたむものがない。
    [Fact]
    public void An_item_without_children_is_not_folded()
    {
        var shell = ShellWithPage();
        var first = shell.View.Outline.Rows[0].Item.Id;

        Assert.True(shell.Pressed(EditorKey.Fold, first, Editor("", 0)));
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

        Assert.True(shell.Pressed(EditorKey.MoveDown, first, Editor("first", 0)));
        main.WaitUntil("the first item is below", () => Bodies(shell) is ["second", "first"]);
        Assert.True(shell.Pressed(EditorKey.MoveUp, first, Editor("first", 0)));
        main.WaitUntil(
            "the first item is on top again",
            () => Bodies(shell) is ["first", "second"]
        );
    }

    // 項目を右へ動かすと字下げになり、左へ動かすと字上げになる。
    [Fact]
    public void An_item_is_moved_in_and_out()
    {
        var shell = ShellWithPage();
        var (_, second) = TwoItems(shell);
        var level = shell.View.Outline.Rows[1].Item.Level;

        Assert.True(shell.Pressed(EditorKey.MoveIn, second, Editor("", 0)));
        Assert.Equal(level + 1, shell.View.Outline.Rows[1].Item.Level);
        Assert.True(shell.Pressed(EditorKey.MoveOut, second, Editor("", 0)));
        Assert.Equal(level, shell.View.Outline.Rows[1].Item.Level);
    }

    // 元に戻すと、直前の操作が取り消される。やり直すと、もう一度行われる。
    [Fact]
    public void Undo_takes_back_the_last_change_and_redo_makes_it_again()
    {
        var shell = ShellWithPage();
        var (_, second) = TwoItems(shell);
        var level = shell.View.Outline.Rows[1].Item.Level;
        Assert.True(shell.Pressed(EditorKey.Tab, second, Editor("", 0)));
        Assert.Equal(level + 1, shell.View.Outline.Rows[1].Item.Level);

        Assert.True(shell.Pressed(EditorKey.Undo, second, Editor("", 0)));
        main.WaitUntil(
            "the indent is taken back",
            () => shell.View.Outline.Rows[1].Item.Level == level
        );
        Assert.True(shell.Pressed(EditorKey.Redo, second, Editor("", 0)));
        main.WaitUntil(
            "the indent is made again",
            () => shell.View.Outline.Rows[1].Item.Level == level + 1
        );
    }

    /// <summary>Two items, "first" and "second", with the second one selected from its editor.</summary>
    private void SecondOfTwoSelected(Shell shell)
    {
        var (_, second) = TwoItems(shell);
        shell.Typed("second", second);
        Assert.True(shell.Pressed(EditorKey.SelectDown, second, Editor("second", 0)));
    }

    // 最後の行でのShift+下は、その項目を選ぶ。最初の行でのShift+上も同じである。ほかの行では、文字を選ぶキーのままである。
    [Fact]
    public void Shift_with_an_arrow_on_the_edge_line_selects_the_item()
    {
        var shell = ShellWithPage();
        var (first, _) = TwoItems(shell);

        Assert.False(
            shell.Pressed(EditorKey.SelectDown, first, Editor("a\nb", 0, lastLine: false))
        );
        Assert.False(shell.Pressed(EditorKey.SelectUp, first, Editor("a\nb", 3, firstLine: false)));
        Assert.False(shell.View.Outline.SelectionMode);
        Assert.True(shell.Pressed(EditorKey.SelectUp, first, Editor("first", 0)));

        Assert.True(shell.View.Outline.SelectionMode);
        Assert.Equal(["first"], Selected(shell));
    }

    // 選んでいるあいだのShift+上下は、選択を広げる。Escapeは、選択をやめる。
    [Fact]
    public void The_selection_grows_and_Escape_ends_it()
    {
        var shell = ShellWithPage();
        SecondOfTwoSelected(shell);

        shell.Selection(SelectionKey.ExtendUp);
        Assert.Equal(["first", "second"], Selected(shell));
        shell.Selection(SelectionKey.Escape);

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

        shell.Selection(SelectionKey.Delete);

        main.WaitUntil("the second item is gone", () => Bodies(shell) is ["first"]);
        Assert.False(shell.View.Outline.SelectionMode);
    }

    // 選んだ項目を上へ動かせる。
    [Fact]
    public void The_selected_items_are_moved()
    {
        var shell = ShellWithPage();
        SecondOfTwoSelected(shell);

        shell.Selection(SelectionKey.MoveUp);

        main.WaitUntil("the second item is on top", () => Bodies(shell) is ["second", "first"]);
        Assert.Equal(["second"], Selected(shell));
    }

    // 選んだ項目をコピーすると、Markdownがクリップボードに載る。同じ内容のHTMLも載る。
    [Fact]
    public void Copy_puts_the_selected_items_on_the_clipboard()
    {
        var shell = ShellWithPage();
        SecondOfTwoSelected(shell);
        shell.Selection(SelectionKey.ExtendUp);

        shell.Selection(SelectionKey.Copy);

        main.WaitUntil("the clipboard has the items", () => copied is not null);
        Assert.Equal("- first\n- second", copied);
        Assert.Contains("<li>first</li>", copiedHtml);
        Assert.Contains("<li>second</li>", copiedHtml);
        Assert.Equal(["first", "second"], Bodies(shell));
    }

    // 選択をやめるように言うと、選択が消える。ほかの場所をクリックしたときである。
    [Fact]
    public void Leaving_the_selection_ends_it()
    {
        var shell = ShellWithPage();
        SecondOfTwoSelected(shell);

        shell.LeaveSelection();

        Assert.False(shell.View.Outline.SelectionMode);
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

    // リンクをたどると、どれもブラウザに渡る。ウィキリンクとハッシュタグは、Webのフロントエンドでそのページを開くURLになる。付箋紙のページは変わらない。
    [Fact]
    public void A_link_that_is_followed_goes_to_the_browser_and_the_sticky_keeps_its_page()
    {
        Uri? browsed = null;
        var shell = new Shell(
            server.Address,
            "/pages/winui-notes",
            main.Post,
            reply => reply(null),
            (_, _) => { },
            uri => browsed = uri
        );
        shells.Add(shell);
        main.WaitUntil("the page is shown", () => shell.View.Outline.Rows.Count > 0);
        var tags = shell.View.Config.TagsDir;
        Assert.NotEqual("", tags);

        shell.Follow(new LinkTarget.Page("linked/idea"));
        Assert.Equal(new Uri($"{server.Address}/pages/linked/idea"), browsed);

        shell.Follow(new LinkTarget.Tag("todo"));
        Assert.Equal(new Uri($"{server.Address}/pages/{tags}todo"), browsed);

        shell.Follow(new LinkTarget.Out(new Uri("https://example.com/a")));
        Assert.Equal(new Uri("https://example.com/a"), browsed);
        Assert.Equal("/pages/winui-notes", shell.Url);
    }

    /// <summary>
    /// Makes the journal of a day long ago, with one item, and waits until the server has it.
    /// </summary>
    private void JournalOfADayLongAgo()
    {
        var old = NewShell("/journals/2020-01-02");
        main.WaitUntil("the old journal is shown", () => old.View.Outline.Rows.Count > 0);
        var pageId = Journal(old)!.PageId;
        var item = old.View.Outline.Rows[0].Item.Id;
        old.StartEditing(item);
        old.Typed("written long ago", item);
        main.WaitUntil(
            "the old journal is saved",
            () => server.Get($"/api/pages/{pageId}")?.Contains("written long ago") ?? false
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

    // 今日でない日のジャーナルのタイトルは、その日付だけである。
    [Fact]
    public void The_title_of_the_journal_of_another_day_is_its_date_alone()
    {
        var shell = NewShell("/journals/2020-01-02");
        main.WaitUntil("the journal is shown", () => shell.View.Outline.Rows.Count > 0);

        Assert.Equal("2020年1月2日 木曜日", shell.Title);
    }

    // 今日のジャーナルを前の日までスクロールすると、タイトルはその日の日付になり、「(Today)」は付かない。今日まで戻ると、また付く。
    [Fact]
    public void Scrolling_the_journal_to_an_earlier_day_shows_that_date_in_the_title()
    {
        JournalOfADayLongAgo();
        var shell = NewShell("/today");
        main.WaitUntil(
            "the earlier day is shown under today",
            () => shell.View.Outline.Rows.Any(row => row.DateHeader is not null)
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
