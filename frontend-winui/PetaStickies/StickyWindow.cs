using System.Collections.ObjectModel;
using System.Runtime.InteropServices;
using Microsoft.UI;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Markup;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Animation;
using PetaBorder;
using PetaTypes;
using Windows.ApplicationModel.DataTransfer;
using Windows.Graphics;
using Windows.System;
using Windows.UI;
using Border = Microsoft.UI.Xaml.Controls.Border;

namespace PetaStickies;

/// <summary>
/// One sticky: a window that shows one page, with a core of its own. Its tools stand in its
/// title bar, and a double click on the title bar rolls it up.
/// </summary>
internal sealed class StickyWindow : Window
{
    /// <summary>How high the title bar is.</summary>
    private const double BarHeight = 32;

    /// <summary>The paper is not smaller than this.</summary>
    private static readonly (double Width, double Height) Smallest = (280, 200);

    /// <summary>The size of a new sticky.</summary>
    private static readonly (double Width, double Height) Usual = (400, 520);

    private static readonly SolidColorBrush Ink = new(Color.FromArgb(255, 0x1F, 0x1F, 0x1F));
    private static readonly SolidColorBrush Pale = new(Color.FromArgb(255, 0x6E, 0x6E, 0x6E));
    private static readonly SolidColorBrush Clear = new(Colors.Transparent);

    private readonly StickyDesk desk;
    private readonly IntPtr handle;
    private readonly Native.SubclassProc hears;

    private readonly Grid root = new() { RequestedTheme = ElementTheme.Light };
    private readonly TextBlock name = new()
    {
        VerticalAlignment = VerticalAlignment.Center,
        Margin = new Thickness(12, 0, 8, 0),
        FontSize = 12,
        Foreground = Ink,
        TextTrimming = TextTrimming.CharacterEllipsis,
    };
    private readonly StackPanel tools = new()
    {
        Orientation = Orientation.Horizontal,
        VerticalAlignment = VerticalAlignment.Center,
    };
    private readonly ColumnDefinition captionButtons = new() { Width = new GridLength(138) };
    private readonly FontIcon pin = new() { FontSize = 14 };
    private readonly TextBlock status = new()
    {
        FontSize = 12,
        Foreground = Pale,
        Margin = new Thickness(12, 0, 12, 6),
    };
    private readonly SearchBox search;
    private readonly TextBlock nothing = new()
    {
        Text = "Search for a page to show here",
        Foreground = Pale,
        HorizontalAlignment = HorizontalAlignment.Center,
        VerticalAlignment = VerticalAlignment.Center,
    };
    private readonly ListView list;
    private readonly ContentControl selection;
    private readonly ObservableCollection<RowModel> rows = [];

    /// <summary>The window is the one that is worked in.</summary>
    private bool hasFocus;
    private bool closed;

    /// <summary>The user is closing the window with the button of its title bar.</summary>
    private bool closedByButton;

    /// <summary>The height of the window before it was rolled up, in pixels.</summary>
    private int fullHeight;

    /// <summary>The window is on its way up or down.</summary>
    private bool rolling;

    /// <summary>
    /// The orders to put the cursor somewhere that were carried out. They are kept for the
    /// whole window, not for each row: the list makes the view of a row again when the row
    /// scrolls back into the window, and that view must not take the cursor for an old order.
    /// </summary>
    private ulong cursorRequest;
    private int cursorOrder;

    /// <summary>The user put the cursor in an item: the core is being told.</summary>
    private bool takesCursor;
    private bool cursorWaits;
    private bool wasSelecting;
    private bool wasSearching;
    private bool seenWaits;
    private (int Top, int First, int Last)? seen;

    /// <summary>The question that the dialog on screen asks.</summary>
    private Question? asked;

    /// <param name="kept">A sticky of the run before, which comes back as it was.</param>
    /// <param name="before">The newest sticky: a new one stands a little down and right of it.</param>
    public StickyWindow(
        StickyDesk desk,
        string server,
        StickySettings? kept,
        string? url,
        StickyWindow? before
    )
    {
        this.desk = desk;
        Id = kept?.Id ?? Guid.NewGuid().ToString();
        Look = new StickyLook(
            kept?.Color ?? StickySettings.Yellow,
            kept?.AlwaysOnTop ?? false,
            kept?.Zoom ?? 1
        );
        handle = WinRT.Interop.WindowNative.GetWindowHandle(this);
        // The core calls from its own threads
        Shell = new Shell(
            server,
            kept?.Url ?? url,
            run => DispatcherQueue.TryEnqueue(() => run()),
            ReadClipboard,
            Copy,
            Browse
        );
        search = new SearchBox(Shell, () => Look.Searching = false)
        {
            Margin = new Thickness(12, 6, 12, 8),
        };
        list = Rows();
        // Takes the keys while items are selected. No editor has the cursor then
        selection = new ContentControl
        {
            Content = list,
            IsTabStop = true,
            UseSystemFocusVisuals = false,
            HorizontalContentAlignment = HorizontalAlignment.Stretch,
            VerticalContentAlignment = VerticalAlignment.Stretch,
        };
        Content = Paper();

        ExtendsContentIntoTitleBar = true;
        Paint();
        Place(kept, before);
        if (AppWindow.Presenter is OverlappedPresenter presenter)
        {
            // A sticky is not made as large as the screen. A double click on its title bar
            // rolls it up
            presenter.IsMaximizable = false;
            presenter.IsAlwaysOnTop = Look.AlwaysOnTop;
        }
        hears = Heard;
        Native.SetWindowSubclass(handle, hears, 1, 0);
        if (kept?.RolledUp == true)
            Roll(animated: false);

        Shell.ViewChanged += Draw;
        Shell.TitleChanged += Name;
        Shell.QuestionChanged += Ask;
        Shell.FocusChanged += () => PutCursor();
        Shell.UrlChanged += () =>
        {
            // The page that was searched for is here. Not by the title: that of a journal
            // changes when it is scrolled
            Look.Searching = false;
            Keep();
        };
        Look.Changed += () =>
        {
            Paint();
            if (AppWindow.Presenter is OverlappedPresenter above)
                above.IsAlwaysOnTop = Look.AlwaysOnTop;
            foreach (var row in rows)
                row.View?.Draw();
            Draw();
            Keep();
        };
        AppWindow.Changed += (_, change) =>
        {
            if (change.DidPositionChange || change.DidSizeChange)
                Keep();
        };
        Activated += (_, change) =>
        {
            hasFocus = change.WindowActivationState != WindowActivationState.Deactivated;
            ShowTools();
            // The keys of the window come by what has the focus: with nothing, none comes
            if (
                hasFocus
                && root.XamlRoot is { } shown
                && FocusManager.GetFocusedElement(shown) is null
            )
                selection.Focus(FocusState.Programmatic);
        };
        Closed += (_, _) =>
        {
            closed = true;
            Native.RemoveWindowSubclass(handle, hears, 1);
            Shell.Dispose();
            desk.Closed(this, closedByButton);
        };
        Name();
        Draw();
        Ask();
    }

    public string Id { get; }
    public Shell Shell { get; }
    public StickyLook Look { get; }

    /// <summary>Only the title bar of the window is shown.</summary>
    public bool RolledUp { get; private set; }

    /// <summary>How many pixels of the screen one pixel of the layout is.</summary>
    private double Scale => Native.GetDpiForWindow(handle) / 96.0;

    // ---- What is on the paper ----

    private Grid Paper()
    {
        root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(BarHeight) });
        root.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        root.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        root.RowDefinitions.Add(new RowDefinition());

        var bar = new Grid();
        bar.ColumnDefinitions.Add(new ColumnDefinition());
        bar.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        bar.ColumnDefinitions.Add(captionButtons);
        // The window is moved by this part of the bar. The tools beside it take their clicks
        var grip = new Border { Background = Clear, Child = name };
        bar.Children.Add(grip);
        SetTitleBar(grip);
        tools.Children.Add(Tool("\uE721", "Search pages (Ctrl+F)", ToggleSearch));
        var pinned = Tool("", "Keep on top of other windows", TogglePin);
        pinned.Content = pin;
        tools.Children.Add(pinned);
        var colors = Tool("\uE790", "Color", () => { });
        var menu = new MenuFlyout();
        foreach (var (title, hex) in StickyLook.Colors)
        {
            var entry = new MenuFlyoutItem { Text = title };
            entry.Click += (_, _) => Look.Color = hex;
            menu.Items.Add(entry);
        }
        colors.Flyout = menu;
        tools.Children.Add(colors);
        tools.Children.Add(Tool("\uE710", "New sticky (Ctrl+N)", () => desk.Add()));
        Grid.SetColumn(tools, 1);
        bar.Children.Add(tools);

        Grid.SetRow(status, 1);
        Grid.SetRow(search, 2);
        Grid.SetRow(selection, 3);
        Grid.SetRow(nothing, 3);
        root.Children.Add(bar);
        root.Children.Add(status);
        root.Children.Add(search);
        root.Children.Add(selection);
        root.Children.Add(nothing);
        root.PreviewKeyDown += Pressed;
        return root;
    }

    private static Button Tool(string glyph, string tip, Action clicked)
    {
        var tool = new Button
        {
            Content = new FontIcon { Glyph = glyph, FontSize = 14 },
            Width = 30,
            Height = 26,
            Padding = new Thickness(0),
            Background = Clear,
            BorderThickness = new Thickness(0),
            IsTabStop = false,
        };
        ToolTipService.SetToolTip(tool, tip);
        tool.Click += (_, _) => clicked();
        return tool;
    }

    private ListView Rows()
    {
        var container = new Style(typeof(ListViewItem));
        container.Setters.Add(new Setter(Control.PaddingProperty, new Thickness(0)));
        container.Setters.Add(new Setter(FrameworkElement.MinHeightProperty, 0.0));
        container.Setters.Add(new Setter(Control.IsTabStopProperty, false));
        container.Setters.Add(
            new Setter(Control.HorizontalContentAlignmentProperty, HorizontalAlignment.Stretch)
        );
        container.Setters.Add(
            new Setter(Control.VerticalContentAlignmentProperty, VerticalAlignment.Top)
        );
        var made = new ListView
        {
            ItemsSource = rows,
            SelectionMode = ListViewSelectionMode.None,
            IsItemClickEnabled = false,
            IsTabStop = false,
            ItemContainerStyle = container,
            // The view of a row is made in code: the template only holds its place
            ItemTemplate = (DataTemplate)
                XamlReader.Load(
                    "<DataTemplate xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation'>"
                        + "<ContentPresenter HorizontalContentAlignment='Stretch'/></DataTemplate>"
                ),
            // An item that comes or goes does not slide: it does so at every Enter
            ItemContainerTransitions = new TransitionCollection(),
            Padding = new Thickness(10, 0, 10, 10),
        };
        // The paper shows through the list, also under the mouse
        foreach (
            var brush in new[]
            {
                "ListViewItemBackgroundPointerOver",
                "ListViewItemBackgroundPressed",
            }
        )
            made.Resources[brush] = Clear;
        made.ContainerContentChanging += (_, change) =>
        {
            if (change.ItemContainer.ContentTemplateRoot is not ContentPresenter place)
                return;
            change.Handled = true;
            if (change.InRecycleQueue)
            {
                (place.Content as RowView)?.Leave();
                return;
            }
            var view = place.Content as RowView ?? new RowView(this);
            place.Content = view;
            view.Show((RowModel)change.Item);
        };
        made.Loaded += (_, _) =>
        {
            if (Scroller(made) is { } scroller)
                scroller.ViewChanged += (_, _) => Seen();
        };
        return made;
    }

    private static ScrollViewer? Scroller(DependencyObject under)
    {
        for (var place = 0; place < VisualTreeHelper.GetChildrenCount(under); place++)
        {
            var child = VisualTreeHelper.GetChild(under, place);
            if (child is ScrollViewer scroller)
                return scroller;
            if (Scroller(child) is { } deeper)
                return deeper;
        }
        return null;
    }

    /// <summary>The color of the paper. It is light in every color, and so the text on it is
    /// always dark.</summary>
    private void Paint()
    {
        var (red, green, blue) = StickyLook.Rgb(Look.Color);
        root.Background = new SolidColorBrush(Color.FromArgb(255, red, green, blue));
        pin.Glyph = Look.AlwaysOnTop ? "\uE841" : "\uE718";
        var bar = AppWindow.TitleBar;
        bar.ButtonBackgroundColor = Colors.Transparent;
        bar.ButtonInactiveBackgroundColor = Colors.Transparent;
        bar.ButtonForegroundColor = Colors.Black;
        bar.ButtonInactiveForegroundColor = Colors.Gray;
        bar.ButtonHoverBackgroundColor = Color.FromArgb(0x24, 0, 0, 0);
        bar.ButtonHoverForegroundColor = Colors.Black;
        bar.ButtonPressedBackgroundColor = Color.FromArgb(0x38, 0, 0, 0);
        bar.ButtonPressedForegroundColor = Colors.Black;
    }

    private void Name()
    {
        // A window without a name is hard to find among the others
        var title = Shell.Title.Length == 0 ? "PetaStickies" : Shell.Title;
        Title = title;
        name.Text = title;
    }

    private void Draw()
    {
        if (closed)
            return;
        var view = Shell.View;
        var hasPage = Shell.Url is not null;
        // Items that do not come need a reason
        status.Text = !hasPage
            ? ""
            : view.Connection switch
            {
                ConnectionState.Connected => "",
                ConnectionState.Connecting => "Connecting to the journal server…",
                _ => "No connection to the journal server",
            };
        status.Visibility = status.Text.Length == 0 ? Visibility.Collapsed : Visibility.Visible;
        // Without a page there is nothing to do but to search for one
        var searching = Look.Searching || !hasPage;
        search.Visibility = searching ? Visibility.Visible : Visibility.Collapsed;
        search.Draw(view.Search);
        if (searching && !wasSearching)
            search.TakeCursor();
        // The focus must not go with the box: the keys of the window come by it
        if (!searching && wasSearching)
            selection.Focus(FocusState.Programmatic);
        wasSearching = searching;
        nothing.Visibility = hasPage ? Visibility.Collapsed : Visibility.Visible;
        selection.Visibility = hasPage ? Visibility.Visible : Visibility.Collapsed;
        Sync(view.Outline.Rows);
        // A key must not change the text of an item while items are selected
        if (view.Outline.SelectionMode && !wasSelecting)
            selection.Focus(FocusState.Programmatic);
        wasSelecting = view.Outline.SelectionMode;
        PutCursor();
        SeenLater();
    }

    /// <summary>
    /// Makes the rows of the list those of the core. A row that stays keeps its view: the
    /// editor with the cursor is not made again at every key.
    /// </summary>
    private void Sync(ObservableCollection<Row> wanted)
    {
        var ids = new HashSet<string>(wanted.Select(row => row.Item.Id));
        if (rows.Count > 0 && !rows.Any(row => ids.Contains(row.Id)))
            rows.Clear();
        for (var place = rows.Count - 1; place >= 0; place--)
        {
            if (!ids.Contains(rows[place].Id))
                rows.RemoveAt(place);
        }
        for (var place = 0; place < wanted.Count; place++)
        {
            var id = wanted[place].Item.Id;
            if (place < rows.Count && rows[place].Id == id)
            {
                rows[place].Set(wanted[place]);
                continue;
            }
            var from = -1;
            for (var later = place + 1; later < rows.Count && from < 0; later++)
            {
                if (rows[later].Id == id)
                    from = later;
            }
            if (from < 0)
            {
                rows.Insert(place, new RowModel(wanted[place]));
                continue;
            }
            rows.Move(from, place);
            rows[place].Set(wanted[place]);
        }
        while (rows.Count > wanted.Count)
            rows.RemoveAt(rows.Count - 1);
    }

    // ---- The cursor ----

    /// <summary>The editor of the item got the cursor, by a click or by an order of the core.</summary>
    public void GotCursor(string itemId)
    {
        // Only the user puts the cursor in an editor while items are selected, and that ends
        // the selection. The core still names this item as the one with the cursor then
        var selecting = Shell.View.Outline.SelectionMode;
        takesCursor = true;
        try
        {
            if (selecting)
                Shell.LeaveSelection();
            // The core may name this item as the one with the cursor and still not count it
            // as edited: the cursor left it for something that is no item, and came back
            var outline = Shell.View.Outline;
            var edited = outline.Rows.Any(row => row.Item.Id == itemId && row.IsEditing);
            if (!selecting && edited && outline.FocusedItemId == itemId)
                return;
            Shell.StartEditing(itemId);
            // The core answers with an order to put the cursor at the start of this item.
            // The cursor is already here, and where the user clicked
            if (Shell.View.Outline.FocusRequest is { } request && request.ItemId == itemId)
                cursorRequest = request.Seq;
        }
        finally
        {
            takesCursor = false;
        }
    }

    /// <summary>Carries out an order of the core that is new.</summary>
    private void PutCursor(int tries = 10)
    {
        if (closed || takesCursor)
            return;
        var outline = Shell.View.Outline;
        var request =
            outline.FocusRequest is { } wanted && wanted.Seq != cursorRequest ? wanted : null;
        var order = Shell.Focus is { } given && given.Number != cursorOrder ? given : null;
        if (request is null && order is null)
            return;
        // The order of the shell names no item when the cursor goes to the first one
        var itemId = request?.ItemId ?? order?.ItemId ?? rows.FirstOrDefault()?.Id;
        var row = rows.FirstOrDefault(row => row.Id == itemId);
        if (row is null)
        {
            // The item may come with the next view
            return;
        }
        // A cursor in an editor would end the selection
        if (!outline.SelectionMode)
        {
            if (row.View is null)
            {
                list.ScrollIntoView(row);
                list.UpdateLayout();
            }
            int? cursor = request?.Cursor is { } characters
                ? Cursor.Utf16(characters, row.Row.Item.Body)
                : null;
            if (row.View?.TakeCursor(cursor) != true)
            {
                // The view of a new item is not laid out yet when the order comes with it
                if (tries > 0 && !cursorWaits)
                {
                    cursorWaits = true;
                    DispatcherQueue.TryEnqueue(() =>
                    {
                        cursorWaits = false;
                        PutCursor(tries - 1);
                    });
                }
                return;
            }
        }
        if (request is not null)
            cursorRequest = request.Seq;
        if (order is not null)
            cursorOrder = order.Number;
    }

    // ---- The keys of the window ----

    private void Pressed(object sender, KeyRoutedEventArgs e)
    {
        var held = Input.Held();
        if (held == Held.Control || (held == (Held.Control | Held.Shift) && e.Key == Input.Plus))
        {
            Action? does = e.Key switch
            {
                VirtualKey.N => () => desk.Add(),
                VirtualKey.F => ToggleSearch,
                // Ends the app and keeps the stickies. A sticky that is closed is gone
                VirtualKey.Q => desk.End,
                Input.Plus or VirtualKey.Add => Look.ZoomIn,
                Input.Minus or VirtualKey.Subtract => Look.ZoomOut,
                VirtualKey.Number0 or VirtualKey.NumberPad0 => Look.ResetZoom,
                _ => null,
            };
            if (does is not null)
            {
                e.Handled = true;
                does();
                return;
            }
        }
        if (
            !Shell.View.Outline.SelectionMode
            || asked is not null
            || FocusManager.GetFocusedElement(root.XamlRoot) is TextBox
            || Input.KeyOf(e.Key) is not { } key
        )
            return;
        // A key that means nothing to a selection does nothing: there is no text to type into
        e.Handled = true;
        if (Keyboard.InSelection(key, held) is { } pressed)
            Shell.Selection(pressed);
    }

    private void ToggleSearch() => Look.Searching = !Look.Searching;

    private void TogglePin() => Look.AlwaysOnTop = !Look.AlwaysOnTop;

    // ---- The day at the top of a journal ----

    private void SeenLater()
    {
        if (seenWaits)
            return;
        seenWaits = true;
        // After the layout: the list has not drawn the new rows yet
        DispatcherQueue.TryEnqueue(() =>
        {
            seenWaits = false;
            Seen();
        });
    }

    /// <summary>
    /// A journal tells the day at the top of the window by the rows that can be seen, and
    /// loads the days before by the rows that are drawn.
    /// </summary>
    private void Seen()
    {
        // Only a journal does something with it, and every time costs a view model
        if (
            closed
            || Shell.View.Screen is not ScreenView.Journal
            || list.ItemsPanelRoot is not ItemsStackPanel panel
            || panel.FirstVisibleIndex < 0
            || panel.LastCacheIndex >= rows.Count
        )
            return;
        var now = (
            (int)rows[panel.FirstVisibleIndex].Row.Index,
            (int)rows[Math.Max(panel.FirstCacheIndex, 0)].Row.Index,
            (int)rows[Math.Max(panel.LastCacheIndex, panel.FirstVisibleIndex)].Row.Index
        );
        if (seen == now)
            return;
        seen = now;
        Shell.Scrolled(now.Item1, now.Item2, now.Item3);
    }

    // ---- What the core asks ----

    private async void Ask()
    {
        if (closed || asked is not null || Shell.Question is not { } question)
            return;
        if (root.XamlRoot is null)
        {
            // The window is not on screen yet
            root.Loaded += (_, _) => Ask();
            return;
        }
        asked = question;
        var typed = new TextBox();
        var dialog = new ContentDialog
        {
            XamlRoot = root.XamlRoot,
            RequestedTheme = ElementTheme.Light,
            Title = question.Message,
            PrimaryButtonText = "OK",
            CloseButtonText = "Cancel",
            // No key says yes to something hard to undo: Enter and Escape both cancel
            DefaultButton =
                question.Kind == QuestionKind.ConfirmDestructive
                    ? ContentDialogButton.Close
                    : ContentDialogButton.Primary,
            Content = question.Kind == QuestionKind.Prompt ? typed : null,
        };
        var yes = await dialog.ShowAsync() == ContentDialogResult.Primary;
        asked = null;
        if (closed)
            return;
        if (question.Kind == QuestionKind.Prompt)
            Shell.Answer(yes ? typed.Text : null);
        else
            Shell.Answer(yes);
        // The next question waited behind this one
        Ask();
    }

    private async void ReadClipboard(Action<string?> hand)
    {
        string? text = null;
        try
        {
            var content = Clipboard.GetContent();
            if (content.Contains(StandardDataFormats.Text))
                text = (await content.GetTextAsync()).Replace("\r\n", "\n");
        }
        catch (Exception e) when (e is COMException or UnauthorizedAccessException)
        {
            // Another app holds the clipboard: there is no text to read
        }
        hand(text);
    }

    private static void Copy(string text, string? html)
    {
        var content = new DataPackage();
        // The other apps of Windows expect its line breaks
        content.SetText(text.Replace("\r\n", "\n").Replace("\n", "\r\n"));
        // An app that takes rich text reads this, and keeps lists, bold and links
        if (html is not null)
            content.SetHtmlFormat(HtmlFormatHelper.CreateHtmlFormat(html));
        try
        {
            Clipboard.SetContent(content);
        }
        catch (Exception e) when (e is COMException or UnauthorizedAccessException)
        {
            // Another app holds the clipboard
        }
    }

    private static void Browse(Uri address)
    {
        // A note must not start any app by a link: only what a browser or a mail app opens
        if (address.Scheme is "http" or "https" or "mailto")
            _ = Launcher.LaunchUriAsync(address);
    }

    // ---- The window ----

    private void Place(StickySettings? kept, StickyWindow? before)
    {
        if (kept is not null)
        {
            AppWindow.Move(new PointInt32(kept.X, kept.Y));
            // After the move: the scale is that of the screen the window is on
            AppWindow.Resize(Pixels(kept.Width, kept.Height));
            return;
        }
        // A little down and to the right of the newest sticky
        if (before is not null)
        {
            var step = (int)Math.Round(BarHeight * before.Scale);
            var corner = before.AppWindow.Position;
            AppWindow.Move(new PointInt32(corner.X + step, corner.Y + step));
        }
        AppWindow.Resize(Pixels(Usual.Width, Usual.Height));
    }

    private SizeInt32 Pixels(double width, double height) =>
        new((int)Math.Round(width * Scale), (int)Math.Round(height * Scale));

    /// <summary>How high the window is when only its title bar is shown, in pixels.</summary>
    private int BarPixels =>
        AppWindow.Size.Height - AppWindow.ClientSize.Height + (int)Math.Round(BarHeight * Scale);

    /// <summary>Keeps the sticky as it is now.</summary>
    public void Keep()
    {
        if (closed)
            return;
        var size = AppWindow.Size;
        // What is kept is the whole sticky, also while only its title bar is shown
        var height = RolledUp || rolling ? fullHeight : size.Height;
        desk.Keep(
            new StickySettings
            {
                Id = Id,
                Url = Shell.Url,
                X = AppWindow.Position.X,
                Y = AppWindow.Position.Y,
                Width = size.Width / Scale,
                Height = height / Scale,
                Color = Look.Color,
                AlwaysOnTop = Look.AlwaysOnTop,
                Zoom = Look.Zoom,
                RolledUp = RolledUp,
            }
        );
    }

    /// <summary>
    /// The tools are for the sticky that is worked on. On the others they are only in the way
    /// of the paper, and a sticky that is rolled up has no paper to work on.
    /// </summary>
    private void ShowTools()
    {
        tools.Visibility = hasFocus && !RolledUp ? Visibility.Visible : Visibility.Collapsed;
        var inset = AppWindow.TitleBar.RightInset;
        if (inset > 0)
            captionButtons.Width = new GridLength(inset / Scale);
    }

    /// <summary>
    /// Rolls the sticky up to its title bar, or down again to the height it had. The top edge
    /// stays where it is. The window moves there in steps.
    /// </summary>
    public void Roll(bool animated = true)
    {
        if (rolling)
            return;
        var from = AppWindow.Size.Height;
        var down = RolledUp;
        if (!down)
            fullHeight = from;
        var to = down ? fullHeight : BarPixels;
        // On the way the sticky counts as rolled up: what is kept then is its whole height,
        // not the height of a step
        RolledUp = true;
        rolling = true;
        ShowTools();
        var step = 0;
        const int steps = 8;
        if (!animated)
        {
            Finish();
            return;
        }
        var timer = DispatcherQueue.CreateTimer();
        timer.Interval = TimeSpan.FromMilliseconds(15);
        timer.Tick += (_, _) =>
        {
            step++;
            if (step >= steps || closed)
            {
                timer.Stop();
                if (!closed)
                    Finish();
                return;
            }
            Resize(from + (to - from) * step / steps);
        };
        timer.Start();

        void Finish()
        {
            Resize(to);
            rolling = false;
            RolledUp = !down;
            ShowTools();
            Keep();
        }

        void Resize(int height) => AppWindow.Resize(new SizeInt32(AppWindow.Size.Width, height));
    }

    /// <summary>Hears what Windows tells the window, before WinUI does.</summary>
    private IntPtr Heard(
        IntPtr window,
        uint message,
        IntPtr wParam,
        IntPtr lParam,
        UIntPtr id,
        UIntPtr data
    )
    {
        switch (message)
        {
            case Native.WmNcLeftButtonDoubleClick when wParam == Native.HitCaption:
                Roll();
                return IntPtr.Zero;
            // The button of the title bar comes with the place of the mouse. Alt+F4, the
            // taskbar and the end of the session come without one, and keep the sticky
            case Native.WmSysCommand
                when (wParam.ToInt64() & 0xFFF0) == Native.SysClose && lParam != IntPtr.Zero:
                closedByButton = true;
                return Native.DefSubclassProc(window, message, wParam, lParam);
            case Native.WmGetMinMaxInfo:
            {
                var result = Native.DefSubclassProc(window, message, wParam, lParam);
                var limits = Marshal.PtrToStructure<Native.MinMaxInfo>(lParam);
                limits.MinTrackSize.X = (int)Math.Round(Smallest.Width * Scale);
                // A sticky that is rolled up keeps its height: only its width can be changed
                if (rolling)
                    limits.MinTrackSize.Y = 0;
                else if (RolledUp)
                    limits.MinTrackSize.Y = limits.MaxTrackSize.Y = BarPixels;
                else
                    limits.MinTrackSize.Y = (int)Math.Round(Smallest.Height * Scale);
                Marshal.StructureToPtr(limits, lParam, false);
                return result;
            }
            default:
                return Native.DefSubclassProc(window, message, wParam, lParam);
        }
    }
}
