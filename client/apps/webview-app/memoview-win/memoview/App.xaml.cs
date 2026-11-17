using MemoView.Services;
using Microsoft.UI.Xaml;

namespace MemoView;

public partial class App : Application
{
    private static JournalApiService _api = null!;
    private static SettingsService _settingsService = null!;
    private static readonly List<MainWindow> _windows = new();

    public App()
    {
        InitializeComponent();
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        _api = new JournalApiService();
        _settingsService = new SettingsService();
        _settingsService.Load();

        if (_settingsService.Windows.Count > 0)
        {
            foreach (var ws in _settingsService.Windows.ToList())
            {
                var window = new MainWindow(_api, _settingsService, ws);
                _windows.Add(window);
                window.Activate();
            }
        }
        else
        {
            CreateNewWindow();
        }
    }

    public static void CreateNewWindow()
    {
        var window = new MainWindow(_api, _settingsService);
        _windows.Add(window);
        window.Activate();
    }

    public static void OnWindowClosed(MainWindow window)
    {
        _windows.Remove(window);
        if (_windows.Count == 0)
        {
            _settingsService.Save();
            _api.Dispose();
            Current.Exit();
        }
    }
}
