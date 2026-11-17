using Microsoft.UI.Xaml;
using PetaBorder;
using PetaScreens;

namespace PetaStickies;

/// <summary>
/// The app. It needs a running journal server, which <c>-server</c> on the command line
/// chooses. The stickies of the run before come back. <c>-stickies</c> names the file that
/// keeps them. With <c>-url</c> the app shows only that page instead, and keeps nothing.
/// </summary>
public partial class App : Application
{
    private StickyDesk? desk;

    public App()
    {
        InitializeComponent();
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        var given = Environment.GetCommandLineArgs();
        var server = Value("-server", given) ?? "http://localhost:8087";
        if (Value("-url", given) is { } url)
        {
            desk = new StickyDesk(server);
            desk.Add(url);
        }
        else
        {
            var file = Value("-stickies", given) ?? StickyStore.StandardFile;
            desk = new StickyDesk(server, new StickyStore(file));
            desk.Open();
        }
    }

    /// <summary>What stands behind the name on the command line.</summary>
    private static string? Value(string name, string[] given)
    {
        var place = Array.IndexOf(given, name);
        return place >= 0 && place + 1 < given.Length ? given[place + 1] : null;
    }
}
