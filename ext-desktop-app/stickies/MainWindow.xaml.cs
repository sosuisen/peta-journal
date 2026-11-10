using System.Runtime.InteropServices;
using Microsoft.UI;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using Stickies.Models;
using Stickies.Services;
using Windows.Graphics;
using WinRT.Interop;

namespace Stickies;

public sealed partial class MainWindow : Window
{
    private readonly JournalApiService _api;
    private readonly SettingsService _settingsService;
    private StickyWindowSettings _windowSettings;
    private DispatcherTimer? _debounceTimer;
    private AppWindow? _appWindow;
    private bool _initialized;
    private bool _isDeactivated;
    private bool _closedByUser;
    private List<ItemModel> _currentItems = new();
    private readonly HashSet<string> _collapsedItemIds = new();

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

        _api.PageUpdated += OnPageUpdated;
        _api.Connected += OnApiConnected;
        _api.Disconnected += OnApiDisconnected;
        _api.Reconnecting += OnApiReconnecting;
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
                presenter.SetBorderAndTitleBar(true, false);
                presenter.IsAlwaysOnTop = _windowSettings.AlwaysOnTop;
            }
        }
    }

    private void RestoreState()
    {
        UpdatePinIcon();

        if (_appWindow != null)
        {
            var scale = GetScaleFactor();
            _appWindow.MoveAndResize(new RectInt32(
                _windowSettings.X, _windowSettings.Y,
                (int)(_windowSettings.Width * scale), (int)(_windowSettings.Height * scale)));
        }
    }

    private async Task InitializeContentAsync()
    {
        if (_initialized) return;
        _initialized = true;

        await EnsureWebViewInitialized();

        if (!string.IsNullOrEmpty(_windowSettings.PageId))
        {
            await LoadPageAsync(_windowSettings.PageId);
        }
        else
        {
            ShowSearch();
            ShowEmptyContent();
        }
    }

    private void OnApiConnected()
    {
        DispatcherQueue.TryEnqueue(async () =>
        {
            ReconnectingText.Visibility = Visibility.Collapsed;
            PageTitleText.Visibility = Visibility.Visible;

            if (!string.IsNullOrEmpty(_windowSettings.PageId))
            {
                await LoadPageAsync(_windowSettings.PageId, isRefresh: true);
            }
        });
    }

    private void OnApiDisconnected()
    {
        DispatcherQueue.TryEnqueue(() =>
        {
            ReconnectingText.Visibility = Visibility.Visible;
            PageTitleText.Visibility = Visibility.Collapsed;
        });
    }

    private void OnApiReconnecting()
    {
        DispatcherQueue.TryEnqueue(() =>
        {
            ReconnectingText.Visibility = Visibility.Visible;
            PageTitleText.Visibility = Visibility.Collapsed;
        });
    }

    private void OnPageUpdated(string pageId)
    {
        if (pageId != _windowSettings.PageId) return;
        DispatcherQueue.TryEnqueue(async () =>
        {
            await LoadPageAsync(pageId, isRefresh: true);
        });
    }

    // --- Title bar visibility on focus/unfocus ---

    private void OnActivated(object sender, WindowActivatedEventArgs args)
    {
        if (args.WindowActivationState == WindowActivationState.Deactivated)
        {
            _isDeactivated = true;
            TitleBarRow.Visibility = Visibility.Collapsed;
            SearchPanel.Visibility = Visibility.Collapsed;
            _ = ExecuteScriptAsync("document.body.style.overflow='hidden'");
        }
        else
        {
            _isDeactivated = false;
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
        if (e.ClickedItem is SearchResultModel result)
        {
            SearchPanel.Visibility = Visibility.Collapsed;
            _windowSettings.PageId = result.PageId;
            _settingsService.UpdateWindow(_windowSettings);
            _ = LoadPageAsync(result.PageId);
        }
    }

    // --- Page loading & HTML rendering ---

    private async Task LoadPageAsync(string pageId, bool isRefresh = false)
    {
        try
        {
            if (!_api.IsConnected) await _api.ConnectAsync();
            var response = await _api.GetPageByIdAsync(pageId);
            if (response == null) return;

            _currentItems = response.Items.ToList();

            // Initialize collapsed state from server data
            _collapsedItemIds.Clear();
            foreach (var b in _currentItems)
            {
                if (b.Collapsed) _collapsedItemIds.Add(b.Id);
            }

            if (!isRefresh)
            {
                PageTitleText.Text = response.Page?.Name ?? "Untitled";
            }

            var html = BuildHtml();
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
            var core = ContentWebView.CoreWebView2!;
            core.NavigationStarting += (sender, e) =>
            {
                if (Uri.TryCreate(e.Uri, UriKind.Absolute, out var uri)
                    && uri.Scheme != "http" && uri.Scheme != "https"
                    && uri.Scheme != "about" && uri.Scheme != "data")
                {
                    e.Cancel = true;
                    _ = Windows.System.Launcher.LaunchUriAsync(uri);
                }
            };
            core.WebMessageReceived += (sender, e) => OnWebMessage(WebMessage.Parse(e.WebMessageAsJson));
            core.Settings.IsZoomControlEnabled = false;
        }
    }

    private void OnWebMessage(WebMessage? message)
    {
        switch (message)
        {
            case WebMessage.ZoomChanged zoom:
                _windowSettings.ZoomFactor = zoom.Factor;
                _settingsService.UpdateWindow(_windowSettings);
                break;
            case WebMessage.CollapseToggled toggled:
                if (!_collapsedItemIds.Remove(toggled.ItemId))
                    _collapsedItemIds.Add(toggled.ItemId);
                ContentWebView.NavigateToString(BuildHtml());
                break;
            case WebMessage.WikilinkClicked wikilink:
                _ = Windows.System.Launcher.LaunchUriAsync(
                    PageLinks.ForWikilink(_api.BaseUrl, wikilink.PageName));
                break;
            case WebMessage.HashtagClicked hashtag:
                _ = OpenTagPageAsync(hashtag.Tag);
                break;
        }
    }

    private async Task OpenTagPageAsync(string tag)
    {
        try
        {
            // The folder of the tag pages is asked each time, because the user can change it.
            var tagsDir = await _api.GetTagsDirAsync();
            await Windows.System.Launcher.LaunchUriAsync(
                PageLinks.ForHashtag(_api.BaseUrl, tagsDir, tag));
        }
        catch
        {
            // The server did not answer. The note stays as it is.
        }
    }

    private string BuildHtml()
    {
        var look = new PageLook(
            _windowSettings.BackgroundColor, _windowSettings.ZoomFactor, Scrolls: !_isDeactivated);
        return PageHtml.ForItems(
            OutlineHtml.Build(_currentItems, _collapsedItemIds),
            look,
            PageLinks.PagesUrl(_api.BaseUrl),
            EmbeddedPageAssets.Value);
    }

    private void ShowEmptyContent()
    {
        _ = ShowHtmlAsync(PageHtml.ForMessage(
            "Click the search icon to find a page", "#999", _windowSettings.BackgroundColor));
    }

    private void ShowErrorContent(string message)
    {
        _ = ShowHtmlAsync(PageHtml.ForMessage(message, "#cc3333", _windowSettings.BackgroundColor));
    }

    private async Task ShowHtmlAsync(string html)
    {
        await EnsureWebViewInitialized();
        ContentWebView.NavigateToString(html);
    }

    // --- New Window / Close ---

    private void NewWindowButton_Click(object sender, RoutedEventArgs e)
    {
        App.CreateNewWindow();
    }

    private void CloseButton_Click(object sender, RoutedEventArgs e)
    {
        _closedByUser = true;
        Close();
    }

    // --- Always on Top ---

    private void PinButton_Click(object sender, RoutedEventArgs e)
    {
        _windowSettings.AlwaysOnTop = !_windowSettings.AlwaysOnTop;
        if (_appWindow?.Presenter is OverlappedPresenter presenter)
        {
            presenter.IsAlwaysOnTop = _windowSettings.AlwaysOnTop;
        }
        UpdatePinIcon();
        _settingsService.UpdateWindow(_windowSettings);
    }

    private void UpdatePinIcon()
    {
        // E718 = Pin, E841 = Pinned
        PinIcon.Glyph = _windowSettings.AlwaysOnTop ? "\uE841" : "\uE718";
    }

    // --- Background Color ---

    private void ColorMenuItem_Click(object sender, RoutedEventArgs e)
    {
        if (sender is MenuFlyoutItem item && item.Tag is string color)
        {
            _windowSettings.BackgroundColor = color;
            _settingsService.UpdateWindow(_windowSettings);

            // Refresh content with new color
            if (!string.IsNullOrEmpty(_windowSettings.PageId))
            {
                _ = LoadPageAsync(_windowSettings.PageId);
            }
            else
            {
                ShowEmptyContent();
            }
        }
    }

    // --- DPI helpers ---

    [DllImport("user32.dll")]
    private static extern uint GetDpiForWindow(nint hwnd);

    private double GetScaleFactor()
    {
        // Prefer XamlRoot when available (after XAML tree is loaded)
        var xamlScale = Content?.XamlRoot?.RasterizationScale;
        if (xamlScale.HasValue) return xamlScale.Value;

        // Fallback to Win32 API (works before XAML tree is ready)
        var hwnd = WindowNative.GetWindowHandle(this);
        return GetDpiForWindow(hwnd) / 96.0;
    }

    // --- Window state persistence ---

    private void AppWindow_Changed(AppWindow sender, AppWindowChangedEventArgs args)
    {
        if (args.DidPositionChange || args.DidSizeChange)
        {
            var scale = GetScaleFactor();
            var pos = sender.Position;
            var size = sender.Size;
            _windowSettings.X = pos.X;
            _windowSettings.Y = pos.Y;
            _windowSettings.Width = (int)(size.Width / scale);
            _windowSettings.Height = (int)(size.Height / scale);
            _settingsService.UpdateWindow(_windowSettings);
        }
    }

    private void OnClosed(object sender, WindowEventArgs args)
    {
        _api.PageUpdated -= OnPageUpdated;
        _api.Connected -= OnApiConnected;
        _api.Disconnected -= OnApiDisconnected;
        _api.Reconnecting -= OnApiReconnecting;
        if (_appWindow != null) _appWindow.Changed -= AppWindow_Changed;
        // Only the close button throws a sticky away. A window also closes when Windows
        // or another program ends the app, and then the sticky must come back at the next start.
        if (_closedByUser)
            _settingsService.RemoveWindow(_windowSettings.Id);
        else
            _settingsService.UpdateWindow(_windowSettings);
        App.OnWindowClosed(this);
    }
}
