using System.Text.Json;

namespace Stickies.Models;

/// <summary>
/// The state of a task.
/// </summary>
public enum TaskState
{
    Todo,
    Running,
    Done,
}

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
    public const string Fields = "id,level,ast,collapsed,types,properties";

    public string Id { get; set; } = "";
    public int Level { get; set; }
    public string Ast { get; set; } = "";
    public bool Collapsed { get; set; }
    public List<string> Types { get; set; } = [];
    public Dictionary<string, JsonElement> Properties { get; set; } = [];

    /// <summary>
    /// The state of the task, or null when the item is not a task.
    /// The server does not send the state. It follows from the times the task was begun and
    /// ended: a task with <c>end</c> is done, and a task with only <c>begin</c> is running.
    /// </summary>
    public TaskState? TaskState
    {
        get
        {
            if (!Types.Contains("task"))
                return null;
            if (HasProperty("end"))
                return Models.TaskState.Done;
            if (HasProperty("begin"))
                return Models.TaskState.Running;
            return Models.TaskState.Todo;
        }
    }

    // An empty value means that the time is not set.
    private bool HasProperty(string key)
    {
        if (!Properties.TryGetValue(key, out var value))
            return false;
        return value.ValueKind switch
        {
            JsonValueKind.Null or JsonValueKind.Undefined => false,
            JsonValueKind.String => value.GetString() != "",
            _ => true,
        };
    }
}
