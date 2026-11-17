using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Markup;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.XamlTypeInfo;
using Microsoft.VisualStudio.TestTools.UnitTesting.AppContainer;

[assembly: WinUITestTarget(typeof(PetaScreens.Tests.TestApp))]

namespace PetaScreens.Tests;

/// <summary>
/// The app that the tests run in. MSTest starts it, and runs the tests marked
/// [UITestMethod] on its UI thread (ADR 059). It has the resources of App.xaml of PetaStickies.
/// </summary>
public sealed partial class TestApp : Application, IXamlMetadataProvider
{
    // An app without XAML has none of the types of XAML that the controls need. Without them,
    // XamlControlsResources cannot be read
    private readonly XamlControlsXamlMetaDataProvider controls = new();

    public IXamlType GetXamlType(Type type) => controls.GetXamlType(type);

    public IXamlType GetXamlType(string fullName) => controls.GetXamlType(fullName);

    public XmlnsDefinition[] GetXmlnsDefinitions() => controls.GetXmlnsDefinitions();

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        // The tests open stickies and close them. The UI thread must go on when the last one
        // is closed, so that the test that closed it can end
        DispatcherShutdownMode = DispatcherShutdownMode.OnExplicitShutdown;
        // An exception that the app does not handle ends the process with no word of what it
        // was. A test that ends so is failed by MSTest, and this tells why
        UnhandledException += (_, e) => Console.Error.WriteLine($"unhandled: {e.Exception}"); // Not in the constructor: the resources of an app cannot be read before it is launched
        Resources.MergedDictionaries.Add(new XamlControlsResources());
        Resources["ContentControlThemeFontFamily"] = new FontFamily("Segoe UI, Noto Sans JP");
    }
}
