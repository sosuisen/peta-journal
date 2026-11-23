using PetaTypes;

namespace PetaBorder;

/// <summary>The keys that are held down with another key.</summary>
[Flags]
public enum Held
{
    None = 0,
    Shift = 1,
    Control = 2,
    Alt = 4,
}

public static class Keyboard
{
    /// <summary>
    /// The key with the keys held down with it, as the core knows it (ADR 052). What the chord
    /// means is decided in the core.
    /// </summary>
    public static Chord Chord(Key key, Held held = Held.None) =>
        new()
        {
            Key = key,
            Shift = held.HasFlag(Held.Shift),
            Ctrl = held.HasFlag(Held.Control),
            Alt = held.HasFlag(Held.Alt),
            // Windows has no Command key
            Cmd = false,
        };
}

public sealed partial class Shell
{
    /// <summary>
    /// The user pressed a key. The core decides what it means (ADR 052). False when it means
    /// nothing to the outline there, and the editor does with it what it does in any text.
    /// </summary>
    public bool Pressed(KeyPress press)
    {
        if (!border.Press(press))
            return false;
        Show(border.Changes());
        return true;
    }

    /// <summary>
    /// A step of a drag of the mouse that started on the text of an item. The core decides when
    /// the drag leaves the text and selects items, and which items.
    /// </summary>
    public void Dragged(DragMove step)
    {
        if (border.Drag(step))
            Show(border.Changes());
    }

    /// <summary>
    /// The user pasted into the editor of an item. The core asks the server, which cuts the
    /// text into items (ADR 011). <paramref name="html"/> is empty when the clipboard has no
    /// HTML. A raw paste puts the text into the item as it is.
    /// </summary>
    public void Paste(string itemId, EditorState editor, string text, string html, bool raw)
    {
        ArgumentNullException.ThrowIfNull(editor);
        Send(new Event.PasteText(itemId, text, html, CursorOf(editor), raw));
    }

    /// <summary>
    /// The user pasted an image into the editor of an item. The shell wrote it to the file at
    /// <paramref name="path"/>, which the core uploads, links at the cursor and then deletes
    /// (ADR 071).
    /// </summary>
    public void PasteImage(string itemId, EditorState editor, string path)
    {
        ArgumentNullException.ThrowIfNull(editor);
        border.PasteImage(itemId, path, CursorOf(editor));
    }

    /// <summary>
    /// The user dropped files on the drawn item. The image goes at the end of its body: the
    /// drawn text and the Markdown do not line up.
    /// </summary>
    public void DropImage(string itemId, IReadOnlyList<string> paths)
    {
        ArgumentNullException.ThrowIfNull(paths);
        var body = View.Outline.Rows.First(row => row.Item.Id == itemId).Item.Body;
        // The core deletes the file it is given (ADR 071): it gets a copy with the same name
        var copy = Path.Combine(
            Directory.CreateTempSubdirectory("peta-dropped-").FullName,
            Path.GetFileName(paths[0])
        );
        File.Copy(paths[0], copy);
        border.PasteImage(itemId, copy, PetaCore.PetaNative.CharsOfUtf16(body, (ulong)body.Length));
    }

    /// <summary>Where the cursor is in the editor, in the characters that the core counts.</summary>
    private static ulong CursorOf(EditorState editor) =>
        // WinUI counts in UTF-16 units (ADR 043)
        PetaCore.PetaNative.CharsOfUtf16(editor.Text, editor.SelectionStart);
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
