using System.Runtime.CompilerServices;
using System.Text.Json;
using PetaTypes;

namespace PetaBorder.Tests;

/// <summary>
/// The examples of the core of the values that cross the border of a native shell (ADR 090 D2).
/// They are in <c>client/tools/shell-check/common/border-native.json</c>, which the core writes:
/// every variant of <c>Changes</c> and <c>Event</c>, as their bytes in hexadecimal. No journal
/// server is needed.
/// </summary>
public sealed class BorderValuesTests
{
    /// <summary>One example: a value of the border, as its bytes in hexadecimal.</summary>
    private sealed record Example(string Id, string Bytes);

    /// <summary>The examples of the section <paramref name="section"/> of the file.</summary>
    private static List<Example> Examples(string section)
    {
        using var file = JsonDocument.Parse(File.ReadAllText(ExamplesFile()));
        return
        [
            .. file
                .RootElement.GetProperty(section)
                .EnumerateArray()
                .Select(example => new Example(
                    example.GetProperty("id").GetString()!,
                    example.GetProperty("bytes").GetString()!
                )),
        ];
    }

    /// <summary>The file of the examples, found in a folder above this file.</summary>
    private static string ExamplesFile([CallerFilePath] string here = "")
    {
        for (var folder = Directory.GetParent(here); folder is not null; folder = folder.Parent)
        {
            var file = Path.Combine(
                folder.FullName,
                "tools",
                "shell-check",
                "common",
                "border-native.json"
            );
            if (File.Exists(file))
            {
                return file;
            }
        }
        throw new FileNotFoundException("no tools/shell-check/common/border-native.json above");
    }

    /// <summary>
    /// Reads each example as the shell reads the border, writes it back as the shell writes it,
    /// and expects the bytes it began with. Gives the ids of the examples it ran.
    /// </summary>
    private static List<string> ReadAndWriteBack(
        List<Example> examples,
        Func<byte[], byte[]> readAndWrite
    )
    {
        List<string> ran = [];
        foreach (var example in examples)
        {
            var read = Convert.FromHexString(example.Bytes);

            var written = readAndWrite(read);

            Assert.True(read.AsSpan().SequenceEqual(written), example.Id);
            ran.Add(example.Id);
        }
        return ran;
    }

    // Changesのすべての例は、生成された型で読んで書き直すと、元のバイト列になる。回した例は、ファイルの例のすべてである。
    [Fact]
    public void The_shell_reads_and_writes_back_every_example_of_the_changes_as_it_was()
    {
        var examples = Examples("changes");

        var ran = ReadAndWriteBack(examples, bytes =>
            Changes.BincodeDeserialize(bytes).BincodeSerialize()
        );

        Assert.NotEmpty(ran);
        Assert.Equal(examples.Select(example => example.Id), ran);
    }

    // Eventのすべての例は、生成された型で読んで書き直すと、元のバイト列になる。回した例は、ファイルの例のすべてである。
    [Fact]
    public void The_shell_reads_and_writes_back_every_example_of_the_events_as_it_was()
    {
        var examples = Examples("events");

        var ran = ReadAndWriteBack(examples, bytes =>
            Event.BincodeDeserialize(bytes).BincodeSerialize()
        );

        Assert.NotEmpty(ran);
        Assert.Equal(examples.Select(example => example.Id), ran);
    }
}
