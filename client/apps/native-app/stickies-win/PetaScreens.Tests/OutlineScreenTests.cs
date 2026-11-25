using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.VisualStudio.TestTools.UnitTesting.AppContainer;
using PetaBorder;
using PetaTypes;
using Windows.System;
using Border = Microsoft.UI.Xaml.Controls.Border;

namespace PetaScreens.Tests;

/// <summary>
/// The outline in a real sticky, with the keys and the clicks sent through Windows. These need
/// a real journal server, so they are skipped without <c>PETA_E2E_SERVER</c>.
/// </summary>
[TestClass]
public sealed class OutlineScreenTests
{
    private const int Port = 18113;

    // 項目をクリックすると、その項目のエディタにカーソルが入る。入力した文字は、コアに届く。
    [UITestMethod]
    public async Task What_is_typed_into_a_clicked_item_reaches_the_core()
    {
        await using var screen = await Screen.Open(Port);

        await screen.Edit(0);
        await screen.Type("typed 日本語");

        await Screen.Until("the core has the text", () => screen.Bodies()[0] == "typed 日本語");
        Assert.AreEqual(screen.Rows[0].Item.Id, screen.Shell.View.Outline.FocusedItemId);
    }

    // Enterは、カーソルの位置で項目を分ける。カーソルは、新しい項目の先頭に移る。
    [UITestMethod]
    public async Task Enter_splits_the_item_and_the_cursor_goes_to_the_new_one()
    {
        await using var screen = await Screen.Open(Port);
        await screen.Edit(0);
        await screen.Type("firstsecond");
        await screen.MoveCursor(5);

        Screen.Press(VirtualKey.Enter);

        await Screen.Until("the item is split", () => screen.BodiesAre("first", "second"));
        await Screen.Until("the cursor is in the new item", () => screen.Focused == 1);
        Assert.AreEqual(0, screen.Editor.SelectionStart);
    }

    // 編集中でない空の項目には、行頭の点が見えない。文字のある項目と、編集中の項目には見える（#1042）。
    [UITestMethod]
    public async Task An_empty_item_that_is_not_edited_shows_no_bullet()
    {
        await using var screen = await Screen.Open(Port);
        await screen.Edit(0);
        await screen.Type("first");
        Screen.Press(VirtualKey.Enter);
        await Screen.Until("the cursor is in the new item", () => screen.Focused == 1);
        Assert.AreEqual(1, Screen.BulletOf(screen.Items()[1]).Opacity);

        await screen.Edit(0);

        await Screen.Until("the empty item has no bullet", () =>
            Screen.BulletOf(screen.Items()[1]).Opacity == 0
        );
        Assert.AreEqual(1, Screen.BulletOf(screen.Items()[0]).Opacity);
    }

    // 上矢印は、前の項目へ、同じ位置のまま移る。
    [UITestMethod]
    public async Task Up_goes_to_the_item_before_at_the_same_place()
    {
        await using var screen = await Screen.Open(Port);
        await screen.TwoItems();
        await screen.MoveCursor(3);

        Screen.Press(VirtualKey.Up);

        await Screen.Until("the cursor is in the first item", () => screen.Focused == 0);
        Assert.AreEqual(3, screen.Editor.SelectionStart);
    }

    // Shift+Enterは、項目の中で行を分ける。
    [UITestMethod]
    public async Task Shift_Enter_breaks_the_line_inside_the_item()
    {
        await using var screen = await Screen.Open(Port);
        await screen.Edit(0);
        await screen.Type("ab");
        await screen.MoveCursor(1);

        Screen.Press(VirtualKey.Enter, Held.Shift);

        await Screen.Until("the line is broken", () => screen.Bodies()[0] == "a\nb");
        Assert.HasCount(1, screen.Items());
    }

    // Ctrl+Enterは、項目をタスクにし、押すたびに状態を進める。文字は入らない。
    [UITestMethod]
    public async Task Control_Enter_makes_the_item_a_task_and_steps_its_state()
    {
        await using var screen = await Screen.Open(Port);
        await screen.Edit(0);
        await screen.Type("to do");

        Screen.Press(VirtualKey.Enter, Held.Control);
        await Screen.Until("the item is a task", () => screen.Rows[0].Task == TaskState.NotStarted);
        Screen.Press(VirtualKey.Enter, Held.Control);
        await Screen.Until("the task is running", () => screen.Rows[0].Task == TaskState.Running);
        Screen.Press(VirtualKey.Enter, Held.Control);
        await Screen.Until("the task is done", () => screen.Rows[0].Task == TaskState.Done);

        Assert.AreEqual("to do", screen.Editor.Text);
    }

    // Tabは、項目を字下げする。文字は入らない。
    [UITestMethod]
    public async Task Tab_indents_the_item_and_types_nothing()
    {
        await using var screen = await Screen.Open(Port);
        await screen.TwoItems();
        var level = screen.Rows[1].Item.Level;

        Screen.Press(VirtualKey.Tab);

        await Screen.Until("the item is indented", () => screen.Rows[1].Item.Level == level + 1);
        Assert.AreEqual("second", screen.Editor.Text);
        Assert.AreEqual(1, screen.Focused);
    }

    // 先頭でのBackspaceは、項目を前の項目につなぐ。カーソルは、つなぎ目に置かれる。
    [UITestMethod]
    public async Task Backspace_at_the_start_joins_the_items()
    {
        await using var screen = await Screen.Open(Port);
        await screen.TwoItems();
        await screen.MoveCursor(0);

        Screen.Press(VirtualKey.Back);

        await Screen.Until("the items are joined", () => screen.BodiesAre("firstsecond"));
        await Screen.Until("the cursor is at the joint", () =>
            screen.Focused == 0 && screen.Editor.SelectionStart == 5
        );
    }

    // 末尾でのDeleteは、次の項目をつなぐ。
    [UITestMethod]
    public async Task Delete_at_the_end_joins_the_next_item()
    {
        await using var screen = await Screen.Open(Port);
        await screen.TwoItems();
        Screen.Press(VirtualKey.Up);
        await Screen.Until("the cursor is in the first item", () => screen.Focused == 0);
        await screen.MoveCursor(5);

        Screen.Press(VirtualKey.Delete);

        await Screen.Until("the items are joined", () => screen.BodiesAre("firstsecond"));
    }

    // Shift+Alt+下矢印は、項目を下へ動かす。カーソルは、その項目に残る。
    [UITestMethod]
    public async Task Shift_Alt_Down_moves_the_item_down()
    {
        await using var screen = await Screen.Open(Port);
        await screen.TwoItems();
        Screen.Press(VirtualKey.Up);
        await Screen.Until("the cursor is in the first item", () => screen.Focused == 0);

        Screen.Press(VirtualKey.Down, Held.Shift | Held.Alt);

        await Screen.Until("the first item is below", () => screen.BodiesAre("second", "first"));
        await Screen.Until("the cursor is still in it", () =>
            screen.Focused == 1 && screen.Editor.Text == "first"
        );
    }

    // Ctrl+上矢印は子の項目を隠し、Ctrl+下矢印は表示する。
    [UITestMethod]
    public async Task Control_Up_hides_the_children_and_Control_Down_shows_them()
    {
        await using var screen = await Screen.Open(Port);
        await screen.TwoItems();
        Screen.Press(VirtualKey.Tab);
        await Screen.Until("the second item is a child", () => screen.Rows[0].HasChildren);
        Screen.Press(VirtualKey.Up);
        await Screen.Until("the cursor is in the parent", () => screen.Focused == 0);

        Screen.Press(VirtualKey.Up, Held.Control);
        await Screen.Until("only the parent is shown", () => screen.Items().Count == 1);
        Screen.Press(VirtualKey.Down, Held.Control);

        await Screen.Until("the child is shown again", () => screen.Items().Count == 2);
    }

    // Ctrl+Zはコアの操作を元に戻し、Ctrl+Shift+Zはやり直す。
    [UITestMethod]
    public async Task Control_Z_takes_back_a_change_of_the_outline_and_Control_Shift_Z_makes_it_again()
    {
        await using var screen = await Screen.Open(Port);
        await screen.TwoItems();
        var level = screen.Rows[1].Item.Level;
        Screen.Press(VirtualKey.Tab);
        await Screen.Until("the item is indented", () => screen.Rows[1].Item.Level == level + 1);

        Screen.Press(VirtualKey.Z, Held.Control);
        await Screen.Until("the indent is taken back", () => screen.Rows[1].Item.Level == level);
        Screen.Press(VirtualKey.Z, Held.Control | Held.Shift);

        await Screen.Until("the indent is made again", () =>
            screen.Rows[1].Item.Level == level + 1
        );
    }

    // Shift+上矢印は、項目を選ぶ。もう一度押すと選択が広がり、Escapeで選択が消える。
    [UITestMethod]
    public async Task Shift_Up_selects_items_and_Escape_ends_the_selection()
    {
        await using var screen = await Screen.Open(Port);
        await screen.SecondOfTwoSelected();

        Screen.Press(VirtualKey.Up, Held.Shift);
        await Screen.Until("the selection is wider", () => screen.SelectedAre("first", "second"));
        Screen.Press(VirtualKey.Escape);

        await Screen.Until("nothing is selected", () => screen.Selected().Count == 0);
        Assert.IsFalse(screen.Shell.View.Outline.SelectionMode);
    }

    // 選んでいるあいだのDeleteは、選んだ項目を消す。
    [UITestMethod]
    public async Task Delete_deletes_the_selected_items()
    {
        await using var screen = await Screen.Open(Port);
        await screen.SecondOfTwoSelected();

        Screen.Press(VirtualKey.Delete);

        await Screen.Until("the second item is gone", () => screen.BodiesAre("first"));
    }

    // 選んでいるあいだのCtrl+Cは、選んだ項目をクリップボードに載せる。
    [UITestMethod]
    public async Task Control_C_puts_the_selected_items_on_the_clipboard()
    {
        await using var screen = await Screen.Open(Port);
        await screen.SecondOfTwoSelected();
        Screen.PutOnClipboard("before");

        Screen.Press(VirtualKey.C, Held.Control);

        await Screen.UntilClipboardHas("- second");
    }

    // Ctrl+Vは、Markdownのリストを項目に分けて、カーソルの位置に貼る。
    [UITestMethod]
    public async Task Control_V_cuts_a_markdown_list_into_items()
    {
        await using var screen = await Screen.Open(Port);
        await screen.TwoItems();
        // Other apps of Windows put line breaks of two letters on the clipboard
        Screen.PutOnClipboard("- apple\r\n- pear");

        Screen.Press(VirtualKey.V, Held.Control);

        await Screen.Until("the list is two items", () =>
            screen.BodiesAre("first", "secondapple", "pear")
        );
    }

    // Ctrl+Vは、クリップボードの画像をサーバーのassetsに保存し、カーソルの位置にそのリンクを入れる。
    [UITestMethod]
    public async Task Control_V_pastes_the_image_on_the_clipboard_as_a_link_to_the_assets()
    {
        await using var screen = await Screen.Open(Port);
        await screen.TwoItems();
        await Screen.PutImageOnClipboard();

        Screen.Press(VirtualKey.V, Held.Control);

        await Screen.Until("the second item links the image", () =>
            screen.Bodies() is ["first", var second]
            && second.StartsWith("second![", StringComparison.Ordinal)
            && second.Contains("](../assets/", StringComparison.Ordinal)
        );
    }

    // Ctrl+Shift+Vは、クリップボードの文字をそのまま項目に貼る。
    [UITestMethod]
    public async Task Control_Shift_V_puts_the_text_into_the_item_as_it_is()
    {
        await using var screen = await Screen.Open(Port);
        await screen.TwoItems();
        Screen.PutOnClipboard("- apple\r\n- pear");

        Screen.Press(VirtualKey.V, Held.Control | Held.Shift);

        await Screen.Until("the text is in the item", () =>
            screen.BodiesAre("first", "second- apple\n- pear")
        );
    }

    // 親と子を選んでコピーし、別の項目の終わりに貼り付けると、親と子の形のまま戻る。
    [UITestMethod]
    public async Task A_parent_and_its_child_copied_and_pasted_at_the_end_of_another_item_come_back_in_their_shape()
    {
        await using var screen = await Screen.Open(Port);
        await ParentChildAndThird(screen);
        await screen.Edit(0);
        await screen.MoveCursor(0);
        Screen.Press(VirtualKey.Up, Held.Shift);
        await Screen.Until("the parent and its child are selected", () =>
            screen.SelectedAre("parent", "child") && screen.Focused is null
        );
        Screen.PutOnClipboard("before");
        Screen.Press(VirtualKey.C, Held.Control);
        await Screen.UntilClipboardHas("- parent\n  - child");
        await screen.Edit(2);
        Screen.Press(VirtualKey.End);

        Screen.Press(VirtualKey.V, Held.Control);

        await Screen.Until("the parent and its child are pasted", () =>
            LevelsAndBodiesAre(screen, "1 parent", "2 child", "1 thirdparent", "2 child")
        );
    }

    /// <summary>Whether the level and the body of each item are these, as "level body".</summary>
    private static bool LevelsAndBodiesAre(Screen screen, params string[] items) =>
        screen.Rows.Select(row => $"{row.Item.Level} {row.Item.Body}").SequenceEqual(items);

    /// <summary>
    /// Three items: "parent", its child "child", and "third" after them, with the cursor at
    /// the end of "third".
    /// </summary>
    private static async Task ParentChildAndThird(Screen screen)
    {
        await screen.Edit(0);
        await screen.Type("parent");
        Screen.Press(VirtualKey.Enter);
        await Screen.Until("the cursor is in the second item", () => screen.Focused == 1);
        await screen.Type("child");
        Screen.Press(VirtualKey.Tab);
        await Screen.Until("the second item is a child", () =>
            LevelsAndBodiesAre(screen, "1 parent", "2 child")
        );
        Screen.Press(VirtualKey.Enter);
        await Screen.Until("the cursor is in the third item", () => screen.Focused == 2);
        Screen.Press(VirtualKey.Tab, Held.Shift);
        await Screen.Until("the third item is not a child", () =>
            LevelsAndBodiesAre(screen, "1 parent", "2 child", "1 ")
        );
        await screen.Type("third");
    }

    // 選んでいるあいだのShift+Alt+上矢印は、選んだ項目を上へ動かす。
    [UITestMethod]
    public async Task Shift_Alt_Up_moves_the_selected_items()
    {
        await using var screen = await Screen.Open(Port);
        await screen.SecondOfTwoSelected();

        Screen.Press(VirtualKey.Up, Held.Shift | Held.Alt);

        await Screen.Until("the second item is on top", () => screen.BodiesAre("second", "first"));
    }

    // 選んでいるあいだに項目の文字をクリックすると、選択が消え、その項目に入力できる。
    [UITestMethod]
    public async Task A_click_on_an_item_ends_the_selection()
    {
        await using var screen = await Screen.Open(Port);
        await screen.SecondOfTwoSelected();

        await screen.Edit(0);
        Screen.Press(VirtualKey.End);
        await screen.Type("!");

        Assert.IsFalse(screen.Shell.View.Outline.SelectionMode);
        await Screen.Until("the text reaches the core", () => screen.BodiesAre("first!", "second"));
    }

    // 選んでいるあいだに検索ボタンをクリックすると、選択が消える。
    [UITestMethod]
    public async Task A_click_on_the_search_button_ends_the_selection()
    {
        await using var screen = await Screen.Open(Port);
        await screen.SecondOfTwoSelected();

        await screen.Click(Screen.ButtonOf(screen.Window, "Search pages (Ctrl+F)"));

        await Screen.Until("the selection ends", () => !screen.Shell.View.Outline.SelectionMode);
    }

    // 選んでいるあいだにタイトルバーをクリックすると、選択が消える。
    [UITestMethod]
    public async Task A_click_on_the_title_bar_ends_the_selection()
    {
        await using var screen = await Screen.Open(Port);
        await screen.SecondOfTwoSelected();

        await screen.ClickTitleBar();

        await Screen.Until("the selection ends", () => !screen.Shell.View.Outline.SelectionMode);
    }

    // 編集していない項目を右クリックすると、メニューに「Paste as Transclusion」がある。
    [UITestMethod]
    public async Task The_menu_of_an_item_that_is_not_edited_has_paste_as_transclusion()
    {
        await using var screen = await Screen.Open(Port);
        await screen.TwoItems();

        await screen.Click(screen.Items()[0], right: true);

        await screen.MenuEntry("Paste as Transclusion");
        Screen.Press(VirtualKey.Escape);
    }

    // Ctrl+Fで検索欄が開く。入力するとページが見つかり、Enterでそのページが開く。
    [UITestMethod]
    public async Task Typing_in_the_search_box_finds_a_page_and_Enter_opens_it()
    {
        await using var screen = await Screen.Open(Port);
        screen.Shell.Open("/pages/apple-pie");
        await Screen.Until("the second page is shown", () => screen.Shell.Title == "apple-pie");
        screen.Shell.Open("/pages/screen");
        await Screen.Until("the first page is shown again", () => screen.Shell.Title == "screen");
        await screen.Edit(0);

        Screen.Press(VirtualKey.F, Held.Control);
        await Screen.Until("the search box is open", () => screen.Window.Look.Searching);
        await Task.Delay(100);
        UserInput.Type("apple");
        await Screen.Until("the page is found", () =>
            screen.Shell.View.Search.Results.FirstOrDefault()?.Title == "apple-pie"
        );
        Screen.Press(VirtualKey.Down);
        Screen.Press(VirtualKey.Enter);

        await Screen.Until("the page found is shown", () => screen.Shell.Title == "apple-pie");
        await Screen.Until("the search box is closed", () => !screen.Window.Look.Searching);
    }

    // 検索結果をクリックすると、そのページが開く。
    [UITestMethod]
    public async Task A_click_on_a_page_found_by_the_search_opens_it()
    {
        await using var screen = await Screen.Open(Port);
        screen.Shell.Open("/pages/apple-pie");
        await Screen.Until("the second page is shown", () => screen.Shell.Title == "apple-pie");
        screen.Shell.Open("/pages/screen");
        await Screen.Until("the first page is shown again", () => screen.Shell.Title == "screen");
        await screen.Click(Screen.ButtonOf(screen.Window, "Search pages (Ctrl+F)"));
        await Screen.Until("the search box is open", () => screen.Window.Look.Searching);
        await Task.Delay(100);
        UserInput.Type("apple");
        Border? found = null;
        await Screen.Until("the page is found", () =>
        {
            found = Screen
                .Under<Border>((UIElement)screen.Window.Content)
                .FirstOrDefault(entry =>
                    entry.Child is StackPanel { Children: [TextBlock { Text: "apple-pie" }, ..] }
                );
            return found is not null;
        });

        await screen.Click(found!);

        await Screen.Until("the page found is shown", () => screen.Shell.Title == "apple-pie");
        await Screen.Until("the search box is closed", () => !screen.Window.Look.Searching);
    }

    // ドラッグで項目をまたぐと、またいだ項目が選ばれる。
    [UITestMethod]
    public async Task A_drag_across_items_selects_them()
    {
        await using var screen = await Screen.OpenWritten(Port, "one", "two", "three");

        await screen.Drag(
            Screen.Under<RichTextBlock>(screen.Items()[0]).First(),
            screen.Items()[2]
        );

        await Screen.Until("the items are selected", () =>
            screen.SelectedAre("one", "two", "three")
        );
    }

    // ほかの項目に映されている項目を消すときは、先に確かめる。OKで消える。
    [UITestMethod]
    public async Task Deleting_an_item_that_another_item_transcludes_asks_first_and_OK_deletes_it()
    {
        await using var screen = await Screen.OpenWritten(Port, "source", "other", "shows it");
        var source = await TranscludedAndSelected(screen);

        Screen.Press(VirtualKey.Delete);
        await screen.Click(await screen.DialogButton("OK"));

        await Screen.Until("the item is deleted", () =>
            screen.Rows.Count == 2 && screen.Rows.All(row => row.Item.Id != source)
        );
    }

    // 確かめるダイアログでは、Enterは取り消しになる。何も消えず、選択も残る。
    [UITestMethod]
    public async Task Enter_in_the_dialog_before_deleting_a_transcluded_item_deletes_nothing()
    {
        await using var screen = await Screen.OpenWritten(Port, "source", "other", "shows it");
        var source = await TranscludedAndSelected(screen);

        Screen.Press(VirtualKey.Delete);
        await screen.DialogButton("Cancel");
        Screen.Press(VirtualKey.Enter);

        await Screen.Until("the dialog is closed", () => !screen.Dialogs().Any());
        await Task.Delay(300);
        Assert.AreEqual(source, screen.Rows[0].Item.Id);
        Assert.IsTrue(screen.SelectedAre("source"));
    }

    /// <summary>
    /// Makes the last item show the first one, and selects the first one. Returns the id of
    /// the first one.
    /// </summary>
    private static async Task<string> TranscludedAndSelected(Screen screen)
    {
        var source = screen.Rows[0].Item.Id;
        screen.Transclude(screen.Rows[2].Item.Id, source);
        await screen.Edit(0);
        Screen.Press(VirtualKey.Up, Held.Shift);
        await Screen.Until("the first item is selected", () => screen.SelectedAre("source"));
        return source;
    }
}
