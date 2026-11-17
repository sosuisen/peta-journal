namespace PetaBorder.Tests;

/// <summary>
/// Skips a test, as xUnit does it. <see cref="JournalServer"/> is shared with the tests of
/// PetaScreens, which run on MSTest and have their own <c>SkipTest</c>.
/// </summary>
internal static class SkipTest
{
    public static void When(bool condition, string reason) => Assert.SkipWhen(condition, reason);
}
