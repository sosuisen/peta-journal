// The namespace of JournalServer, which is shared with PetaBorder.Tests
namespace PetaBorder.Tests;

/// <summary>Skips a test, as MSTest does it: the test is counted as inconclusive.</summary>
internal static class SkipTest
{
    public static void When(bool condition, string reason)
    {
        if (condition)
            Assert.Inconclusive(reason);
    }
}
