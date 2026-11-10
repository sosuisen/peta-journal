namespace Stickies.Models;

/// <summary>
/// An item as the app shows it. The server has more fields of an item, such as the Markdown
/// text. The app does not ask for them: it draws the text from <see cref="Ast"/>.
/// </summary>
public class ItemModel
{
    /// <summary>
    /// The fields the server is asked for, as the value of <c>fields</c> in the query.
    /// Keep it the same as the properties below that the server sends.
    /// </summary>
    public const string Fields = "id,level,ast,collapsed,types";

    public string Id { get; set; } = "";
    public int Level { get; set; }
    public string Ast { get; set; } = "";
    public bool Collapsed { get; set; }
    public string Status { get; set; } = "normal";
    public List<string> Types { get; set; } = [];
}
