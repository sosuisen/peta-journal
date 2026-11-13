using System.Text.Json;
using Stickies.Models;
using Stickies.Services;

namespace Stickies.Tests;

public class OutlineHtmlTests
{
    private const string Toggle = "class=\"collapse-toggle\"";
    private const string Checkbox = "class=\"task-checkbox\"";
    private const string Bullet = "&#x2022;";
    private const string OpenArrow = "&#x25BC;";
    private const string ClosedArrow = "&#x25B6;";

    private static readonly IReadOnlySet<string> NothingCollapsed = new HashSet<string>();

    private static string AstOf(string text)
    {
        return JsonSerializer.Serialize(
            new
            {
                type = "document",
                children = new[]
                {
                    new
                    {
                        type = "paragraph",
                        children = new[] { new { type = "text", value = text } },
                    },
                },
            }
        );
    }

    private static ItemModel Item(string id, int level, string text)
    {
        return new ItemModel
        {
            Id = id,
            Level = level,
            Ast = AstOf(text),
        };
    }

    private static ItemModel Task(string id, int level, string text, params string[] times)
    {
        var item = Item(id, level, text);
        item.Types = ["task"];
        foreach (var time in times)
        {
            item.Properties[time] = JsonSerializer.SerializeToElement("2026-11-10T10:06:51Z");
        }
        return item;
    }

    private static string[] Rows(IList<ItemModel> items, IReadOnlySet<string> collapsed)
    {
        return OutlineHtml.Build(items, collapsed).Split('\n');
    }

    // --- Which items are shown ---

    // 項目は、ページの順に1行ずつ出る。
    [Fact]
    public void Each_item_is_one_row_in_the_order_of_the_page()
    {
        var rows = Rows([Item("a", 1, "first"), Item("b", 1, "second")], NothingCollapsed);

        Assert.Equal(2, rows.Length);
        Assert.Contains("first", rows[0]);
        Assert.Contains("second", rows[1]);
    }

    // 折りたたまれた項目の子孫は出ない。そのあとの項目は出る。
    [Fact]
    public void The_descendants_of_a_collapsed_item_are_not_shown_and_the_items_after_them_are()
    {
        var items = new[]
        {
            Item("parent", 1, "parent"),
            Item("child", 2, "child"),
            Item("grandchild", 3, "grandchild"),
            Item("next", 1, "next"),
        };

        var rows = Rows(items, new HashSet<string> { "parent" });

        Assert.Equal(2, rows.Length);
        Assert.Contains("parent", rows[0]);
        Assert.Contains("next", rows[1]);
    }

    // --- The text of an item ---

    // 行は、項目のASTを属性として持つ。ASTの中の引用符は、属性を壊さない。
    [Fact]
    public void A_row_carries_the_AST_of_its_item_and_the_quotes_in_it_do_not_end_the_attribute()
    {
        var item = Item("a", 1, "say \"hi\" <b>");

        var row = Rows([item], NothingCollapsed)[0];

        var start = row.IndexOf("data-ast=\"", StringComparison.Ordinal) + "data-ast=\"".Length;
        var end = row.IndexOf('"', start);
        var attribute = System.Net.WebUtility.HtmlDecode(row[start..end]);
        Assert.Equal(item.Ast, attribute);
    }

    // 本文のない項目は、ASTを持たない空の行になる。
    [Theory]
    [InlineData("")]
    [InlineData("""{"type":"document","children":[]}""")]
    public void An_item_without_text_is_an_empty_row_without_an_AST(string ast)
    {
        var item = new ItemModel
        {
            Id = "a",
            Level = 1,
            Ast = ast,
        };

        var row = Rows([item], NothingCollapsed)[0];

        Assert.DoesNotContain("data-ast", row);
        Assert.DoesNotContain(Bullet, row);
    }

    // --- Bullets and collapse arrows ---

    // 子のない項目には、黒丸が付く。
    [Fact]
    public void An_item_without_children_has_a_bullet()
    {
        var row = Rows([Item("a", 1, "text")], NothingCollapsed)[0];

        Assert.Contains(Bullet, row);
        Assert.DoesNotContain(Toggle, row);
    }

    // 見出しには、黒丸が付かない。
    [Fact]
    public void A_heading_has_no_bullet()
    {
        var heading = Item("a", 1, "title");
        heading.Types = ["heading"];

        var row = Rows([heading], NothingCollapsed)[0];

        Assert.DoesNotContain(Bullet, row);
    }

    // 子のある項目には、黒丸の代わりに、その項目のIDを持つ折りたたみの矢印が付く。
    [Fact]
    public void An_item_with_children_has_a_collapse_arrow_with_its_id_in_place_of_the_bullet()
    {
        var row = Rows([Item("parent", 1, "parent"), Item("child", 2, "child")], NothingCollapsed)[
            0
        ];

        Assert.Contains(Toggle, row);
        Assert.Contains("data-item-id=\"parent\"", row);
        Assert.DoesNotContain(Bullet, row);
    }

    // 矢印は、開いているときは下向き、折りたたまれているときは右向きである。
    [Fact]
    public void The_arrow_points_down_when_open_and_right_when_collapsed()
    {
        var items = new[] { Item("parent", 1, "parent"), Item("child", 2, "child") };

        var open = Rows(items, NothingCollapsed)[0];
        var collapsed = Rows(items, new HashSet<string> { "parent" })[0];

        Assert.Contains(OpenArrow, open);
        Assert.Contains(ClosedArrow, collapsed);
    }

    // --- Tasks ---

    // タスクではない項目には、チェックボックスが付かない。
    [Fact]
    public void An_item_that_is_not_a_task_has_no_checkbox()
    {
        var row = Rows([Item("a", 1, "text")], NothingCollapsed)[0];

        Assert.DoesNotContain(Checkbox, row);
    }

    // TODOのタスクには、黒丸の代わりに、空のチェックボックスが付く。
    [Fact]
    public void A_todo_task_has_an_empty_checkbox_in_place_of_the_bullet()
    {
        var row = Rows([Task("a", 1, "task")], NothingCollapsed)[0];

        Assert.Contains(Checkbox, row);
        Assert.DoesNotContain(" checked", row);
        Assert.DoesNotContain(" data-running", row);
        Assert.DoesNotContain(Bullet, row);
    }

    // RUNNINGのタスクのチェックボックスには、横線を入れるための印が付く。
    [Fact]
    public void The_checkbox_of_a_running_task_is_marked_to_get_the_line()
    {
        var row = Rows([Task("a", 1, "task", "begin")], NothingCollapsed)[0];

        Assert.Contains(Checkbox, row);
        Assert.Contains(" data-running", row);
        Assert.DoesNotContain(" checked", row);
    }

    // DONEのタスクのチェックボックスには、チェックが入る。
    [Fact]
    public void The_checkbox_of_a_done_task_is_checked()
    {
        var row = Rows([Task("a", 1, "task", "begin", "end")], NothingCollapsed)[0];

        Assert.Contains(Checkbox, row);
        Assert.Contains(" checked", row);
        Assert.DoesNotContain(" data-running", row);
    }

    // 子のあるタスクには、チェックボックスの左に、折りたたみの矢印が付く。
    [Fact]
    public void A_task_with_children_has_the_collapse_arrow_to_the_left_of_the_checkbox()
    {
        var row = Rows([Task("parent", 1, "task"), Item("child", 2, "child")], NothingCollapsed)[0];

        var toggle = row.IndexOf(Toggle, StringComparison.Ordinal);
        var checkbox = row.IndexOf(Checkbox, StringComparison.Ordinal);
        Assert.True(toggle >= 0, "The row has no collapse arrow.");
        Assert.True(toggle < checkbox, "The collapse arrow is not to the left of the checkbox.");
        Assert.Contains("data-item-id=\"parent\"", row);
    }

    // 子のないタスクには、折りたたみの矢印が付かない。
    [Fact]
    public void A_task_without_children_has_no_collapse_arrow()
    {
        var row = Rows([Task("a", 1, "task")], NothingCollapsed)[0];

        Assert.DoesNotContain(Toggle, row);
    }
}
