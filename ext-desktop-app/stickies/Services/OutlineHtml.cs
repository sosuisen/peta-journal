using Stickies.Models;

namespace Stickies.Services;

/// <summary>
/// Builds the HTML of the items of a page, one row for each item that is shown.
/// The text of an item is not drawn here. A row carries the AST of its item in
/// <c>data-ast</c>, and the script of the page turns it into HTML.
/// </summary>
internal static class OutlineHtml
{
    /// <summary>
    /// The rows of the items, joined by line breaks.
    /// </summary>
    /// <param name="items">The items of the page, in the order of the page.</param>
    /// <param name="collapsedItemIds">The ids of the collapsed items. Their children are not shown.</param>
    internal static string Build(IList<ItemModel> items, IReadOnlySet<string> collapsedItemIds)
    {
        ArgumentNullException.ThrowIfNull(items);
        ArgumentNullException.ThrowIfNull(collapsedItemIds);

        var parentIds = ParentIds(items);
        return string.Join("\n", VisibleItems(items, collapsedItemIds).Select(item =>
            Row(item, parentIds.Contains(item.Id), collapsedItemIds.Contains(item.Id))));
    }

    private static HashSet<string> ParentIds(IList<ItemModel> items)
    {
        var parentIds = new HashSet<string>();
        for (int i = 0; i < items.Count - 1; i++)
        {
            if (items[i + 1].Level > items[i].Level)
                parentIds.Add(items[i].Id);
        }
        return parentIds;
    }

    // Leaves out the children of the collapsed items (same logic as frontend visibleItems).
    private static List<ItemModel> VisibleItems(IList<ItemModel> items, IReadOnlySet<string> collapsedItemIds)
    {
        var visibleItems = new List<ItemModel>();
        int skipLevel = -1;
        foreach (var item in items)
        {
            if (skipLevel >= 0 && item.Level > skipLevel) continue;
            skipLevel = -1;
            visibleItems.Add(item);
            if (collapsedItemIds.Contains(item.Id))
                skipLevel = item.Level;
        }
        return visibleItems;
    }

    private static string Row(ItemModel item, bool hasChildren, bool isCollapsed)
    {
        var indent = (item.Level - 1) * 24;
        if (HasNoText(item.Ast))
        {
            return $"<div style=\"padding-left:{indent + 8}px;height:1.6em;\"></div>";
        }

        var marker = Marker(item, hasChildren, isCollapsed);
        var astAttr = $" data-ast=\"{EscapeHtmlAttr(item.Ast)}\"";
        var style = marker.Length > 0
            ? $"padding-left:{indent}px;padding:4px 8px 4px {indent + 8}px;display:flex;align-items:baseline;"
            : $"padding-left:{indent}px;padding:4px 8px 4px {indent + 8}px;";
        return $"<div style=\"{style}\">{marker}<div{astAttr}></div></div>";
    }

    private static string Marker(ItemModel item, bool hasChildren, bool isCollapsed)
    {
        // The arrow and the empty slot have the same width, so the checkboxes of the tasks
        // with and without children stand in one column.
        const string slotStyle = "display:inline-block;width:10px;flex-shrink:0;margin-right:6px;";
        var arrow = isCollapsed ? "&#x25B6;" : "&#x25BC;";
        var toggle = $"<span class=\"collapse-toggle\" data-item-id=\"{item.Id}\" style=\"{slotStyle}color:#666;cursor:pointer;font-size:10px;user-select:none;\">{arrow}</span>";

        if (item.TaskState is { } taskState)
        {
            // The same checkbox as the web app: empty for todo, a line for running, a check
            // for done. HTML has no attribute for the line, so the script of the page sets it.
            var state = taskState switch
            {
                TaskState.Running => " data-running",
                TaskState.Done => " checked",
                _ => ""
            };
            // A checkbox takes the place of the bullet, so the arrow needs a place of its own,
            // to the left of the checkbox.
            var slot = hasChildren ? toggle : $"<span style=\"{slotStyle}\"></span>";
            return $"{slot}<input type=\"checkbox\" class=\"task-checkbox\" tabindex=\"-1\"{state}>";
        }
        if (hasChildren) return toggle;
        if (!item.Types.Contains("heading"))
        {
            return "<span style=\"margin-right:6px;color:#999;\">&#x2022;</span>";
        }
        return "";
    }

    /// <summary>
    /// Whether the AST of an item has no text. The server sends an empty AST for an empty
    /// item, and a document without children for an item of only spaces.
    /// </summary>
    private static bool HasNoText(string ast)
    {
        if (string.IsNullOrEmpty(ast)) return true;
        using var document = System.Text.Json.JsonDocument.Parse(ast);
        return !document.RootElement.TryGetProperty("children", out var children)
            || children.GetArrayLength() == 0;
    }

    private static string EscapeHtmlAttr(string text)
    {
        return text.Replace("&", "&amp;").Replace("\"", "&quot;").Replace("<", "&lt;").Replace(">", "&gt;");
    }
}
