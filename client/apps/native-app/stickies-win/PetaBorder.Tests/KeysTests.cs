using PetaTypes;

namespace PetaBorder.Tests;

public sealed class KeysTests
{
    // コアはカーソルを文字（Unicodeスカラー）で数える。絵文字はUTF-16では2つだが、1文字である。
    [Fact]
    public void A_cursor_of_WinUI_is_counted_in_characters_for_the_core()
    {
        Assert.Equal(0UL, Cursor.Characters(0, "日本😀語"));
        Assert.Equal(2UL, Cursor.Characters(2, "日本😀語"));
        Assert.Equal(3UL, Cursor.Characters(4, "日本😀語"));
        Assert.Equal(4UL, Cursor.Characters(5, "日本😀語"));
    }

    // コアのカーソルは、UTF-16の位置に戻せる。本文より先の位置は、末尾になる。
    [Fact]
    public void A_cursor_of_the_core_is_counted_in_UTF16_for_WinUI()
    {
        Assert.Equal(0, Cursor.Utf16(0, "日本😀語"));
        Assert.Equal(4, Cursor.Utf16(3, "日本😀語"));
        Assert.Equal(5, Cursor.Utf16(4, "日本😀語"));
        Assert.Equal(5, Cursor.Utf16(long.MaxValue, "日本😀語"));
    }

    // 編集欄のキーは、Web版と同じである。
    [Theory]
    [InlineData(Key.Enter, Held.None, EditorKey.Enter)]
    [InlineData(Key.Enter, Held.Control, EditorKey.TaskCycle)]
    [InlineData(Key.Tab, Held.None, EditorKey.Tab)]
    [InlineData(Key.Tab, Held.Shift, EditorKey.BackTab)]
    [InlineData(Key.Up, Held.None, EditorKey.Up)]
    [InlineData(Key.Down, Held.None, EditorKey.Down)]
    [InlineData(Key.Left, Held.None, EditorKey.Left)]
    [InlineData(Key.Right, Held.None, EditorKey.Right)]
    [InlineData(Key.Backspace, Held.None, EditorKey.Backspace)]
    [InlineData(Key.Delete, Held.None, EditorKey.Delete)]
    [InlineData(Key.Up, Held.Shift, EditorKey.SelectUp)]
    [InlineData(Key.Down, Held.Shift, EditorKey.SelectDown)]
    [InlineData(Key.Up, Held.Control, EditorKey.Fold)]
    [InlineData(Key.Down, Held.Control, EditorKey.Unfold)]
    [InlineData(Key.Up, Held.Shift | Held.Alt, EditorKey.MoveUp)]
    [InlineData(Key.Down, Held.Shift | Held.Alt, EditorKey.MoveDown)]
    [InlineData(Key.Left, Held.Shift | Held.Alt, EditorKey.MoveOut)]
    [InlineData(Key.Right, Held.Shift | Held.Alt, EditorKey.MoveIn)]
    [InlineData(Key.Z, Held.Control, EditorKey.Undo)]
    [InlineData(Key.Z, Held.Control | Held.Shift, EditorKey.Redo)]
    public void A_key_in_the_editor_means_what_it_means_in_the_web_frontend(
        Key key,
        Held held,
        EditorKey means
    )
    {
        Assert.Equal(means, Keyboard.InEditor(key, held));
    }

    // Shift+Enterは項目の中の改行で、アウトラインのキーではない。ほかの組み合わせも、編集欄に任せる。
    [Theory]
    [InlineData(Key.Enter, Held.Shift)]
    [InlineData(Key.Left, Held.Shift)]
    [InlineData(Key.Left, Held.Control)]
    [InlineData(Key.C, Held.Control)]
    [InlineData(Key.Z, Held.None)]
    [InlineData(Key.Escape, Held.None)]
    public void A_key_that_only_means_something_to_the_text_is_left_to_the_editor(
        Key key,
        Held held
    )
    {
        Assert.Null(Keyboard.InEditor(key, held));
    }

    // 項目を選んでいるあいだのキーは、Web版と同じである。
    [Theory]
    [InlineData(Key.Escape, Held.None, SelectionKey.Escape)]
    [InlineData(Key.Backspace, Held.None, SelectionKey.Delete)]
    [InlineData(Key.Delete, Held.None, SelectionKey.Delete)]
    [InlineData(Key.Up, Held.Shift, SelectionKey.ExtendUp)]
    [InlineData(Key.Down, Held.Shift, SelectionKey.ExtendDown)]
    [InlineData(Key.Up, Held.Shift | Held.Alt, SelectionKey.MoveUp)]
    [InlineData(Key.Down, Held.Shift | Held.Alt, SelectionKey.MoveDown)]
    [InlineData(Key.Left, Held.Shift | Held.Alt, SelectionKey.Outdent)]
    [InlineData(Key.Tab, Held.Shift, SelectionKey.Outdent)]
    [InlineData(Key.Right, Held.Shift | Held.Alt, SelectionKey.Indent)]
    [InlineData(Key.Tab, Held.None, SelectionKey.Indent)]
    [InlineData(Key.C, Held.Control, SelectionKey.Copy)]
    [InlineData(Key.X, Held.Control, SelectionKey.Cut)]
    [InlineData(Key.Z, Held.Control, SelectionKey.Undo)]
    [InlineData(Key.Z, Held.Control | Held.Shift, SelectionKey.Redo)]
    public void A_key_while_items_are_selected_means_what_it_means_in_the_web_frontend(
        Key key,
        Held held,
        SelectionKey means
    )
    {
        Assert.Equal(means, Keyboard.InSelection(key, held));
    }

    // 選択に意味のないキーは、何もしない。
    [Theory]
    [InlineData(Key.Enter, Held.None)]
    [InlineData(Key.Up, Held.None)]
    [InlineData(Key.C, Held.None)]
    public void A_key_that_means_nothing_to_a_selection_does_nothing(Key key, Held held)
    {
        Assert.Null(Keyboard.InSelection(key, held));
    }
}
