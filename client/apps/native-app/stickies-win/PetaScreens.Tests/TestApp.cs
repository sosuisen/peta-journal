using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Markup;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.XamlTypeInfo;
using Microsoft.VisualStudio.TestTools.UnitTesting.AppContainer;

namespace PetaScreens.Tests;

/// <summary>
/// Starts TestApp before the tests, and gives its UI thread to [UITestMethod] (ADR 059).
/// </summary>
[TestClass]
public static class TestAppStart
{
    [AssemblyInitialize]
    public static void Start(TestContext context)
    {
        var started = new TaskCompletionSource<DispatcherQueue>();
        var thread = new Thread(() =>
        {
            try
            {
                Application.Start(initialized =>
                {
                    try
                    {
                        var queue = DispatcherQueue.GetForCurrentThread();
                        SynchronizationContext.SetSynchronizationContext(
                            new DispatcherQueueSynchronizationContext(queue)
                        );
                        _ = new TestApp();
                        started.TrySetResult(queue);
                    }
                    catch (Exception e)
                    {
                        started.TrySetException(e);
                    }
                });
            }
            catch (Exception e)
            {
                started.TrySetException(e);
            }
        })
        {
            Name = "UI thread of the tests",
            // Application.Start does not return while the tests run. The process must end
            // when they are over
            IsBackground = true,
        };
        // Not [WinUITestTarget]: MSTest starts the app on a thread of the MTA then, as the
        // thread of an app is not. The clipboard of Windows fails on such a thread
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        UITestMethodAttribute.DispatcherQueue = started
            .Task.WaitAsync(TimeSpan.FromSeconds(30), context.CancellationToken)
            .GetAwaiter()
            .GetResult();
    }
}

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
