using System.Text.Json;
using Stickies.Models;

namespace Stickies.Tests;

public class ItemModelTests
{
    private static readonly JsonSerializerOptions ServerJson = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    private static ItemModel FromServer(string json)
    {
        return JsonSerializer.Deserialize<ItemModel>(json, ServerJson)!;
    }

    // タスクではない項目には、タスクの状態がない。
    [Fact]
    public void An_item_that_is_not_a_task_has_no_task_state()
    {
        var item = FromServer("""{"types":[],"properties":{"begin":"2026-11-10T10:06:51Z"}}""");

        Assert.Null(item.TaskState);
    }

    // 開始も終了もないタスクは、TODOである。
    [Fact]
    public void A_task_without_begin_and_end_is_todo()
    {
        var item = FromServer("""{"types":["task"],"properties":{"plan":"2026-11-10"}}""");

        Assert.Equal(TaskState.Todo, item.TaskState);
    }

    // 開始だけがあるタスクは、RUNNINGである。
    [Fact]
    public void A_task_with_only_begin_is_running()
    {
        var item = FromServer("""{"types":["task"],"properties":{"begin":"2026-11-10T14:44:42Z"}}""");

        Assert.Equal(TaskState.Running, item.TaskState);
    }

    // 開始と終了があるタスクは、DONEである。
    [Fact]
    public void A_task_with_begin_and_end_is_done()
    {
        var item = FromServer(
            """{"types":["task"],"properties":{"begin":"2026-11-09T16:40:19Z","end":"2026-11-09T16:40:19Z"}}""");

        Assert.Equal(TaskState.Done, item.TaskState);
    }

    // 終了だけがあるタスクも、DONEである。
    [Fact]
    public void A_task_with_only_end_is_done()
    {
        var item = FromServer("""{"types":["task"],"properties":{"end":"2026-03-02T14:56:55"}}""");

        Assert.Equal(TaskState.Done, item.TaskState);
    }

    // 値が空の時刻は、ないものとして扱う。
    [Fact]
    public void A_time_with_an_empty_value_counts_as_not_set()
    {
        var item = FromServer("""{"types":["task"],"properties":{"begin":"2026-11-10T14:44:42Z","end":""}}""");

        Assert.Equal(TaskState.Running, item.TaskState);
    }

    // サーバーに頼むフィールドには、状態を決めるtypesとpropertiesが入っている。
    [Fact]
    public void The_fields_asked_of_the_server_hold_types_and_properties()
    {
        var fields = ItemModel.Fields.Split(',');

        Assert.Contains("types", fields);
        Assert.Contains("properties", fields);
    }
}
