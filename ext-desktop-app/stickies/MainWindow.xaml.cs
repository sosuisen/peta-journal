using Microsoft.UI;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Stickies.Services;
using ProtoBlock = Petajournal.Block;
using ProtoPage = Petajournal.Page;
using Petajournal;
using Windows.Graphics;
using WinRT.Interop;

namespace Stickies;

public sealed partial class MainWindow : Window
{
    private readonly JournalApiService _api;
    private readonly SettingsService _settingsService;
    private StickyWindowSettings _windowSettings;
    private DispatcherTimer? _refreshTimer;
    private DispatcherTimer? _debounceTimer;
    private string? _currentPageId;
    private string _backgroundColor = "#FFFDD0";
    private AppWindow? _appWindow;
    private bool _initialized;

    public string WindowId => _windowSettings.Id;

    public MainWindow(JournalApiService api, SettingsService settingsService, StickyWindowSettings? settings = null)
    {
        _api = api;
        _settingsService = settingsService;
        _windowSettings = settings ?? settingsService.AddWindow();

        InitializeComponent();

        SetupTitleBar();
        SetupAppWindow();
        RestoreState();
        SetupRefreshTimer();

        Activated += OnActivated;
        Closed += OnClosed;
    }

    private void SetupTitleBar()
    {
        ExtendsContentIntoTitleBar = true;
        SetTitleBar(TitleBarRow);
    }

    private void SetupAppWindow()
    {
        var hwnd = WindowNative.GetWindowHandle(this);
        var windowId = Win32Interop.GetWindowIdFromWindow(hwnd);
        _appWindow = AppWindow.GetFromWindowId(windowId);

        if (_appWindow != null)
        {
            _appWindow.Changed += AppWindow_Changed;

            if (_appWindow.Presenter is OverlappedPresenter presenter)
            {
                presenter.IsMinimizable = false;
                presenter.IsMaximizable = false;
            }
        }
    }

    private void RestoreState()
    {
        _backgroundColor = _windowSettings.BackgroundColor;
        _currentPageId = _windowSettings.PageId;

        if (_appWindow != null)
        {
            _appWindow.MoveAndResize(new RectInt32(
                _windowSettings.X, _windowSettings.Y,
                _windowSettings.Width, _windowSettings.Height));
        }
    }

    private async Task InitializeContentAsync()
    {
        if (_initialized) return;
        _initialized = true;

        await EnsureWebViewInitialized();

        if (!string.IsNullOrEmpty(_currentPageId))
        {
            await LoadPageAsync(_currentPageId);
        }
        else
        {
            ShowSearch();
            ShowEmptyContent();
        }
    }

    private void SetupRefreshTimer()
    {
        _refreshTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(10) };
        _refreshTimer.Tick += async (_, _) =>
        {
            if (!string.IsNullOrEmpty(_currentPageId))
            {
                await LoadPageAsync(_currentPageId, isRefresh: true);
            }
        };
        _refreshTimer.Start();
    }

    // --- Title bar visibility on focus/unfocus ---

    private void OnActivated(object sender, WindowActivatedEventArgs args)
    {
        if (args.WindowActivationState == WindowActivationState.Deactivated)
        {
            TitleBarRow.Visibility = Visibility.Collapsed;
            SearchPanel.Visibility = Visibility.Collapsed;
            _ = ExecuteScriptAsync("document.body.style.overflow='hidden'");
        }
        else
        {
            TitleBarRow.Visibility = Visibility.Visible;
            _ = ExecuteScriptAsync("document.body.style.overflow='auto'");

            _ = InitializeContentAsync();
        }
    }

    // --- Search ---

    private void SearchButton_Click(object sender, RoutedEventArgs e)
    {
        if (SearchPanel.Visibility == Visibility.Visible)
        {
            SearchPanel.Visibility = Visibility.Collapsed;
        }
        else
        {
            ShowSearch();
        }
    }

    private void ShowSearch()
    {
        SearchPanel.Visibility = Visibility.Visible;
        SearchBox.Text = "";
        SearchBox.Focus(FocusState.Programmatic);
    }

    private void SearchBox_TextChanged(object sender, TextChangedEventArgs e)
    {
        _debounceTimer?.Stop();
        _debounceTimer = new DispatcherTimer { Interval = TimeSpan.FromMilliseconds(300) };
        _debounceTimer.Tick += async (_, _) =>
        {
            _debounceTimer.Stop();
            var query = SearchBox.Text.Trim();
            if (query.Length > 0)
            {
                await PerformSearch(query);
            }
            else
            {
                SearchResultsList.ItemsSource = null;
            }
        };
        _debounceTimer.Start();
    }

    private async Task PerformSearch(string query)
    {
        try
        {
            if (!_api.IsConnected) await _api.ConnectAsync();
            var response = await _api.SearchPageNamesAsync(query);
            if (response != null)
            {
                SearchResultsList.ItemsSource = response.Results;
            }
        }
        catch
        {
            // Search failed silently
        }
    }

    private void SearchResultsList_ItemClick(object sender, ItemClickEventArgs e)
    {
        if (e.ClickedItem is SearchResult result)
        {
            SearchPanel.Visibility = Visibility.Collapsed;
            _currentPageId = result.PageId;
            _windowSettings.PageId = _currentPageId;
            _settingsService.UpdateWindow(_windowSettings);
            _ = LoadPageAsync(_currentPageId);
        }
    }

    // --- Page loading & HTML rendering ---

    private async Task LoadPageAsync(string pageId, bool isRefresh = false)
    {
        try
        {
            if (!_api.IsConnected) await _api.ConnectAsync();
            var response = await _api.GetPageAsync(pageId);
            if (response == null) return;

            var page = response.Page;
            var blocks = response.Blocks;

            if (!isRefresh)
            {
                PageTitleText.Text = page?.FileName ?? "Untitled";
            }

            var html = BuildHtml(page, blocks);
            await EnsureWebViewInitialized();
            ContentWebView.NavigateToString(html);
        }
        catch
        {
            if (!isRefresh) ShowErrorContent("Failed to load page");
        }
    }

    private async Task ExecuteScriptAsync(string script)
    {
        try
        {
            if (ContentWebView.CoreWebView2 != null)
                await ContentWebView.CoreWebView2.ExecuteScriptAsync(script);
        }
        catch { }
    }

    private async Task EnsureWebViewInitialized()
    {
        if (ContentWebView.CoreWebView2 == null)
        {
            await ContentWebView.EnsureCoreWebView2Async();
        }
    }

    private string BuildHtml(ProtoPage? page, IEnumerable<ProtoBlock> blocks)
    {
        var blocksHtml = string.Join("\n", blocks.Select(block =>
        {
            var indent = block.Level * 24;
            var checkbox = "";
            if (block.Status != "normal" && !string.IsNullOrEmpty(block.Status))
            {
                var label = block.Status switch
                {
                    "todo" => "&#x2610;",
                    "running" => "&#x25B6;",
                    "done" => "&#x2611;",
                    _ => ""
                };
                checkbox = $"<span style=\"margin-right:6px;font-size:16px;\">{label}</span>";
            }
            var content = !string.IsNullOrEmpty(block.Html) ? block.Html : EscapeHtml(block.Body);
            var style = checkbox.Length > 0
                ? $"padding-left:{indent}px;padding:4px 8px 4px {indent + 8}px;display:flex;align-items:baseline;"
                : $"padding-left:{indent}px;padding:4px 8px 4px {indent + 8}px;";
            return $"<div style=\"{style}\">{checkbox}<div>{content}</div></div>";
        }));

        return $@"<!DOCTYPE html>
<html>
<head>
<meta charset=""utf-8"">
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{
    font-family: -apple-system, 'Segoe UI', sans-serif;
    font-size: 14px;
    line-height: 1.6;
    background: {_backgroundColor};
    color: #333;
    padding: 12px;
    overflow-wrap: break-word;
  }}
  a {{ color: #2563eb; }}
  code {{
    background: rgba(0,0,0,0.06);
    padding: 1px 4px;
    border-radius: 3px;
    font-size: 13px;
  }}
  pre {{
    background: rgba(0,0,0,0.06);
    padding: 8px 12px;
    border-radius: 6px;
    overflow-x: auto;
    margin: 4px 0;
  }}
  pre code {{ background: none; padding: 0; }}
  h1 {{ font-size: 1.5em; margin: 8px 0 4px; }}
  h2 {{ font-size: 1.3em; margin: 8px 0 4px; }}
  h3 {{ font-size: 1.1em; margin: 8px 0 4px; }}
  blockquote {{
    border-left: 3px solid #ccc;
    padding-left: 12px;
    color: #666;
    margin: 4px 0;
  }}
  img {{ max-width: 100%; }}
  table {{ border-collapse: collapse; margin: 4px 0; }}
  th, td {{ border: 1px solid #ddd; padding: 4px 8px; }}
  ul, ol {{ padding-left: 24px; }}
</style>
</head>
<body>
{blocksHtml}
</body>
</html>";
    }

    private void ShowEmptyContent()
    {
        var html = $@"<!DOCTYPE html>
<html><head><meta charset=""utf-8"">
<style>
  body {{
    font-family: 'Segoe UI', sans-serif;
    display: flex; justify-content: center; align-items: center;
    height: 100vh; margin: 0;
    background: {_backgroundColor}; color: #999;
  }}
</style></head>
<body><p>Click the search icon to find a page</p></body></html>";
        _ = ShowHtmlAsync(html);
    }

    private void ShowErrorContent(string message)
    {
        var html = $@"<!DOCTYPE html>
<html><head><meta charset=""utf-8"">
<style>
  body {{
    font-family: 'Segoe UI', sans-serif;
    display: flex; justify-content: center; align-items: center;
    height: 100vh; margin: 0;
    background: {_backgroundColor}; color: #cc3333;
  }}
</style></head>
<body><p>{EscapeHtml(message)}</p></body></html>";
        _ = ShowHtmlAsync(html);
    }

    private async Task ShowHtmlAsync(string html)
    {
        await EnsureWebViewInitialized();
        ContentWebView.NavigateToString(html);
    }

    private static string EscapeHtml(string text)
    {
        return text.Replace("&", "&amp;").Replace("<", "&lt;").Replace(">", "&gt;");
    }

    // --- New Window ---

    private void NewWindowButton_Click(object sender, RoutedEventArgs e)
    {
        App.CreateNewWindow();
    }

    // --- Background Color ---

    private void ColorMenuItem_Click(object sender, RoutedEventArgs e)
    {
        if (sender is MenuFlyoutItem item && item.Tag is string color)
        {
            _backgroundColor = color;
            _windowSettings.BackgroundColor = color;
            _settingsService.UpdateWindow(_windowSettings);

            // Refresh content with new color
            if (!string.IsNullOrEmpty(_currentPageId))
            {
                _ = LoadPageAsync(_currentPageId);
            }
            else
            {
                ShowEmptyContent();
            }
        }
    }

    // --- Window state persistence ---

    private void AppWindow_Changed(AppWindow sender, AppWindowChangedEventArgs args)
    {
        if (args.DidPositionChange || args.DidSizeChange)
        {
            var pos = sender.Position;
            var size = sender.Size;
            _windowSettings.X = pos.X;
            _windowSettings.Y = pos.Y;
            _windowSettings.Width = size.Width;
            _windowSettings.Height = size.Height;
            _settingsService.UpdateWindow(_windowSettings);
        }
    }

    private void OnClosed(object sender, WindowEventArgs args)
    {
        _refreshTimer?.Stop();
        _settingsService.UpdateWindow(_windowSettings);
        App.OnWindowClosed(this);
    }
}
