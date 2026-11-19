using System.Collections.Concurrent;
using System.Diagnostics;

namespace PetaBorder.Tests;

/// <summary>
/// The main thread of a test. The core calls from its own threads; what it hands over waits
/// here until the test turns to it, as it waits for the main thread in the app.
/// </summary>
internal sealed class MainThread
{
    private readonly ConcurrentQueue<Action> waiting = new();

    public void Post(Action work) => waiting.Enqueue(work);

    /// <summary>Waits until the core hands something over, and does not run it.</summary>
    public void WaitForWork()
    {
        var waited = Stopwatch.StartNew();
        while (waiting.IsEmpty)
        {
            Assert.True(waited.Elapsed < TimeSpan.FromSeconds(10), "the core handed nothing over");
            Thread.Sleep(10);
        }
    }

    /// <summary>Runs what waits.</summary>
    public void Turn()
    {
        while (waiting.TryDequeue(out var work))
            work();
    }

    /// <summary>
    /// Waits for what the core and the server do. A test that waits in vain fails here.
    /// </summary>
    public void WaitUntil(string what, Func<bool> done)
    {
        var waited = Stopwatch.StartNew();
        while (true)
        {
            Turn();
            if (done())
                return;
            Assert.True(waited.Elapsed < TimeSpan.FromSeconds(10), $"never happened: {what}");
            Thread.Sleep(10);
        }
    }
}
