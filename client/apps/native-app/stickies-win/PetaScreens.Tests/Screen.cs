using System.Globalization;
using System.Runtime.InteropServices;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using PetaBorder;
using PetaBorder.Tests;
using PetaTypes;
using Windows.ApplicationModel.DataTransfer;
using Windows.System;

namespace PetaScreens.Tests;

/// <summary>
/// The stickies of a test, on a journal server of its own. The windows are real, and they are
/// brought to the front: the keys and the clicks of a test go to them through Windows.
/// </summary>
internal sealed class Screen : IAsyncDisposable
{
    private readonly JournalServer server;
    private readonly string folder;

    /// <summary>The desks of the runs of the app before this one, which a test ended.</summary>
    private readonly List<StickyDesk> earlier = [];

    private Screen(int port)
    {
        server = new JournalServer(port);
        folder = Path.Combine(Path.GetTempPath(), $"peta-screens-test-{Guid.NewGuid()}");
        File = Path.Combine(folder, "stickies.json");
        Desk = NewDesk();
    }

    /// <summary>The file that keeps the stickies for the next run of the app.</summary>
    public string File { get; }

    public StickyDesk Desk { get; private set; }

    /// <summary>The sticky that was opened first.</summary>
    public StickyWindow Window => Desk.Stickies[0];

    public Shell Shell => Window.Shell;

    public IReadOnlyList<Row> Rows => Shell.View.Outline.Rows;

    /// <summary>The address of the journal server of the test.</summary>
    public string Server => server.Address;

    /// <summary>
    /// A sticky that shows the page at <paramref name="url"/>, once its first item is there.
    /// Without a URL, the desk opens the stickies that were kept.
    /// </summary>
    public static Task<Screen> Open(int port, string? url = "/pages/screen") =>
        Start(new Screen(port), async screen =>
        {
            if (url is null)
                screen.Desk.Open();
            else
            {
                screen.Desk.Add(url);
                await Until("the item is there", () => screen.Items().Count == 1);
            }
        });

    /// <summary>
    /// A sticky that shows a page of <paramref name="items"/> items, once all of them are
    /// there. The page is made on the server before the sticky opens. Each item has a line of
    /// text, at one of three levels.
    /// </summary>
    public static Task<Screen> OpenLong(int port, int items) =>
        Start(new Screen(port), async screen =>
        {
            screen.MakePage(
                "long",
                [
                    .. Enumerable
                        .Range(0, items)
                        .Select(n => (n % 3 + 1, n == 0 ? "" : $"Item {n} with some text to draw")),
                ]
            );
            screen.Desk.Add("/pages/long");
            await Until("the items are there", () => screen.Rows.Count == items);
            await Until("the first items are drawn", () => screen.Items().Count > 0);
        });

    /// <summary>
    /// A sticky that shows a page with an item for each of <paramref name="bodies"/>, once all
    /// of them are drawn. The page is made on the server before the sticky opens.
    /// </summary>
    public static Task<Screen> OpenWritten(int port, params string[] bodies) =>
        Start(new Screen(port), async screen =>
        {
            screen.MakePage("written", [.. bodies.Select(body => (1, body))]);
            screen.Desk.Add("/pages/written");
            await Until("the items are drawn", () => screen.Items().Count == bodies.Length);
        });

    /// <summary>Makes a page on the server with the items, each at its level.</summary>
    private void MakePage(string title, IReadOnlyList<(int Level, string Body)> items)
    {
        var made = server.Answer(HttpMethod.Post, "/api/pages/doc", $$"""{"title":"{{title}}"}""");
        var page = System.Text.Json.JsonElement.Parse(made!);
        var pageId = page.GetProperty("page").GetProperty("id").GetString();
        // A new page has one empty item: it becomes the first
        var first = page.GetProperty("items")[0].GetProperty("id").GetString();
        Assert.IsTrue(
            server.Send(
                HttpMethod.Put,
                $"/api/items/{first}",
                System.Text.Json.JsonSerializer.Serialize(
                    new Dictionary<string, object?>
                    {
                        ["body"] = items[0].Body,
                        ["level"] = items[0].Level,
                    }
                )
            )
        );
        foreach (var (level, body) in items.Skip(1))
        {
            var item = System.Text.Json.JsonSerializer.Serialize(
                new Dictionary<string, object?>
                {
                    ["pageId"] = pageId,
                    ["position"] = "last",
                    ["level"] = level,
                    ["body"] = body,
                }
            );
            Assert.IsTrue(server.Send(HttpMethod.Post, "/api/items", item));
        }
    }

    /// <summary>Makes the journal of the day on the server, with one item.</summary>
    public void MakeJournal(string day, string body)
    {
        var made = server.Answer(HttpMethod.Post, $"/api/journals/{day}", "{}");
        var first = System
            .Text.Json.JsonElement.Parse(made!)
            .GetProperty("items")[0]
            .GetProperty("id")
            .GetString();
        Assert.IsTrue(
            server.Send(
                HttpMethod.Put,
                $"/api/items/{first}",
                System.Text.Json.JsonSerializer.Serialize(
                    new Dictionary<string, object?> { ["body"] = body }
                )
            )
        );
    }

    /// <summary>Makes the item show the item <paramref name="sourceId"/>, as the server does.</summary>
    public void Transclude(string itemId, string sourceId) =>
        Assert.IsTrue(
            server.Send(
                HttpMethod.Post,
                $"/api/items/{itemId}/transclude",
                $$"""{"sourceItemId":"{{sourceId}}"}"""
            )
        );

    private static async Task<Screen> Start(Screen screen, Func<Screen, Task> open)
    {
        current = screen;
        FocusManager.GotFocus += screen.FocusMoved;
        try
        {
            await open(screen);
            screen.ToFront();
            return screen;
        }
        catch
        {
            // The test has no screen to end yet: without this, its windows keep the app open
            await screen.DisposeAsync();
            throw;
        }
    }

    /// <summary>Ends the run of the app, and starts the next one on the same file.</summary>
    public void RunAgain()
    {
        Desk.End();
        earlier.Add(Desk);
        Desk = NewDesk();
        Desk.Open();
    }

    private StickyDesk NewDesk() => new(server.Address, new StickyStore(File));

    /// <summary>Where the focus went while the test ran, the last moves last.</summary>
    private readonly List<string> moves = [];

    private void FocusMoved(object? sender, FocusManagerGotFocusEventArgs e)
    {
        var view = e.NewFocusedElement as DependencyObject;
        while (view is not null and not RowView)
            view = VisualTreeHelper.GetParent(view);
        var item = (view as RowView)?.ItemId is { } id
            ? Rows.FirstOrDefault(row => row.Item.Id == id)?.Item.Body ?? id
            : null;
        moves.Add($"{e.NewFocusedElement?.GetType().Name}{(item is null ? "" : $"({item})")}");
    }

    /// <summary>The screen of the test that runs now. A wait that fails tells how it looks.</summary>
    private static Screen? current;

    /// <summary>
    /// Waits until <paramref name="done"/>, while the windows go on. A wait that fails tells
    /// <paramref name="more"/> too, after how the screen looks.
    /// </summary>
    public static async Task Until(string what, Func<bool> done, Func<string>? more = null)
    {
        var deadline = DateTime.Now.AddSeconds(10);
        while (!done())
        {
            if (DateTime.Now > deadline)
                Assert.Fail(
                    $"never happened: {what}\n{current?.Describe()}"
                        + (more is null ? "" : $"\n{more()}")
                );
            await Task.Delay(10);
        }
    }

    /// <summary>What the test sees on the screen, and what the core holds.</summary>
    private string Describe()
    {
        if (Desk.Stickies.Count == 0)
            return "no sticky is open";
        var focused = FocusManager.GetFocusedElement(Window.Content.XamlRoot);
        var rows = Rows.Select(row =>
            $"{row.Item.Body}{(row.IsEditing ? " editing" : "")}{(row.IsSelected ? " selected" : "")}"
        );
        return $"front={GetForegroundWindow() == Handle(Window)} focus={focused?.GetType().Name} "
            + $"cursor={Focused?.ToString(CultureInfo.InvariantCulture) ?? "none"} views={Items().Count} "
            + $"rows=[{string.Join(" | ", rows)}] selecting={Shell.View.Outline.SelectionMode} "
            + $"focus moves=[{string.Join(" > ", moves.TakeLast(12))}]";
    }

    /// <summary>Brings the first sticky to the front, where the keys of Windows go.</summary>
    public void ToFront() => BringToFront(Handle(Window));

    /// <summary>
    /// Puts the window in front. Windows refuses SetForegroundWindow to a process that did not
    /// send the last input. A move of the mouse by nothing makes the tests that process.
    /// </summary>
    private static void BringToFront(IntPtr window)
    {
        if (GetForegroundWindow() == window)
            return;
        UserInput.Nudge();
        SetForegroundWindow(window);
    }

    /// <summary>The items of the first sticky as it shows them, from the top down.</summary>
    public List<RowView> Items() => Items(Window);

    /// <summary>
    /// The views of the items that the core shows, in its order. Not in the order of their
    /// places on the screen: the view of a new item is at the top until it is laid out.
    /// </summary>
    public static List<RowView> Items(StickyWindow window)
    {
        var order = window.Shell.View.Outline.Rows.Select(row => row.Item.Id).ToList();
        return
        [
            .. Under<RowView>((UIElement)window.Content)
                .Where(item => item.ActualHeight > 0 && order.Contains(item.ItemId ?? ""))
                .OrderBy(item => order.IndexOf(item.ItemId ?? "")),
        ];
    }

    /// <summary>The text box that edits the item. It is hidden while the item is not edited.</summary>
    public static TextBox EditorOf(RowView item) => Under<TextBox>(item).First();

    /// <summary>The bullet before the text of the item: the dots are drawn in it.</summary>
    public static UIElement BulletOf(RowView item) =>
        (UIElement)
            VisualTreeHelper.GetParent(Under<Microsoft.UI.Xaml.Shapes.Ellipse>(item).First());

    /// <summary>The number of the item whose editor has the cursor, if one has it.</summary>
    public int? Focused =>
        FocusManager.GetFocusedElement(Window.Content.XamlRoot) is TextBox focused
        && Items().FindIndex(item => EditorOf(item) == focused) is var at
        && at >= 0
            ? at
            : null;

    public List<string> Bodies() => [.. Rows.Select(row => row.Item.Body)];

    public bool BodiesAre(params string[] bodies) => Bodies().SequenceEqual(bodies);

    public bool SelectedAre(params string[] bodies) => Selected().SequenceEqual(bodies);

    /// <summary>
    /// Clicks the element of the sticky with the mouse, at <paramref name="across"/> of its
    /// width, or clicks it twice. The sticky is the first one when none is named.
    /// </summary>
    public async Task Click(
        FrameworkElement element,
        StickyWindow? sticky = null,
        double across = 0.5,
        Held held = Held.None,
        bool right = false,
        bool twice = false
    )
    {
        var target = await PlaceToPress(element, sticky ?? Window, across);
        if (twice)
            await UserInput.DoubleClick(target.X, target.Y);
        else
            await UserInput.Click(target.X, target.Y, held, right);
    }

    /// <summary>
    /// Drags with the left button from the middle of <paramref name="from"/> to
    /// <paramref name="to"/>, at <paramref name="down"/> of its height. <paramref name="held"/>
    /// runs before the button comes up.
    /// </summary>
    public async Task Drag(
        FrameworkElement from,
        FrameworkElement to,
        double down = 0.5,
        Func<Task>? held = null
    )
    {
        var start = await PlaceToPress(from, Window, 0.5);
        await UserInput.Drag(start, ScreenPlace(to, Window, 0.5, down), held);
    }

    /// <summary>
    /// The place on the screen to press the element, once the element is laid out, its sticky
    /// is in front and the place stays the same.
    /// </summary>
    private static async Task<(int X, int Y)> PlaceToPress(
        FrameworkElement element,
        StickyWindow sticky,
        double across
    )
    {
        // A window that was just opened is not laid out yet
        await Until("the element is on the screen", () =>
            element.XamlRoot is not null && element.ActualWidth > 0
        );
        await Until("the sticky is in front", () =>
        {
            BringToFront(Handle(sticky));
            return GetForegroundWindow() == Handle(sticky);
        });
        // A window that was just opened may still move or grow, and a click on the place of a
        // moment ago lands beside the element. The place has to stay the same for a while
        (int X, int Y) target = default;
        var still = 0;
        await Until("the element stays in place", () =>
        {
            var now = ScreenPlace(element, sticky, across);
            still = now == target ? still + 1 : 0;
            target = now;
            return still >= 5;
        });
        return target;
    }

    /// <summary>
    /// The place on the screen, in pixels, of a point of the element: at
    /// <paramref name="across"/> of its width and <paramref name="down"/> of its height.
    /// </summary>
    private static (int X, int Y) ScreenPlace(
        FrameworkElement element,
        StickyWindow sticky,
        double across,
        double down = 0.5
    )
    {
        var root = (UIElement)sticky.Content;
        var place = element
            .TransformToVisual(root)
            .TransformPoint(new(element.ActualWidth * across, element.ActualHeight * down));
        var scale = root.XamlRoot.RasterizationScale;
        var corner = new Point();
        ClientToScreen(Handle(sticky), ref corner);
        return (
            corner.X + (int)Math.Round(place.X * scale),
            corner.Y + (int)Math.Round(place.Y * scale)
        );
    }

    /// <summary>The entry with the text in the menu that the first sticky shows now.</summary>
    public async Task<MenuFlyoutItem> MenuEntry(string text)
    {
        MenuFlyoutItem? found = null;
        await Until($"the menu has {text}", () =>
        {
            found = VisualTreeHelper
                .GetOpenPopupsForXamlRoot(Window.Content.XamlRoot)
                .Where(popup => popup.Child is not null)
                .SelectMany(popup => Under<MenuFlyoutItem>(popup.Child))
                .FirstOrDefault(entry => entry.Text == text && entry.ActualWidth > 0);
            return found is not null;
        });
        return found!;
    }

    /// <summary>The button with the text in the dialog that the first sticky shows now.</summary>
    public async Task<Button> DialogButton(string text)
    {
        Button? found = null;
        await Until($"the dialog has {text}", () =>
        {
            found = Dialogs()
                .SelectMany(Under<Button>)
                .FirstOrDefault(button =>
                    button.Content as string == text && button.ActualWidth > 0
                );
            return found is not null;
        });
        return found!;
    }

    /// <summary>The dialogs that the first sticky shows now.</summary>
    public IEnumerable<ContentDialog> Dialogs() =>
        VisualTreeHelper
            .GetOpenPopupsForXamlRoot(Window.Content.XamlRoot)
            .Select(popup => popup.Child)
            .OfType<ContentDialog>();

    /// <summary>
    /// Clicks the title bar of the sticky, in the room left of the Back button, or clicks it
    /// twice.
    /// </summary>
    public Task ClickTitleBar(
        StickyWindow? sticky = null,
        bool right = false,
        bool twice = false
    ) =>
        Click(
            ButtonOf(sticky ?? Window, "Back (Alt+Left)"),
            sticky,
            across: -3,
            right: right,
            twice: twice
        );

    /// <summary>The button of the title bar of the sticky that has the tip.</summary>
    public static Button ButtonOf(StickyWindow sticky, string tip) =>
        Under<Button>((UIElement)sticky.Content)
            .First(button => ToolTipService.GetToolTip(button) as string == tip);

    /// <summary>Clicks the item and waits until its editor has the cursor.</summary>
    public async Task Edit(int item)
    {
        var view = Items()[item];
        var editor = EditorOf(view);
        // Why the marks: the cursor did not come into a clicked item in the CI once, and never
        // on a developer's PC (#1175). They tell which part of the click the item saw, and when
        var clock = System.Diagnostics.Stopwatch.StartNew();
        var seen = new List<string>();
        void Mark(string what) => seen.Add($"{what}@{clock.ElapsedMilliseconds}ms");
        var pressed = new PointerEventHandler((_, _) => Mark("pressed"));
        var released = new PointerEventHandler((_, _) => Mark("released"));
        var tapped = new TappedEventHandler((_, _) => Mark("tapped"));
        void GotFocus(object sender, RoutedEventArgs e) => Mark("editor got focus");
        void LostFocus(object sender, RoutedEventArgs e) => Mark("editor lost focus");
        view.AddHandler(UIElement.PointerPressedEvent, pressed, true);
        view.AddHandler(UIElement.PointerReleasedEvent, released, true);
        view.AddHandler(UIElement.TappedEvent, tapped, true);
        editor.GotFocus += GotFocus;
        editor.LostFocus += LostFocus;
        try
        {
            await Click(view);
            Mark("sent");
            await Until(
                $"item {item} has the cursor",
                () => Focused == item,
                () =>
                    $"item saw=[{string.Join(", ", seen)}] "
                    + $"editor visible={editor.Visibility == Visibility.Visible}"
            );
        }
        finally
        {
            view.RemoveHandler(UIElement.PointerPressedEvent, pressed);
            view.RemoveHandler(UIElement.PointerReleasedEvent, released);
            view.RemoveHandler(UIElement.TappedEvent, tapped);
            editor.GotFocus -= GotFocus;
            editor.LostFocus -= LostFocus;
        }
    }

    /// <summary>The editor that has the cursor.</summary>
    public TextBox Editor
    {
        get
        {
            Assert.IsNotNull(Focused, "no editor has the cursor");
            return EditorOf(Items()[Focused.Value]);
        }
    }

    /// <summary>Types into the editor that has the cursor, as the keyboard does.</summary>
    public async Task Type(string text)
    {
        var editor = Editor;
        UserInput.Type(text);
        await Until($"the editor shows {text}", () =>
            editor.Text.Contains(text, StringComparison.Ordinal)
        );
    }

    /// <summary>
    /// Puts the cursor of the editor that has it after <paramref name="letters"/>, with the
    /// keys: Home, and the right arrow.
    /// </summary>
    public async Task MoveCursor(int letters)
    {
        var editor = Editor;
        Press(VirtualKey.Home);
        for (var i = 0; i < letters; i++)
            Press(VirtualKey.Right);
        await Until($"the cursor is after {letters} letters", () =>
            editor.SelectionStart == letters && editor.SelectionLength == 0
        );
    }

    /// <summary>Two items, "first" and "second", with the cursor at the end of the second.</summary>
    public async Task TwoItems()
    {
        await Edit(0);
        await Type("first");
        Press(VirtualKey.Enter);
        await Until("the cursor is in the new item", () => Items().Count == 2 && Focused == 1);
        await Type("second");
        await Until("the core has both and edits the second", () =>
            BodiesAre("first", "second")
            && Rows[1].IsEditing
            && Shell.View.Outline.FocusedItemId == Rows[1].Item.Id
            && Focused == 1
        );
        await Settle();
    }

    /// <summary>
    /// Waits for the focus events that WinUI sends late. The editor of the first item tells
    /// the core that it lost the cursor a moment after the next one got it. A key pressed
    /// before that ends a selection that it starts. A user is not that fast.
    /// </summary>
    private static Task Settle() => Task.Delay(100);

    /// <summary>The bodies of the selected items.</summary>
    public List<string> Selected() =>
        [.. Rows.Where(row => row.IsSelected).Select(row => row.Item.Body)];

    /// <summary>
    /// Two items, with the second one selected by Shift+Up in its editor. The keys go to the
    /// sticky then: no editor has the cursor while items are selected.
    /// </summary>
    public async Task SecondOfTwoSelected()
    {
        await TwoItems();
        Press(VirtualKey.Up, Held.Shift);
        await Until("the second item is selected and no editor has the cursor", () =>
            SelectedAre("second") && Focused is null
        );
    }

    /// <summary>Puts the text on the clipboard of Windows, as another app does.</summary>
    public static void PutOnClipboard(string text)
    {
        var content = new DataPackage();
        content.SetText(text);
        Clipboard.SetContent(content);
    }

    /// <summary>
    /// Puts a picture of one pixel on the clipboard of Windows as a bitmap, as a screenshot
    /// tool does.
    /// </summary>
    public static async Task PutImageOnClipboard()
    {
        var stream = new Windows.Storage.Streams.InMemoryRandomAccessStream();
        var encoder = await Windows.Graphics.Imaging.BitmapEncoder.CreateAsync(
            Windows.Graphics.Imaging.BitmapEncoder.PngEncoderId,
            stream
        );
        encoder.SetPixelData(
            Windows.Graphics.Imaging.BitmapPixelFormat.Bgra8,
            Windows.Graphics.Imaging.BitmapAlphaMode.Premultiplied,
            1,
            1,
            96,
            96,
            [0, 0, 255, 255]
        );
        await encoder.FlushAsync();
        var content = new DataPackage();
        content.SetBitmap(
            Windows.Storage.Streams.RandomAccessStreamReference.CreateFromStream(stream)
        );
        Clipboard.SetContent(content);
    }

    /// <summary>Waits until the clipboard of Windows has the text.</summary>
    public static async Task UntilClipboardHas(string text)
    {
        var deadline = DateTime.Now.AddSeconds(10);
        string? found = null;
        while (found != text)
        {
            if (DateTime.Now > deadline)
                Assert.Fail(
                    $"the clipboard never had {text}: it has {found}\n{current?.Describe()}"
                );
            await Task.Delay(10);
            var content = Clipboard.GetContent();
            found = content.Contains(StandardDataFormats.Text)
                ? (await content.GetTextAsync()).Replace("\r\n", "\n")
                : null;
        }
    }

    /// <summary>Presses the key in the window in front.</summary>
    public static void Press(VirtualKey key, Held held = Held.None) => UserInput.Press(key, held);

    private static IntPtr Handle(StickyWindow window) =>
        WinRT.Interop.WindowNative.GetWindowHandle(window);

    /// <summary>The elements of the type under the parent, at every depth.</summary>
    public static IEnumerable<T> Under<T>(DependencyObject parent)
        where T : DependencyObject
    {
        for (var i = 0; i < VisualTreeHelper.GetChildrenCount(parent); i++)
        {
            var child = VisualTreeHelper.GetChild(parent, i);
            if (child is T found)
                yield return found;
            foreach (var deeper in Under<T>(child))
                yield return deeper;
        }
    }

    public async ValueTask DisposeAsync()
    {
        current = null;
        FocusManager.GotFocus -= FocusMoved;
        Desk.End();
        // The windows close on a later turn of the UI thread
        await Task.Delay(50);
        server.Dispose();
        try
        {
            Directory.Delete(folder, recursive: true);
        }
        catch (DirectoryNotFoundException)
        {
            // No sticky was kept
        }
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Point
    {
        public int X;
        public int Y;
    }

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetForegroundWindow(IntPtr window);

    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ClientToScreen(IntPtr window, ref Point point);
}
