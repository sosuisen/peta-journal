using System.Diagnostics;
using System.Globalization;
using Microsoft.VisualStudio.TestTools.UnitTesting.AppContainer;
using PetaTypes;

namespace PetaScreens.Tests;

/// <summary>
/// Measures how long one key takes on a long page, before and after the list diffs of ADR 068
/// (#824). It prints the times and fails on none of them. It needs a real journal server, so it
/// is skipped without <c>PETA_E2E_SERVER</c>.
///
/// Run it with the Rust core built in release mode, as <c>generate.ps1</c> builds it. Each line
/// of the result starts with <c>SPEED</c>, and the lines are also written to
/// <c>peta-typing-speed.txt</c> in the folder for temporary files.
/// </summary>
[TestClass]
public sealed class TypingSpeedTests
{
    private const int Port = 18115;
    private const int Items = 1000;
    private const int Keys = 100;

    private static readonly string Results = Path.Combine(
        Path.GetTempPath(),
        "peta-typing-speed.txt"
    );

    // 1000項目のページで、1つのキーにかかる時間を測る。
    [UITestMethod]
    public async Task A_key_on_a_long_page()
    {
        await using var screen = await Screen.OpenLong(Port, Items);
        System.IO.File.WriteAllText(
            Results,
            $"{DateTime.Now:yyyy-MM-dd HH:mm} {Items} items, {Keys} keys\n"
        );

        MeasureTheBorder(screen);
        await MeasureTheWindow(screen);
    }

    /// <summary>
    /// The border alone, on a core of its own: the event into the core, the view model and the
    /// changes of the screen out of it as bytes, and the reading of the bytes in C#.
    /// </summary>
    private static void MeasureTheBorder(Screen screen)
    {
        using var core = new PetaCore.PetaCore(screen.Server, new Unheard());
        core.Open("/pages/long");
        var waited = Stopwatch.StartNew();
        while (ViewModel.BincodeDeserialize(core.View()).Outline.Rows.Count < Items)
        {
            Assert.IsTrue(waited.Elapsed < TimeSpan.FromSeconds(10), "the long page never came");
            Thread.Sleep(10);
        }
        var first = ViewModel.BincodeDeserialize(core.View()).Outline.Rows[0].Item.Id;
        core.Update(new Event.EditRequested(first, new EditPlace.Markdown(0)).BincodeSerialize());
        var events = new List<double>();
        var bytes = new List<double>();
        var reads = new List<double>();
        var changed = new List<double>();
        var changesRead = new List<double>();
        // The first changes hold the whole screen; those of a key are what is measured
        _ = core.Changes();
        for (var n = 1; n <= Keys; n++)
        {
            var typing = new Typing.Typed(first, new string('a', n), (ulong)n).BincodeSerialize();
            events.Add(Timed(() => core.Typed(typing)));
            byte[] view = [];
            bytes.Add(Timed(() => view = core.View()));
            reads.Add(Timed(() => ViewModel.BincodeDeserialize(view)));
            byte[] changes = [];
            changed.Add(Timed(() => changes = core.Changes()));
            changesRead.Add(Timed(() => Changes.BincodeDeserialize(changes)));
        }
        Report("Rust: event into the core", events);
        Report("Rust: view model out of the core as bytes", bytes);
        Report("C#: view model read from the bytes", reads);
        Report("Rust: changes out of the core as bytes (ADR 076)", changed);
        Report("C#: changes read from the bytes (ADR 076)", changesRead);
        // The last edit is not saved: this core goes with the test, and its server too
    }

    /// <summary>
    /// The whole of it: the editor of the sticky takes the text of a key, the shell hands it to
    /// the core and applies the changes of the screen, and the window is laid out again.
    /// </summary>
    private static async Task MeasureTheWindow(Screen screen)
    {
        await screen.Edit(0);
        var editor = screen.Editor;
        var root = (Microsoft.UI.Xaml.UIElement)screen.Window.Content;
        var shell = new List<double>();
        var drawn = new List<double>();
        for (var n = 1; n <= Keys; n++)
        {
            var body = new string('b', n);
            // The editor tells the shell while it changes, as it does for a key
            shell.Add(Timed(() => editor.Text = body));
            drawn.Add(Timed(root.UpdateLayout));
            // Lets the render request of the core arrive, as between two keys
            await Task.Delay(5);
        }
        Report("C#: the shell takes the key and applies the changes", shell);
        Report("WinUI: the window is laid out again", drawn);
        Assert.AreEqual(new string('b', Keys), screen.Rows[0].Item.Body);
    }

    private static double Timed(Action work)
    {
        var watch = Stopwatch.StartNew();
        work();
        return watch.Elapsed.TotalMilliseconds;
    }

    private static void Report(string name, List<double> ms)
    {
        var sorted = ms.Order().ToList();
        var line = string.Create(
            CultureInfo.InvariantCulture,
            $"SPEED {name}: median {sorted[sorted.Count / 2]:F2}ms, worst {sorted[^1]:F2}ms"
        );
        Console.WriteLine(line);
        System.IO.File.AppendAllText(Results, line + "\n");
    }

    /// <summary>A shell that does nothing with the requests of the core.</summary>
    private sealed class Unheard : PetaCore.PetaShell
    {
        public void ProcessEffects(byte[] bytes) { }
    }
}
