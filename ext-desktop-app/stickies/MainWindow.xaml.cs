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
    private string? _currentPageId;
    private string _backgroundColor = "#FFFDD0";
    private AppWindow? _appWindow;
    private bool _initialized;
    private bool _isDeactivated;
    private PageModel? _currentPage;
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
        _backgroundColor = _windowSettings.BackgroundColor;
        _currentPageId = _windowSettings.PageId;
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

    private void OnApiConnected()
    {
        DispatcherQueue.TryEnqueue(async () =>
        {
            ReconnectingText.Visibility = Visibility.Collapsed;
            PageTitleText.Visibility = Visibility.Visible;

            if (!string.IsNullOrEmpty(_currentPageId))
            {
                await LoadPageAsync(_currentPageId, isRefresh: true);
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
        if (pageId != _currentPageId) return;
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
            var response = await _api.GetPageByIdAsync(pageId);
            if (response == null) return;

            var page = response.Page;
            var items = response.Items;

            _currentPage = page;
            _currentItems = items.ToList();

            // Initialize collapsed state from server data
            _collapsedItemIds.Clear();
            foreach (var b in _currentItems)
            {
                if (b.Collapsed) _collapsedItemIds.Add(b.Id);
            }

            if (!isRefresh)
            {
                PageTitleText.Text = page?.Name ?? "Untitled";
            }

            var html = BuildHtml(page, _currentItems);
            await EnsureWebViewInitialized();
            ContentWebView.NavigateToString(html);
        }
        catch(Exception e)
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
            ContentWebView.CoreWebView2.NavigationStarting += (sender, e) =>
            {
                if (Uri.TryCreate(e.Uri, UriKind.Absolute, out var uri)
                    && uri.Scheme != "http" && uri.Scheme != "https"
                    && uri.Scheme != "about" && uri.Scheme != "data")
                {
                    e.Cancel = true;
                    _ = Windows.System.Launcher.LaunchUriAsync(uri);
                }
            };
            ContentWebView.CoreWebView2.WebMessageReceived += (sender, e) =>
            {
                try
                {
                    var msg = System.Text.Json.JsonSerializer.Deserialize<System.Text.Json.JsonElement>(e.WebMessageAsJson);
                    if (msg.TryGetProperty("zoom", out var zoomEl))
                    {
                        _windowSettings.ZoomFactor = zoomEl.GetDouble();
                        _settingsService.UpdateWindow(_windowSettings);
                    }
                    else if (msg.TryGetProperty("toggleCollapse", out var itemIdEl))
                    {
                        var itemId = itemIdEl.GetString();
                        if (itemId != null)
                        {
                            if (!_collapsedItemIds.Remove(itemId))
                                _collapsedItemIds.Add(itemId);
                            var html = BuildHtml(_currentPage, _currentItems);
                            ContentWebView.NavigateToString(html);
                        }
                    }
                    else if (msg.TryGetProperty("openWikilink", out var pageNameEl))
                    {
                        var pageName = pageNameEl.GetString();
                        if (!string.IsNullOrEmpty(pageName))
                            _ = Windows.System.Launcher.LaunchUriAsync(
                                PageLinks.ForWikilink(_api.BaseUrl, pageName));
                    }
                    else if (msg.TryGetProperty("openHashtag", out var tagEl))
                    {
                        var tag = tagEl.GetString();
                        if (!string.IsNullOrEmpty(tag)) _ = OpenTagPageAsync(tag);
                    }
                }
                catch { }
            };
            ContentWebView.CoreWebView2.Settings.IsZoomControlEnabled = false;
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

    private string BuildHtml(PageModel? page, IList<ItemModel> items)
    {
        // Build a set of item IDs that have children
        var hasChildrenSet = new HashSet<string>();
        for (int i = 0; i < items.Count - 1; i++)
        {
            if (items[i + 1].Level > items[i].Level)
                hasChildrenSet.Add(items[i].Id);
        }

        // Filter out children of collapsed items (same logic as frontend visibleItems)
        var visibleItems = new List<ItemModel>();
        int skipLevel = -1;
        foreach (var item in items)
        {
            if (skipLevel >= 0 && item.Level > skipLevel) continue;
            skipLevel = -1;
            visibleItems.Add(item);
            if (_collapsedItemIds.Contains(item.Id))
                skipLevel = item.Level;
        }

        var itemsHtml = string.Join("\n", visibleItems.Select(item =>
        {
            var indent = (item.Level - 1) * 24;
            var types = item.Types;
            var isHeading = types.Contains("heading");
            var isEmpty = HasNoText(item.Ast);
            var hasChildren = hasChildrenSet.Contains(item.Id);
            var isCollapsed = _collapsedItemIds.Contains(item.Id);

            var marker = "";
            if (item.Status != "normal" && !string.IsNullOrEmpty(item.Status))
            {
                var label = item.Status switch
                {
                    "todo" => "&#x2610;",
                    "running" => "&#x25B6;",
                    "done" => "&#x2611;",
                    _ => ""
                };
                marker = $"<span style=\"margin-right:6px;font-size:16px;\">{label}</span>";
            }
            else if (hasChildren)
            {
                var arrow = isCollapsed ? "&#x25B6;" : "&#x25BC;";
                marker = $"<span class=\"collapse-toggle\" data-item-id=\"{item.Id}\" style=\"margin-right:6px;color:#666;cursor:pointer;font-size:10px;user-select:none;\">{arrow}</span>";
            }
            else if (!isHeading && !isEmpty)
            {
                marker = "<span style=\"margin-right:6px;color:#999;\">&#x2022;</span>";
            }

            if (isEmpty)
            {
                return $"<div style=\"padding-left:{indent + 8}px;height:1.6em;\"></div>";
            }

            var astAttr = $" data-ast=\"{EscapeHtmlAttr(item.Ast)}\"";
            var style = marker.Length > 0
                ? $"padding-left:{indent}px;padding:4px 8px 4px {indent + 8}px;display:flex;align-items:baseline;"
                : $"padding-left:{indent}px;padding:4px 8px 4px {indent + 8}px;";
            return $"<div style=\"{style}\">{marker}<div{astAttr}></div></div>";
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
    overflow: {(_isDeactivated ? "hidden" : "auto")};
    zoom: {_windowSettings.ZoomFactor.ToString(System.Globalization.CultureInfo.InvariantCulture)};
  }}
  a {{ color: #2563eb; }}
  a[data-wikilink], a.hashtag {{ text-decoration: none; }}
  a.hashtag {{ color: #0f766e; }}
  mark {{ background: rgba(255,200,0,0.45); }}
  .spoiler {{ background: #333; color: #333; border-radius: 3px; }}
  .spoiler:hover {{ color: #fff; }}
  hr {{ border: none; border-top: 1px solid #ccc; margin: 8px 0; }}
  li > input[type=checkbox] {{ margin-right: 6px; }}
  .alert {{
    border-left: 3px solid #2563eb;
    padding-left: 12px;
    margin: 4px 0;
  }}
  .alert-warning, .alert-caution {{ border-left-color: #d97706; }}
  .alert-title {{ font-weight: 600; }}
  .footnote {{ font-size: 13px; color: #666; }}
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
{itemsHtml}
<script>
{FmAst2HtmlBundle.Script}
</script>
<script>
(function(){{
  document.querySelectorAll('[data-ast]').forEach(function(el){{
    try {{ el.innerHTML=FmAst2Html.renderToHtml(el.getAttribute('data-ast')); }} catch(e){{}}
  }});
  // An image is written with a path relative to its page, such as ../assets/pic.png.
  // This document has no address of its own, so the path is resolved against the pages.
  var pages={System.Text.Json.JsonSerializer.Serialize(PageLinks.PagesUrl(_api.BaseUrl))};
  document.querySelectorAll('[data-ast] [src]').forEach(function(el){{
    try {{ el.setAttribute('src',new URL(el.getAttribute('src'),pages).href); }} catch(e){{}}
  }});
  var z={(_windowSettings.ZoomFactor * 100).ToString(System.Globalization.CultureInfo.InvariantCulture)};
  function apply(){{ document.body.style.zoom=z+'%'; window.chrome.webview.postMessage({{zoom:z/100}}); }}
  document.addEventListener('wheel',function(e){{
    if(!e.ctrlKey)return;
    e.preventDefault();
    z=Math.min(300,Math.max(30,z+(e.deltaY<0?10:-10)));
    apply();
  }},{{passive:false}});
  document.addEventListener('keydown',function(e){{
    if(!e.ctrlKey)return;
    if(e.key==='+'||e.key==='='){{ e.preventDefault(); z=Math.min(300,z+10); apply(); }}
    else if(e.key==='-'){{ e.preventDefault(); z=Math.max(30,z-10); apply(); }}
    else if(e.key==='0'){{ e.preventDefault(); z=100; apply(); }}
  }});
  document.addEventListener('click',function(e){{
    var t=e.target.closest('.collapse-toggle');
    if(t){{ window.chrome.webview.postMessage({{toggleCollapse:t.getAttribute('data-item-id')}}); }}
    // The href of a wikilink or a hashtag is a page name, not a URL. Following it would
    // replace the note with an error page. The app opens the page in the web app instead.
    var w=e.target.closest('a[data-wikilink]');
    var h=e.target.closest('a[data-hashtag],a.hashtag');
    if(w||h){{
      e.preventDefault();
      var name=(w||h).getAttribute('href');
      if(name){{ window.chrome.webview.postMessage(w?{{openWikilink:name}}:{{openHashtag:name}}); }}
    }}
  }});
}})();
</script>
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

    /// <summary>
    /// Whether the AST of an item has no text. The server sends an empty AST for an empty
    /// item, and a document without children for an item of only spaces.
    /// </summary>
    private static bool HasNoText(string ast)
    {
        if (string.IsNullOrEmpty(ast)) return true;
        using var document = System.Text.Json.JsonDocument.Parse(ast);
        return !document.RootElement.TryGetProperty("children", out var children)
            || children.GetArrayLength() == 0;
    }

    private static string EscapeHtml(string text)
    {
        return text.Replace("&", "&amp;").Replace("<", "&lt;").Replace(">", "&gt;");
    }

    private static string EscapeHtmlAttr(string text)
    {
        return text.Replace("&", "&amp;").Replace("\"", "&quot;").Replace("<", "&lt;").Replace(">", "&gt;");
    }

    // --- New Window / Close ---

    private void NewWindowButton_Click(object sender, RoutedEventArgs e)
    {
        App.CreateNewWindow();
    }

    private void CloseButton_Click(object sender, RoutedEventArgs e)
    {
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
        // E718 = Pin, E77A = PinFill (pinned)
        PinIcon.Glyph = _windowSettings.AlwaysOnTop ? "\uE841" : "\uE718";
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
        _settingsService.UpdateWindow(_windowSettings);
        App.OnWindowClosed(this);
    }
}
