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

namespace PetaScreens;

/// <summary>
/// One sticky: a window that shows one page, with a core of its own. Its tools stand in its
/// title bar, and a double click on the title bar rolls it up.
/// </summary>
internal sealed class StickyWindow : Window
{
    /// <summary>How high the title bar is.</summary>
    private const double BarHeight = 32;

    /// <summary>The paper is not smaller than this, as on macOS (<c>client-stickies</c>).</summary>
    private static readonly PetaCore.PetaSize Smallest = PetaCore.PetaNative.SmallestSticky();

    /// <summary>The size of a new sticky, as on macOS.</summary>
    private static readonly PetaCore.PetaSize Usual = PetaCore.PetaNative.UsualSticky();

    private static readonly SolidColorBrush Ink = new(Color.FromArgb(255, 0x1F, 0x1F, 0x1F));
    private static readonly SolidColorBrush Pale = new(Color.FromArgb(255, 0x6E, 0x6E, 0x6E));
    private static readonly SolidColorBrush Clear = new(Colors.Transparent);
    private static readonly SolidColorBrush White = new(Colors.White);

    /// <summary>The red of the button that closes a window of Windows, under the mouse.</summary>
    private static readonly SolidColorBrush CloseRed = new(Color.FromArgb(255, 0xC4, 0x2B, 0x1C));

    /// <summary>The signs of the buttons of a window are smaller than those of the tools.</summary>
    private const double WindowGlyphSize = 10;

    private readonly StickyDesk desk;
    private readonly IntPtr handle;
    private readonly Native.SubclassProc hears;

    private readonly Grid root = new() { RequestedTheme = ElementTheme.Light };
    private readonly TextBlock name = new()
    {
        VerticalAlignment = VerticalAlignment.Center,
        Margin = new Thickness(4, 0, 8, 0),
        FontFamily = RowView.TextFont,
        FontSize = 12,
        Foreground = Ink,
        TextTrimming = TextTrimming.CharacterEllipsis,
    };
    private readonly StackPanel tools = new()
    {
        Orientation = Orientation.Horizontal,
        VerticalAlignment = VerticalAlignment.Center,
    };

    /// <summary>
    /// The buttons that make the window small or large and close it. They are drawn here:
    /// those of Windows are wide, and leave little of a narrow title bar.
    /// </summary>
    private readonly StackPanel windowButtons = new()
    {
        Orientation = Orientation.Horizontal,
        VerticalAlignment = VerticalAlignment.Center,
        Margin = new Thickness(0, 0, 4, 0),
    };
    private readonly FontIcon large = new() { FontSize = WindowGlyphSize };

    /// <summary>The button for a new sticky, at the left of the title bar.</summary>
    private readonly Button add;

    /// <summary>What is behind a sticky without paper. It is made once it is needed.</summary>
    private ClearBackdrop? clear;
    private readonly TextBlock status = new()
    {
        FontFamily = RowView.TextFont,
        FontSize = 12,
        Foreground = Pale,
        Margin = new Thickness(12, 0, 12, 6),
    };
    private readonly SearchBox search;

    /// <summary>The arrows of the history of the pages opened in this sticky (ADR 035).</summary>
    private readonly Button back;

    private readonly Button forward;
    private readonly TextBlock nothing = new()
    {
        Text = "Search for a page to show here",
        FontFamily = RowView.TextFont,
        Foreground = Pale,
        HorizontalAlignment = HorizontalAlignment.Center,
        VerticalAlignment = VerticalAlignment.Center,
    };
    private readonly ListView list;
    private readonly ContentControl selection;
    private readonly ObservableCollection<RowModel> rows = [];

    /// <summary>The scroller of the list, once the list is loaded.</summary>
    private ScrollViewer? scroller;

    /// <summary>The mouse button that went down on an item and is still held.</summary>
    private Pointer? dragging;

    /// <summary>Where the mouse of the drag is, in the coordinates of the list.</summary>
    private Windows.Foundation.Point dragPoint;

    /// <summary>Scrolls the list while a drag selection holds the mouse at an edge.</summary>
    private readonly Microsoft.UI.Dispatching.DispatcherQueueTimer edgeScroll;

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
    /// The place and the size of the sticky, in pixels: those of the window while it is not
    /// as large as the screen.
    /// </summary>
    private RectInt32 place;

    /// <summary>
    /// The orders to put the cursor somewhere that were carried out. They are kept for the
    /// whole window, not for each row: the list makes the view of a row again when the row
    /// scrolls back into the window, and that view must not take the cursor for an old order.
    /// </summary>
    private ulong cursorRequest;
    private ulong cursorOrder;

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
        PetaCore.PetaSticky? kept,
        string? url,
        StickyWindow? before
    )
    {
        this.desk = desk;
        Id = kept?.Id ?? Guid.NewGuid().ToString();
        Look = new StickyLook(kept?.Color, kept?.AlwaysOnTop ?? false, kept?.Zoom ?? 1);
        handle = WinRT.Interop.WindowNative.GetWindowHandle(this);
        // The core calls from its own threads
        Shell = new Shell(
            server,
            kept?.Url ?? url,
            run => DispatcherQueue.TryEnqueue(() => run()),
            ReadClipboard,
            Copy,
            Launch.Browse
        );
        // The rows of the first screen are there; later ones come as changes to them
        foreach (var row in Shell.View.Outline.Rows)
            rows.Add(new RowModel(row));
        Shell.RowsChanged += Apply;
        edgeScroll = DispatcherQueue.CreateTimer();
        edgeScroll.Interval = EdgeScroll.Interval;
        edgeScroll.Tick += (_, _) => ScrollAtEdge();
        search = new SearchBox(Shell, () => Look.Searching = false)
        {
            Margin = new Thickness(12, 6, 12, 8),
        };
        // The keys of the history in the browsers and the File Explorer of Windows. The
        // chevrons are those of the Mac sticky
        back = Tool("\uE76B", "Back (Alt+Left)", Shell.GoBack);
        forward = Tool("\uE76C", "Forward (Alt+Right)", Shell.GoForward);
        list = Rows();
        // A new sticky is one click away from every sticky: one without the focus, and one
        // that is rolled up. So this button is not among the tools, which are hidden then.
        // It answers when it is pressed, and does not wait for the mouse button to come up
        add = Tool("", "New sticky (Ctrl+N)", () => desk.AddNextTo(this));
        add.ClickMode = ClickMode.Press;
        add.Margin = new Thickness(4, 0, 0, 0);
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
        // Alt+Tab shows the icon of the window, not that of the .exe
        AppWindow.SetIcon(Path.Combine(AppContext.BaseDirectory, "PetaStickies.ico"));
        Paint();
        Place(kept, before);
        if (AppWindow.Presenter is OverlappedPresenter presenter)
        {
            // The title bar of Windows goes, and its buttons with it: the sticky has its own
            presenter.SetBorderAndTitleBar(true, false);
            presenter.IsAlwaysOnTop = Look.AlwaysOnTop;
        }
        hears = Heard;
        Native.SetWindowSubclass(handle, hears, 1, 0);
        if (kept?.RolledUp == true)
            Roll(animated: false);
        Measure();
        ShowTools();

        Shell.ViewChanged += Draw;
        Shell.TitleChanged += Name;
        Shell.NewStickyAsked += url => desk.AddNextTo(this, url);
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
            ShowTools();
            if (AppWindow.Presenter is OverlappedPresenter above)
                above.IsAlwaysOnTop = Look.AlwaysOnTop;
            foreach (var row in rows)
                row.View?.Draw();
            Draw();
            Keep();
        };
        AppWindow.Changed += (_, change) =>
        {
            if (!change.DidPositionChange && !change.DidSizeChange)
                return;
            // Windows + Up and a drag to the top of the screen make the window large without
            // asking first. A sticky that is rolled up would become a bar as wide as the
            // screen, so it goes back to what it was
            if (Maximized && (RolledUp || rolling))
                DispatcherQueue.TryEnqueue(Restore);
            ShowTools();
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

    /// <summary>The sticky shows only what is written on it: no bullets either.</summary>
    public bool Bare { get; private set; }

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
        bar.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        bar.ColumnDefinitions.Add(new ColumnDefinition());
        bar.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        bar.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        bar.Children.Add(add);
        // The window is moved by this part of the bar. The tools beside it take their clicks
        var grip = new Border { Background = Clear, Child = name };
        Grid.SetColumn(grip, 1);
        bar.Children.Add(grip);
        SetTitleBar(grip);
        tools.Children.Add(back);
        tools.Children.Add(forward);
        tools.Children.Add(Tool("\uE721", "Search pages (Ctrl+F)", ToggleSearch));
        Grid.SetColumn(tools, 2);
        bar.Children.Add(tools);

        windowButtons.Children.Add(Tool("\uE921", "Minimize", Minimize, WindowGlyphSize));
        var enlarge = Tool("", "Maximize", ToggleMaximized);
        enlarge.Content = large;
        windowButtons.Children.Add(enlarge);
        var close = Tool("\uE8BB", "Close", CloseByButton, WindowGlyphSize);
        close.Resources["ButtonBackgroundPointerOver"] = CloseRed;
        close.Resources["ButtonBackgroundPressed"] = CloseRed;
        close.Resources["ButtonForegroundPointerOver"] = White;
        close.Resources["ButtonForegroundPressed"] = White;
        windowButtons.Children.Add(close);
        Grid.SetColumn(windowButtons, 3);
        bar.Children.Add(windowButtons);

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
        // Also when a text box took them: a drag that starts in an editor selects its text
        root.AddHandler(UIElement.PointerMovedEvent, new PointerEventHandler(Dragged), true);
        root.AddHandler(UIElement.PointerReleasedEvent, new PointerEventHandler(Dropped), true);
        // Also when a button took it: a click on a button of the bar ends a selection too
        root.AddHandler(
            UIElement.PointerPressedEvent,
            new PointerEventHandler(PressedOutsideItems),
            true
        );
        return root;
    }

    /// <summary>
    /// A press on anything but an item ends a selection, as the web frontend does. A press on
    /// an item is left to the item: it starts an edit or a drag, and the core ends the
    /// selection then.
    /// </summary>
    private void PressedOutsideItems(object sender, PointerRoutedEventArgs e)
    {
        if (!Shell.View.Outline.SelectionMode || IsOnItem(e.OriginalSource as DependencyObject))
            return;
        Shell.ClickedOutsideItems();
    }

    private static bool IsOnItem(DependencyObject? element)
    {
        for (var at = element; at is not null; at = VisualTreeHelper.GetParent(at))
        {
            if (at is RowView)
                return true;
        }
        return false;
    }

    private static Button Tool(string glyph, string tip, Action clicked, double size = 14)
    {
        var tool = new Button
        {
            Content = new FontIcon { Glyph = glyph, FontSize = size },
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
            scroller = Scroller(made);
            if (scroller is not null)
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
    /// always dark. A sticky without paper shows the windows behind it.</summary>
    private void Paint()
    {
        if (Look.Transparent)
        {
            SystemBackdrop = clear ??= new ClearBackdrop(handle);
            // Not null: the clicks on the empty part of the paper still come to the window
            root.Background = Clear;
        }
        else
        {
            SystemBackdrop = null;
            var paper = PetaCore.PetaNative.StickyRgb(Look.Color);
            root.Background = new SolidColorBrush(
                Color.FromArgb(255, paper.Red, paper.Green, paper.Blue)
            );
        }
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
        back.IsEnabled = view.History.CanGoBack;
        forward.IsEnabled = view.History.CanGoForward;
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
        // A key must not change the text of an item while items are selected
        if (view.Outline.SelectionMode && !wasSelecting)
            selection.Focus(FocusState.Programmatic);
        wasSelecting = view.Outline.SelectionMode;
        PutCursor();
        SeenLater();
    }

    /// <summary>
    /// Applies the changes of the rows that the core sent. A row that stays keeps its view:
    /// the editor with the cursor is not made again at every key, and only a changed row is
    /// drawn again (ADR 076).
    /// </summary>
    private void Apply(IReadOnlyList<ListChange> changes)
    {
        if (closed)
            return;
        ListChanges.Apply(rows, changes, row => new RowModel(row), (at, row) => rows[at].Set(row));
    }

    // ---- The cursor ----

    /// <summary>The editor of the item got the cursor, by a click or by an order of the core.</summary>
    public void GotCursor(string itemId)
    {
        // The core is gone. WinUI still moves the cursor to an editor while it takes the
        // window down
        if (closed)
            return;
        // Only the user puts the cursor in an editor while items are selected. The core ends
        // the selection then (ADR 057), and still names this item as the one with the cursor
        var selecting = Shell.View.Outline.SelectionMode;
        takesCursor = true;
        try
        {
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

    /// <summary>
    /// The user clicked the shown text of the item after <paramref name="letter"/> of its
    /// letters. The core answers with an order to put the cursor at that place in the Markdown.
    /// </summary>
    public void Touched(string itemId, int letter) => Shell.Touched(itemId, letter);

    // ---- The drag that selects items ----

    /// <summary>
    /// The mouse went down on <paramref name="body"/>, the text of the item. A drag that leaves
    /// it up or down selects items, as in the web frontend.
    /// </summary>
    public void DragFrom(FrameworkElement body, string itemId, PointerRoutedEventArgs e)
    {
        if (!e.GetCurrentPoint(list).Properties.IsLeftButtonPressed)
            return;
        var top = body.TransformToVisual(list).TransformPoint(default).Y;
        Shell.Dragged(new DragMove.Pressed(itemId, top, top + body.ActualHeight));
        dragging = e.Pointer;
    }

    private void Dragged(object sender, PointerRoutedEventArgs e)
    {
        if (dragging is null)
            return;
        var point = e.GetCurrentPoint(list);
        // The button came up where the window did not hear it
        if (!point.Properties.IsLeftButtonPressed)
        {
            EndDrag();
            return;
        }
        dragPoint = point.Position;
        Shell.Dragged(new DragMove.Moved(dragPoint.Y, ItemAt(dragPoint)));
        if (!Shell.View.Outline.SelectionMode)
            return;
        // The text box that took the mouse is gone, and the mouse may leave the window
        root.CapturePointer(e.Pointer);
        if (!edgeScroll.IsRunning && EdgeScroll.Step(dragPoint.Y, 0, list.ActualHeight) != 0)
            edgeScroll.Start();
    }

    private void Dropped(object sender, PointerRoutedEventArgs e)
    {
        if (dragging is not null)
            EndDrag();
    }

    private void EndDrag()
    {
        Shell.Dragged(new DragMove.Released());
        dragging = null;
        edgeScroll.Stop();
        root.ReleasePointerCaptures();
    }

    /// <summary>
    /// One step of the scroll while the mouse stays at an edge. The row that scrolls under the
    /// mouse joins the selection: the mouse does not move, so no move tells it.
    /// </summary>
    private void ScrollAtEdge()
    {
        var step = EdgeScroll.Step(dragPoint.Y, 0, list.ActualHeight);
        if (dragging is null || scroller is null || step == 0)
        {
            edgeScroll.Stop();
            return;
        }
        scroller.ChangeView(null, scroller.VerticalOffset + step, null, true);
        Shell.Dragged(new DragMove.Moved(dragPoint.Y, ItemAt(dragPoint)));
    }

    /// <summary>
    /// The item at <paramref name="place"/> of the list. A place beyond the list counts as
    /// the nearest place in it: the mouse of a drag may leave the window.
    /// </summary>
    private string? ItemAt(Windows.Foundation.Point place)
    {
        var inside = new Windows.Foundation.Point(
            Math.Clamp(place.X, list.Padding.Left + 1, list.ActualWidth - list.Padding.Right - 1),
            EdgeScroll.PlaceInList(place.Y, 0, list.ActualHeight)
        );
        var there = list.TransformToVisual(null).TransformPoint(inside);
        foreach (var hit in VisualTreeHelper.FindElementsInHostCoordinates(there, list))
        {
            if (hit is ListViewItem container && list.ItemFromContainer(container) is RowModel row)
                return row.Id;
        }
        return null;
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
                ? (int)PetaCore.PetaNative.Utf16OfChars(row.Row.Item.Body, characters)
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
                VirtualKey.N => () => desk.AddNextTo(this),
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
        if (held == Held.Alt)
        {
            Action? turns = e.Key switch
            {
                VirtualKey.Left => Shell.GoBack,
                VirtualKey.Right => Shell.GoForward,
                _ => null,
            };
            if (turns is not null)
            {
                e.Handled = true;
                turns();
                return;
            }
        }
        if (
            !Shell.View.Outline.SelectionMode
            || asked is not null
            || FocusManager.GetFocusedElement(root.XamlRoot) is TextBox
            || Input.ChordOf(e.Key, held) is not { } chord
        )
            return;
        // A key that means nothing to a selection does nothing: there is no text to type into
        e.Handled = true;
        _ = Shell.Pressed(new KeyPress.InSelection(chord));
    }

    private void ToggleSearch() => Look.Searching = !Look.Searching;

    private void TogglePin() => Look.AlwaysOnTop = !Look.AlwaysOnTop;

    /// <summary>Shows the menu of the sticky where the mouse is on the title bar.</summary>
    /// <param name="screen">Where the mouse is, in pixels of the screen.</param>
    private void ShowMenu(Native.Point screen)
    {
        Native.ScreenToClient(handle, ref screen);
        ShowMenu(new Windows.Foundation.Point(screen.X / Scale, screen.Y / Scale));
    }

    /// <summary>
    /// Shows the menu of the sticky: the pin, and the colors. The pin and the color of the
    /// sticky are checked. They are not tools of the bar: they would leave little of it to
    /// the name of a narrow sticky. On an item, what can be done to the item comes first.
    /// </summary>
    /// <param name="place">Where the mouse is in the window.</param>
    /// <param name="itemId">The item that was clicked, or null on the title bar.</param>
    public void ShowMenu(Windows.Foundation.Point place, string? itemId = null)
    {
        var menu = new MenuFlyout();
        if (itemId is not null)
        {
            var transclude = new MenuFlyoutItem { Text = "Paste as Transclusion" };
            transclude.Click += (_, _) => Shell.PasteAsTransclusion(itemId);
            menu.Items.Add(transclude);
            menu.Items.Add(new MenuFlyoutSeparator());
        }
        var pin = new ToggleMenuFlyoutItem { Text = "Keep on top", IsChecked = Look.AlwaysOnTop };
        pin.Click += (_, _) => TogglePin();
        menu.Items.Add(pin);
        menu.Items.Add(new MenuFlyoutSeparator());
        foreach (var (title, hex) in StickyLook.Colors)
        {
            var color = new RadioMenuFlyoutItem
            {
                Text = title,
                GroupName = "color",
                IsChecked = Look.Color == hex,
            };
            color.Click += (_, _) => Look.Color = hex;
            menu.Items.Add(color);
        }
        menu.ShowAt(
            root,
            new Microsoft.UI.Xaml.Controls.Primitives.FlyoutShowOptions { Position = place }
        );
    }

    private void Minimize()
    {
        if (AppWindow.Presenter is OverlappedPresenter presenter)
            presenter.Minimize();
    }

    private void ToggleMaximized()
    {
        if (Maximized)
            Restore();
        else if (AppWindow.Presenter is OverlappedPresenter presenter)
            presenter.Maximize();
    }

    /// <summary>The user closes the sticky with its button: it is gone then.</summary>
    private void CloseByButton()
    {
        closedByButton = true;
        Close();
    }

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
            (int)rows[panel.FirstVisibleIndex].Row.Position,
            (int)rows[Math.Max(panel.FirstCacheIndex, 0)].Row.Position,
            (int)rows[Math.Max(panel.LastCacheIndex, panel.FirstVisibleIndex)].Row.Position
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
        };
        var yes = await dialog.ShowAsync() == ContentDialogResult.Primary;
        asked = null;
        if (closed)
            return;
        Shell.Answer(yes);
        // The next question waited behind this one
        Ask();
    }

    private async void ReadClipboard(Action<string?> hand) => hand(await WindowsClipboard.Text());

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

    // ---- The window ----

    private void Place(PetaCore.PetaSticky? kept, StickyWindow? before)
    {
        if (kept is { } sticky)
        {
            AppWindow.Move(new PointInt32((int)Math.Round(sticky.X), (int)Math.Round(sticky.Y)));
            // After the move: the scale is that of the screen the window is on
            AppWindow.Resize(Pixels(sticky.Width, sticky.Height));
            return;
        }
        if (before is null)
        {
            AppWindow.Resize(Pixels(Usual.Width, Usual.Height));
            return;
        }
        // A little down and to the right of the sticky the user worked in
        var step = (int)Math.Round(BarHeight * before.Scale);
        var corner = before.AppWindow.Position;
        var work = DisplayArea
            .GetFromWindowId(before.AppWindow.Id, DisplayAreaFallback.Nearest)
            .WorkArea;
        AppWindow.Move(new PointInt32(corner.X + step, corner.Y + step));
        // After the move: the scale is that of the screen the window is on
        AppWindow.Resize(Pixels(Usual.Width, Usual.Height));
        var size = AppWindow.Size;
        var inside = new Area(corner.X + step, corner.Y + step, size.Width, size.Height).Within(
            new Area(work.X, work.Y, work.Width, work.Height)
        );
        AppWindow.Move(new PointInt32(inside.Left, inside.Top));
    }

    private SizeInt32 Pixels(double width, double height) =>
        new((int)Math.Round(width * Scale), (int)Math.Round(height * Scale));

    /// <summary>How high the window is when only its title bar is shown, in pixels.</summary>
    private int BarPixels =>
        AppWindow.Size.Height - AppWindow.ClientSize.Height + (int)Math.Round(BarHeight * Scale);

    /// <summary>The window is as large as the screen.</summary>
    private bool Maximized => Native.IsZoomed(handle);

    /// <summary>Makes a window that is as large as the screen as it was before.</summary>
    private void Restore()
    {
        if (!closed && Maximized && AppWindow.Presenter is OverlappedPresenter presenter)
            presenter.Restore();
    }

    /// <summary>
    /// Takes the place and the size of the sticky from the window. Not while the window is as
    /// large as the screen or minimized: the sticky keeps the place and the size it goes back
    /// to. Windows puts a minimized window off the screen, as high as its title bar.
    /// </summary>
    private void Measure()
    {
        if (Maximized || Native.IsIconic(handle))
            return;
        var size = AppWindow.Size;
        // What is kept is the whole sticky, also while only its title bar is shown
        var height = RolledUp || rolling ? fullHeight : size.Height;
        place = new RectInt32(AppWindow.Position.X, AppWindow.Position.Y, size.Width, height);
    }

    /// <summary>Keeps the sticky as it is now.</summary>
    public void Keep()
    {
        if (closed)
            return;
        Measure();
        desk.Keep(
            new PetaCore.PetaSticky(
                Id,
                Shell.Url,
                place.X,
                place.Y,
                place.Width / Scale,
                place.Height / Scale,
                Look.Color,
                Look.AlwaysOnTop,
                Look.Zoom,
                RolledUp
            )
        );
    }

    /// <summary>
    /// The tools and the buttons of the window are for the sticky that is worked on. On the
    /// others they are only in the way of the name, and a sticky that is rolled up has no
    /// paper to work on: a double click rolls it down. The button for a new sticky stays.
    /// </summary>
    private void ShowTools()
    {
        var shown = hasFocus && !RolledUp ? Visibility.Visible : Visibility.Collapsed;
        tools.Visibility = shown;
        windowButtons.Visibility = shown;
        large.Glyph = Maximized ? "\uE923" : "\uE922";
        ShowBare();
    }

    /// <summary>
    /// A sticky without paper that is not worked on shows only what is written on it. The
    /// name and the button stay where they are, unseen: the name is where the window is
    /// moved, and the text does not move when they come back.
    /// </summary>
    private void ShowBare()
    {
        var bare = Look.Bare(hasFocus);
        name.Opacity = bare ? 0 : 1;
        add.Opacity = bare ? 0 : 1;
        add.IsHitTestVisible = !bare;
        // Hidden, and not disabled: the wheel still scrolls the list
        ScrollViewer.SetVerticalScrollBarVisibility(
            list,
            bare ? ScrollBarVisibility.Hidden : ScrollBarVisibility.Auto
        );
        if (bare == Bare)
            return;
        Bare = bare;
        // The line around the window and its shadow go too. The border itself stays: without
        // it the size of the window would change, and the text would move
        var border = bare ? Native.NoColor : Native.UsualColor;
        _ = Native.DwmSetWindowAttribute(handle, Native.BorderColor, ref border, sizeof(uint));
        CutToPaper(bare);
        foreach (var row in rows)
            row.View?.Draw();
    }

    /// <summary>
    /// Cuts the window to its paper, or makes it whole again. Windows draws no shadow under a
    /// window that has a region. The frame is left alone: without the frame of DWM, Windows
    /// draws the old frame of Windows instead. Only the edges where the window is made larger
    /// are cut off. They come back with the focus.
    /// </summary>
    private void CutToPaper(bool cut)
    {
        if (!cut)
        {
            _ = Native.SetWindowRgn(handle, IntPtr.Zero, true);
            return;
        }
        var corner = new Native.Point();
        if (
            !Native.GetWindowRect(handle, out var outer)
            || !Native.GetClientRect(handle, out var paper)
            || !Native.ClientToScreen(handle, ref corner)
        )
            return;
        var left = corner.X - outer.Left;
        var top = corner.Y - outer.Top;
        _ = Native.SetWindowRgn(
            handle,
            Native.CreateRectRgn(left, top, left + paper.Right, top + paper.Bottom),
            true
        );
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
        // Windows tells every window when the PC woke or the clock was set. The window goes on
        // to hear it as it would without this
        if (DayChange.MayFollow(message, wParam))
            Shell.DayMayHaveChanged();
        switch (message)
        {
            // Windows would paint the background of the window over what shows through
            case Native.WmEraseBackground when Look.Transparent && clear is not null:
                clear.Erase(wParam);
                return 1;
            // Windows draws its title bar of old in the window then: it is wiped away
            case Native.WmNcActivate
            or Native.WmNcPaint
            or Native.WmSetText when Look.Transparent && clear is not null:
            {
                var answer = Native.DefSubclassProc(window, message, wParam, lParam);
                clear.Wipe();
                return answer;
            }
            // The title bar is no part of the content: its clicks come only here. Windows
            // still moves the window by it
            case Native.WmNcLeftButtonDown when wParam == Native.HitCaption:
                if (Shell.View.Outline.SelectionMode)
                    Shell.ClickedOutsideItems();
                return Native.DefSubclassProc(window, message, wParam, lParam);
            // A double click on the title bar rolls the sticky up. Windows would make the
            // window as large as the screen: that is left to the button of the title bar
            case Native.WmNcLeftButtonDoubleClick when wParam == Native.HitCaption:
                if (Maximized)
                    Restore();
                else
                    Roll();
                return IntPtr.Zero;
            // A right click on the title bar shows the menu of the sticky. Windows would show
            // its menu of the window: Alt+Space still does
            case Native.WmNcRightButtonUp when wParam == Native.HitCaption:
            {
                var place = lParam.ToInt64();
                ShowMenu(
                    new Native.Point
                    {
                        X = (short)(place & 0xFFFF),
                        Y = (short)(place >> 16 & 0xFFFF),
                    }
                );
                return IntPtr.Zero;
            }
            case Native.WmSysCommand
                when (wParam.ToInt64() & 0xFFF0) == Native.SysMaximize && (RolledUp || rolling):
                // On the way up or down the sticky has no height to come back to yet
                if (rolling)
                    return IntPtr.Zero;
                // A sticky that is rolled up keeps the height of its title bar, also when it
                // is made large. It is rolled down first
                Roll(animated: false);
                return Native.DefSubclassProc(window, message, wParam, lParam);
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
