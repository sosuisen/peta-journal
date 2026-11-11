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
