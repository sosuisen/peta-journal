using System.Collections.ObjectModel;
using System.Runtime.CompilerServices;
using System.Text.Json;
using PetaTypes;

namespace PetaBorder.Tests;

/// <summary>
/// The examples of the core that every shell applies as the core does (ADR 089, ADR 090 D1).
/// They are in <c>client/tools/shell-check/common/list-changes.json</c>, which the core writes.
/// No journal server is needed.
/// </summary>
public sealed class ListChangesTests
{
    /// <summary>The file of the examples, found from this file in the repository.</summary>
    private static string ExamplesFile([CallerFilePath] string here = "") =>
        Path.GetFullPath(
            Path.Combine(
                Path.GetDirectoryName(here)!,
                "../../../../tools/shell-check/common/list-changes.json"
            )
        );

    /// <summary>The changes in the hexadecimal <paramref name="hex"/>, read as the shell reads them from the core.</summary>
    private static Changes ChangesOf(string hex) =>
        Changes.BincodeDeserialize(Convert.FromHexString(hex));

    /// <summary>The rows that the changes in <paramref name="hex"/> make of <paramref name="rows"/>, as the shell makes them.</summary>
    private static ObservableCollection<Row> Applied(ObservableCollection<Row> rows, string hex)
    {
        ListChanges.Apply(rows, ChangesOf(hex).Rows, row => row, (at, row) => rows[at] = row);
        return rows;
    }

    /// <summary>The rows as the bytes that cross the border: a generated row has no equality of its own.</summary>
    private static List<string> Bytes(IEnumerable<Row> rows) =>
        [.. rows.Select(row => Convert.ToHexString(row.BincodeSerialize()))];

    // コアの例のどれも、シェルが当てると、コアの行になる。回した例は、ファイルの例のすべてである。
    [Fact]
    public void The_shell_applies_every_example_of_the_core_as_the_core_does()
    {
        using var file = JsonDocument.Parse(File.ReadAllText(ExamplesFile()));
        var examples = file.RootElement.GetProperty("native").GetProperty("rows");
        var ran = new List<string>();

        foreach (var example in examples.EnumerateArray())
        {
            var id = example.GetProperty("id").GetString()!;
            var before = Applied([], example.GetProperty("before").GetString()!);
            var after = Applied([], example.GetProperty("after").GetString()!);

            var applied = Applied(before, example.GetProperty("changes").GetString()!);

            Assert.True(
                Bytes(applied).SequenceEqual(Bytes(after)),
                $"{id}: the rows are not those of the core"
            );
            ran.Add(id);
        }

        Assert.NotEmpty(ran);
        Assert.Equal(examples.EnumerateArray().Select(e => e.GetProperty("id").GetString()), ran);
    }
}
