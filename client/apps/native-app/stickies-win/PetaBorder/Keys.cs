using System.Text;
using PetaTypes;

namespace PetaBorder;

/// <summary>A key that means something to the outline, and not only to the text of an item.</summary>
public enum EditorKey
{
    Enter,
    Tab,
    BackTab,
    Up,
    Down,
    Left,
    Right,
    Backspace,
    Delete,

    /// <summary>Makes the item a task, or steps the state of the task.</summary>
    TaskCycle,

    /// <summary>Hides the children of the item.</summary>
    Fold,

    /// <summary>Shows the children of the item again.</summary>
    Unfold,

    // Move the item among the items: up and down, and in and out by one level
    MoveUp,
    MoveDown,
    MoveIn,
    MoveOut,
    Undo,
    Redo,

    /// <summary>
    /// Shift with Up. On the first line it selects the item; on any other line it selects text,
    /// as in any editor.
    /// </summary>
    SelectUp,

    /// <summary>Shift with Down, which selects the item on the last line.</summary>
    SelectDown,
}

/// <summary>The editor of an item as a key finds it.</summary>
/// <param name="Text">The Markdown in the editor, with <c>\n</c> between its lines.</param>
/// <param name="SelectionStart">
/// Where the selection starts, in UTF-16 units as WinUI counts. The cursor, when nothing is
/// selected.
/// </param>
/// <param name="SelectionLength">How much is selected. 0 when there is only a cursor.</param>
/// <param name="OnFirstLine">The cursor is on the first line of the text as it is drawn.</param>
/// <param name="OnLastLine">The cursor is on the last line of the text as it is drawn.</param>
public sealed record EditorState(
    string Text,
    int SelectionStart,
    int SelectionLength,
    bool OnFirstLine,
    bool OnLastLine
);

/// <summary>A key of the keyboard that the outline knows, whatever toolkit read it.</summary>
public enum Key
{
    Enter,
    Tab,
    Up,
    Down,
    Left,
    Right,
    Backspace,
    Delete,
    Escape,
    C,
    X,
    Z,
}

/// <summary>The keys that are held down with another key.</summary>
[Flags]
public enum Held
{
    None = 0,
    Shift = 1,
    Control = 2,
    Alt = 4,
}

/// <summary>What the keys of the keyboard mean. They are those of the web frontend.</summary>
public static class Keyboard
{
    private const Held ShiftAlt = Held.Shift | Held.Alt;

    /// <summary>
    /// What a key means in the editor of an item. <c>null</c> for a key that only means
    /// something to the text, such as Shift+Enter, which breaks the line inside the item.
    /// </summary>
    public static EditorKey? InEditor(Key key, Held held) =>
        (key, held) switch
        {
            (Key.Enter, Held.None) => EditorKey.Enter,
            (Key.Enter, Held.Control) => EditorKey.TaskCycle,
            (Key.Tab, Held.None) => EditorKey.Tab,
            (Key.Tab, Held.Shift) => EditorKey.BackTab,
            (Key.Up, Held.None) => EditorKey.Up,
            (Key.Down, Held.None) => EditorKey.Down,
            (Key.Left, Held.None) => EditorKey.Left,
            (Key.Right, Held.None) => EditorKey.Right,
            (Key.Backspace, Held.None) => EditorKey.Backspace,
            (Key.Delete, Held.None) => EditorKey.Delete,
            (Key.Up, Held.Shift) => EditorKey.SelectUp,
            (Key.Down, Held.Shift) => EditorKey.SelectDown,
            (Key.Up, Held.Control) => EditorKey.Fold,
            (Key.Down, Held.Control) => EditorKey.Unfold,
            (Key.Up, ShiftAlt) => EditorKey.MoveUp,
            (Key.Down, ShiftAlt) => EditorKey.MoveDown,
            (Key.Left, ShiftAlt) => EditorKey.MoveOut,
            (Key.Right, ShiftAlt) => EditorKey.MoveIn,
            (Key.Z, Held.Control) => EditorKey.Undo,
            (Key.Z, Held.Control | Held.Shift) => EditorKey.Redo,
            _ => null,
        };

    /// <summary>
    /// What a key means while items are selected. No editor has the cursor then, and a key that
    /// means nothing to a selection does nothing.
    /// </summary>
    public static SelectionKey? InSelection(Key key, Held held) =>
        (key, held) switch
        {
            (Key.Escape, Held.None) => SelectionKey.Escape,
            (Key.Backspace, Held.None) or (Key.Delete, Held.None) => SelectionKey.Delete,
            (Key.Up, Held.Shift) => SelectionKey.ExtendUp,
            (Key.Down, Held.Shift) => SelectionKey.ExtendDown,
            (Key.Up, ShiftAlt) => SelectionKey.MoveUp,
            (Key.Down, ShiftAlt) => SelectionKey.MoveDown,
            (Key.Left, ShiftAlt) or (Key.Tab, Held.Shift) => SelectionKey.Outdent,
            (Key.Right, ShiftAlt) or (Key.Tab, Held.None) => SelectionKey.Indent,
            (Key.C, Held.Control) => SelectionKey.Copy,
            (Key.X, Held.Control) => SelectionKey.Cut,
            (Key.Z, Held.Control) => SelectionKey.Undo,
            (Key.Z, Held.Control | Held.Shift) => SelectionKey.Redo,
            _ => null,
        };
}

public sealed partial class Shell
{
    /// <summary>
    /// The user pressed a key in the editor of an item. False when the key means nothing to the
    /// outline here, and the editor does with it what it does in any text.
    /// </summary>
    public bool Pressed(EditorKey key, string itemId, EditorState editor)
    {
        ArgumentNullException.ThrowIfNull(editor);
        if (EventOf(key, itemId, editor) is not { } pressed)
            return false;
        if (pressed is Event.ShiftSelect)
        {
            // The edit ends first, which saves what was typed. The core does not save it when
            // a selection starts, and a move of the selected item would bring back the text
            // the server has
            Send(new Event.EditFinished(itemId));
        }
        Send(pressed);
        return true;
    }

    /// <summary>
    /// The user pasted into the editor of an item. The core asks the server, which cuts the
    /// text into items (ADR 011). <paramref name="html"/> is empty when the clipboard has no
    /// HTML. A raw paste puts the text into the item as it is.
    /// </summary>
    public void Paste(string itemId, EditorState editor, string text, string html, bool raw)
    {
        ArgumentNullException.ThrowIfNull(editor);
        // The core counts in characters, and WinUI in UTF-16 units (ADR 043)
        var cursor = PetaCore.PetaNative.CharsOfUtf16(editor.Text, (ulong)editor.SelectionStart);
        Send(new Event.PasteText(itemId, text, html, cursor, raw));
    }

    /// <summary>
    /// The user pressed a key while items are selected. No editor has the cursor then.
    /// </summary>
    public void Selection(SelectionKey key) => Send(new Event.SelectionKey(key));

    /// <summary>
    /// The user turned to something else, such as the text of an item: nothing stays selected.
    /// </summary>
    public void LeaveSelection() => Send(new Event.SelectionClickOutside());

    private static Event? EventOf(EditorKey key, string itemId, EditorState editor)
    {
        // The core counts in characters, and WinUI in UTF-16 units (ADR 043)
        var cursor = PetaCore.PetaNative.CharsOfUtf16(editor.Text, (ulong)editor.SelectionStart);
        var onlyCursor = editor.SelectionLength == 0;
        var atStart = onlyCursor && editor.SelectionStart == 0;
        var atEnd = onlyCursor && editor.SelectionStart == editor.Text.Length;
        return key switch
        {
            EditorKey.Enter => new Event.EnterPressed(itemId, cursor, NowMs()),
            EditorKey.Tab or EditorKey.MoveIn => new Event.Indent(itemId),
            EditorKey.BackTab or EditorKey.MoveOut => new Event.Outdent(itemId),
            EditorKey.Up => editor.OnFirstLine ? new Event.NavigateUp(itemId, cursor) : null,
            EditorKey.Down => editor.OnLastLine ? new Event.NavigateDown(itemId, cursor) : null,
            // A place far past any text: the core puts the cursor at the end of the item before
            EditorKey.Left => atStart ? new Event.NavigateUp(itemId, long.MaxValue) : null,
            EditorKey.Right => atEnd ? new Event.NavigateDown(itemId, 0) : null,
            EditorKey.Backspace => atStart ? new Event.BackspaceAtStart(itemId) : null,
            EditorKey.Delete => atEnd ? new Event.DeleteAtEnd(itemId) : null,
            EditorKey.TaskCycle => new Event.TaskCycle(itemId),
            EditorKey.SelectUp => editor.OnFirstLine ? new Event.ShiftSelect(itemId) : null,
            EditorKey.SelectDown => editor.OnLastLine ? new Event.ShiftSelect(itemId) : null,
            EditorKey.Fold => new Event.SetCollapsed(itemId, true),
            EditorKey.Unfold => new Event.SetCollapsed(itemId, false),
            EditorKey.MoveUp => new Event.MoveUp(itemId),
            EditorKey.MoveDown => new Event.MoveDown(itemId),
            EditorKey.Undo => new Event.Undo(),
            EditorKey.Redo => new Event.Redo(),
            _ => throw new ArgumentOutOfRangeException(nameof(key)),
        };
    }
}

/// <summary>The search box. What it found is in <c>View.Search</c>: the core asks the server a
/// moment after the last key.</summary>
public sealed partial class Shell
{
    /// <summary>The user changed the text of the search box.</summary>
    public void Search(string query) => Send(new Event.SearchQueryChanged(query));

    /// <summary>The user pressed a key in the search box that goes through what was found.</summary>
    public void Search(SearchKey key) => Send(new Event.SearchKey(key));

    /// <summary>
    /// The user chose an entry of what was found. <paramref name="index"/> is the one the entry
    /// carries.
    /// </summary>
    public void Pick(ulong index) => Send(new Event.SearchResultClicked(index));
}
