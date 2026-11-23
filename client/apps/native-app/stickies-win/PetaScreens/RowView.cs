using System.Runtime.InteropServices;
using Microsoft.UI;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Documents;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using Microsoft.UI.Xaml.Shapes;
using PetaBorder;
using PetaTypes;
using Windows.ApplicationModel.DataTransfer;
using Windows.UI;
using Windows.UI.Text;
using Border = Microsoft.UI.Xaml.Controls.Border;

namespace PetaScreens;

/// <summary>A row of the outline as the list keeps it: the list draws only the rows near the
/// window.</summary>
internal sealed class RowModel(Row row)
{
    public string Id { get; } = row.Item.Id;
    public Row Row { get; private set; } = row;

    /// <summary>The view that draws the row now, when the list has made one for it.</summary>
    public RowView? View { get; set; }

    public void Set(Row row)
    {
        Row = row;
        View?.Draw();
    }
}

/// <summary>
/// One item: its bullet and its text, as far to the right as its level says.
///
/// The item that is edited shows its Markdown in a text box. Every other item shows its text
/// with its decoration (ADR 025), and cannot be typed into. The core says which item is edited.
/// </summary>
internal sealed class RowView : Grid
{
    /// <summary>
    /// The size of the usual text, at a zoom of 1. The SwiftUI frontend has the same size.
    /// </summary>
    public const double FontSize = 13;

    /// <summary>
    /// How much room there is between two items, at a zoom of 1. The SwiftUI frontend has
    /// about the same: its list puts this room around each row.
    /// </summary>
    private const double RowGap = 8;

    /// <summary>How high a line is, in sizes of its font.</summary>
    private const double LineHeight = 1.34;

    private static readonly SolidColorBrush Ink = new(Color.FromArgb(255, 0x1F, 0x1F, 0x1F));
    private static readonly SolidColorBrush Pale = new(Color.FromArgb(255, 0x6E, 0x6E, 0x6E));
    private static readonly SolidColorBrush Faint = new(Color.FromArgb(0x4D, 0x6E, 0x6E, 0x6E));
    private static readonly SolidColorBrush Quoted = new(Color.FromArgb(255, 0x55, 0x55, 0x55));
    private static readonly SolidColorBrush Clear = new(Colors.Transparent);
    private static readonly SolidColorBrush Chosen = new(Color.FromArgb(0x40, 0x00, 0x78, 0xD4));
    private static readonly SolidColorBrush Marked = new(Color.FromArgb(255, 0xFF, 0xE0, 0x66));
    private static readonly SolidColorBrush CodePaper = new(Color.FromArgb(0x16, 0, 0, 0));
    private static readonly SolidColorBrush Hidden = new(Color.FromArgb(255, 0xDD, 0xDD, 0xDD));
    private static readonly FontFamily CodeFont = new("Consolas");

    /// <summary>
    /// The font of all text, the one the web frontend shows on Windows: Segoe UI, and Noto
    /// Sans JP for the Japanese letters that Segoe UI does not have.
    /// </summary>
    public static readonly FontFamily TextFont = new("Segoe UI, Noto Sans JP");

    private readonly StickyWindow window;
    private readonly StackPanel header = new();
    private readonly StackPanel marks = new()
    {
        Orientation = Orientation.Horizontal,
        // The marks stand beside the first line. Without this they go to the middle of an
        // item of many lines
        VerticalAlignment = VerticalAlignment.Top,
    };
    private readonly Border paper = new() { Background = Clear };
    private readonly StackPanel shown = new();
    private readonly TextBox editor;

    private RowModel? model;

    /// <summary>The editor has the cursor.</summary>
    private bool hasCursor;

    /// <summary>The input method is composing text in the editor.</summary>
    private bool composing;

    /// <summary>What the paper shows now, so that it is not made again for nothing.</summary>
    private (string Body, double Zoom)? drawn;

    /// <summary>
    /// The runs on the paper, each with the place of its first letter in the text of its
    /// paragraph, in UTF-16 units.
    /// </summary>
    private readonly List<(RichTextBlock Block, Run Run, int Start)> runs = [];

    /// <summary>When a link was last clicked.</summary>
    private long followed;

    public RowView(StickyWindow window)
    {
        this.window = window;
        RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        ColumnDefinitions.Add(new ColumnDefinition());

        editor = new TextBox
        {
            AcceptsReturn = true,
            TextWrapping = TextWrapping.Wrap,
            BorderThickness = new Thickness(0),
            Padding = new Thickness(0),
            MinHeight = 0,
            MinWidth = 0,
            FontFamily = TextFont,
            Foreground = Ink,
            IsSpellCheckEnabled = false,
            Visibility = Visibility.Collapsed,
        };
        // The text box is as plain as the paper: no box, and no other color with the cursor
        foreach (
            var name in new[]
            {
                "TextControlBackground",
                "TextControlBackgroundPointerOver",
                "TextControlBackgroundFocused",
                "TextControlBorderBrush",
                "TextControlBorderBrushPointerOver",
                "TextControlBorderBrushFocused",
            }
        )
            editor.Resources[name] = Clear;
        foreach (
            var name in new[]
            {
                "TextControlForeground",
                "TextControlForegroundPointerOver",
                "TextControlForegroundFocused",
            }
        )
            editor.Resources[name] = Ink;
        editor.Resources["TextControlBorderThemeThicknessFocused"] = new Thickness(0);
        editor.PreviewKeyDown += Pressed;
        // Also when the text box took it: the text box selects its text with the mouse
        editor.AddHandler(PointerPressedEvent, new PointerEventHandler(PressedOn(editor)), true);
        paper.AddHandler(PointerPressedEvent, new PointerEventHandler(PressedOn(paper)), true);
        editor.TextChanging += (_, _) => Typed();
        // The core pastes, and not the text box: a list that was copied becomes items
        editor.Paste += (_, e) =>
        {
            e.Handled = true;
            PasteFromClipboard(raw: false);
        };
        editor.TextCompositionStarted += (_, _) => composing = true;
        editor.TextCompositionEnded += (_, _) =>
        {
            composing = false;
            // What was composed is in the text now
            Typed();
        };
        editor.GotFocus += (_, _) => GotCursor();
        editor.LostFocus += (_, _) => LostCursor();

        paper.Child = shown;
        // A click beside the text puts the cursor at its end
        paper.Tapped += (_, e) =>
        {
            e.Handled = true;
            Edit(int.MaxValue);
        };

        // A right click on an item that is not edited shows the menu of the item and the
        // sticky. The item that is edited has the menu of its text box: cut, copy and paste
        paper.RightTapped += (_, e) =>
        {
            e.Handled = true;
            window.ShowMenu(e.GetPosition(null), model?.Id);
        };

        SetColumnSpan(header, 2);
        SetRow(marks, 1);
        SetRow(paper, 1);
        SetColumn(paper, 1);
        SetRow(editor, 1);
        SetColumn(editor, 1);
        Children.Add(header);
        Children.Add(marks);
        Children.Add(paper);
        Children.Add(editor);
    }

    private Shell Shell => window.Shell;

    /// <summary>The mouse went down on the text: a drag from it may select items.</summary>
    private PointerEventHandler PressedOn(FrameworkElement body) =>
        (_, e) =>
        {
            if (model is not null)
                window.DragFrom(body, model.Id, e);
        };

    private double Zoom => window.Look.Zoom;

    /// <summary>The room above an item and below it: half of the room between two items.</summary>
    private double HalfGap => Math.Round(RowGap / 2 * Zoom);

    /// <summary>The Markdown in the editor, with the line breaks of the core.</summary>
    private string Markdown => editor.Text.Replace("\r\n", "\n").Replace('\r', '\n');

    /// <summary>The item that the view shows now. The list gives a view to another row later.</summary>
    internal string? ItemId => model?.Id;

    /// <summary>The list gives the view to a row.</summary>
    public void Show(RowModel row)
    {
        Leave();
        model = row;
        row.View = this;
        drawn = null;
        Draw();
    }

    /// <summary>The list takes the view from its row: the row left the window.</summary>
    public void Leave()
    {
        if (model is null)
            return;
        var left = model;
        model = null;
        if (left.View == this)
            left.View = null;
        if (hasCursor)
        {
            hasCursor = false;
            // Not at once: the list takes the view in the middle of a change of its rows
            Shell.FinishEditingLater(left.Id);
        }
        composing = false;
    }

    public void Draw()
    {
        if (model is not { Row: var row })
            return;
        Margin = new Thickness(row.IndentPx * Zoom, 0, 0, 0);
        Background = row.IsSelected ? Chosen : null;
        // The room is inside the paper, and not around the row: a click between two items
        // still puts the cursor in one, and the color of chosen items has no holes
        paper.Padding = new Thickness(0, HalfGap, 0, HalfGap);
        editor.Margin = new Thickness(0, HalfGap, 0, HalfGap);
        DrawHeader(row);
        DrawMarks(row);
        // The editor with the cursor is Markdown at once, before the core counts its item as
        // edited: what is typed right after a click must not be lost
        if (row.IsEditing || hasCursor)
            DrawMarkdown(row);
        else
            DrawShown(row);
    }

    /// <summary>
    /// The first item of a day of a journal, below another day: a line between the days, and
    /// the date of this one.
    /// </summary>
    private void DrawHeader(Row row)
    {
        header.Children.Clear();
        if (row.DateHeader is not { } date)
            return;
        header.Children.Add(
            new Border
            {
                Height = 1,
                Background = Faint,
                Margin = new Thickness(0, 4, 0, 0),
            }
        );
        header.Children.Add(
            new TextBlock
            {
                Text = date,
                FontFamily = TextFont,
                FontSize = FontSize * 1.3 * Zoom,
                FontWeight = FontWeights.Bold,
                Foreground = Ink,
                Margin = new Thickness(0, 8, 0, 8),
            }
        );
    }

    /// <summary>
    /// What stands before the text of the item: the bullet, and after it the state of a task.
    /// Every item has a bullet, a task as well. Without the bullet an item with no text cannot
    /// be seen.
    /// </summary>
    private void DrawMarks(Row row)
    {
        var line = Math.Round(FontSize * Zoom * LineHeight);
        // The first line of a heading is higher. The marks keep their size and stand in the
        // middle of it, as in the web frontend
        var first = Math.Round(FontSize * Zoom * RowLook.FirstLineScale(row) * LineHeight);
        var side = Math.Round(line * 0.85);
        marks.Children.Clear();
        var bullet = Bullet(row.IsCollapsed, side, Math.Max(line, first));
        // Unseen, and not gone: the text stays where it is when the bullet comes back
        bullet.Opacity = window.Bare ? 0 : 1;
        marks.Children.Add(bullet);
        if (row.Task is { } task)
        {
            var box = TaskBox(task, side, Math.Max(line, first));
            // A click steps the state of the task
            box.Tapped += (_, e) =>
            {
                e.Handled = true;
                if (model is not null)
                    Shell.ClickedTask(model.Id);
            };
            marks.Children.Add(box);
        }
        marks.Margin = new Thickness(0, HalfGap, 4 * Zoom, HalfGap);
    }

    /// <summary>
    /// The bullet of an item: a dot, with a pale circle around it while its children are
    /// hidden. It is drawn, and not written as a letter: a letter does not stand in the middle
    /// of its line.
    /// </summary>
    private static Grid Bullet(bool collapsed, double side, double height)
    {
        var bullet = new Grid { Width = side, Height = height };
        if (collapsed)
            bullet.Children.Add(Dot(side, Faint));
        bullet.Children.Add(Dot(Math.Round(side * 0.31), Pale));
        return bullet;
    }

    private static Ellipse Dot(double side, Brush fill) =>
        new()
        {
            Width = side,
            Height = side,
            Fill = fill,
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center,
        };

    /// <summary>
    /// The box that shows the state of a task: empty, with a line while the task runs, or
    /// checked. It is drawn, and not written as a letter: the three letters come from
    /// different fonts and are of different sizes.
    /// </summary>
    private static Grid TaskBox(TaskState task, double side, double height)
    {
        var stroke = Math.Max(1, side / 11);
        var box = new Grid
        {
            Width = side,
            Height = side,
            VerticalAlignment = VerticalAlignment.Center,
        };
        box.Children.Add(
            new Border
            {
                BorderBrush = Pale,
                BorderThickness = new Thickness(stroke),
                CornerRadius = new CornerRadius(side / 6),
            }
        );
        var sign = new Polyline
        {
            Stroke = Pale,
            StrokeThickness = stroke * 1.4,
            StrokeStartLineCap = PenLineCap.Round,
            StrokeEndLineCap = PenLineCap.Round,
            StrokeLineJoin = PenLineJoin.Round,
        };
        if (task == TaskState.Running)
        {
            sign.Points.Add(new(side * 0.25, side * 0.5));
            sign.Points.Add(new(side * 0.75, side * 0.5));
        }
        else if (task == TaskState.Done)
        {
            sign.Points.Add(new(side * 0.22, side * 0.52));
            sign.Points.Add(new(side * 0.42, side * 0.72));
            sign.Points.Add(new(side * 0.78, side * 0.28));
        }
        box.Children.Add(sign);
        // The whole place of the box takes the click, not only its lines
        var place = new Grid
        {
            Width = side,
            Height = height,
            Background = Clear,
            Margin = new Thickness(2, 0, 0, 0),
        };
        place.Children.Add(box);
        return place;
    }

    // ---- The Markdown of the item, to type into ----

    private void DrawMarkdown(Row row)
    {
        var heading = RowLook.EditedHeading(row);
        // An item that is one heading line has the size of its heading, in bold, as the web
        // frontend has: the text keeps its size when the cursor goes in and out
        editor.FontSize = FontSize * Zoom * RowLook.HeadingScale(heading);
        editor.FontWeight = heading > 0 ? FontWeights.Bold : FontWeights.Normal;
        // Text that is still being composed is not in the core yet, and must not be lost to
        // what the core has
        if (!composing && Markdown != row.Item.Body)
        {
            var cursor = Math.Min(editor.SelectionStart, row.Item.Body.Length);
            editor.Text = row.Item.Body.Replace('\n', '\r');
            editor.SelectionStart = cursor;
        }
        if (editor.Visibility == Visibility.Visible)
            return;
        editor.Visibility = Visibility.Visible;
        paper.Visibility = Visibility.Collapsed;
        drawn = null;
    }

    private void Typed()
    {
        if (composing || model is not { } row)
            return;
        var body = Markdown;
        // The core put this text here itself
        if (body != row.Row.Item.Body)
            Shell.Typed(body, row.Id, editor.SelectionStart);
    }

    private void Pressed(object sender, KeyRoutedEventArgs e)
    {
        // While text is composed every key belongs to the input method. The Enter that ends
        // a composition splits nothing
        if (composing || e.Key == Input.InputMethod || model is not { } row)
            return;
        var held = Input.Held();
        if (e.Key == Windows.System.VirtualKey.Enter && held == Held.Shift)
        {
            // A line break inside the item. Enter alone splits the item
            var at = editor.SelectionStart;
            editor.SelectedText = "\r";
            editor.Select(at + 1, 0);
            e.Handled = true;
            return;
        }
        if (e.Key == Windows.System.VirtualKey.V && held == (Held.Control | Held.Shift))
        {
            // The text goes into the item as it is, as in the web frontend
            e.Handled = true;
            PasteFromClipboard(raw: true);
            return;
        }
        if (e.Key == Windows.System.VirtualKey.Y && held == Held.Control)
        {
            // The history is the one of the core: the text box must not bring back its own
            e.Handled = true;
            return;
        }
        if (Input.ChordOf(e.Key, held) is not { } chord)
            return;
        e.Handled = Shell.Pressed(new KeyPress.InEditor(row.Id, chord, State()));
    }

    /// <summary>The editor as a key or a paste finds it.</summary>
    private EditorState State()
    {
        var (first, last) = CursorLines();
        return new EditorState
        {
            Text = Markdown,
            SelectionStart = (ulong)editor.SelectionStart,
            SelectionEnd = (ulong)(editor.SelectionStart + editor.SelectionLength),
            OnFirstLine = first,
            OnLastLine = last,
        };
    }

    /// <summary>
    /// Sends the image or the text on the clipboard to the core, which pastes it at the
    /// cursor.
    /// </summary>
    private async void PasteFromClipboard(bool raw)
    {
        if (model is not { } row)
            return;
        // The cursor where the paste was asked, before the clipboard is read
        var state = State();
        if (await WindowsClipboard.Image() is { } image)
        {
            Shell.PasteImage(row.Id, state, image);
            return;
        }
        if (await WindowsClipboard.Text() is not { } text)
            return;
        // The core reads no HTML yet: it pastes the text
        Shell.Paste(row.Id, state, text, "", raw);
    }

    /// <summary>Whether the cursor is on the first line, and on the last line, of the text as
    /// it is drawn.</summary>
    private (bool First, bool Last) CursorLines()
    {
        var text = editor.Text;
        if (text.Length == 0)
            return (true, true);
        var mine = Top(editor.SelectionStart + editor.SelectionLength);
        var top = Top(editor.SelectionStart);
        return (Math.Abs(top - Top(0)) < 1, Math.Abs(mine - Top(text.Length)) < 1);

        double Top(int place)
        {
            if (place < text.Length)
                return editor.GetRectFromCharacterIndex(place, false).Top;
            var end = editor.GetRectFromCharacterIndex(text.Length - 1, true);
            // After a line break at the end there is a line with no letter in it
            return text[^1] == '\r' ? end.Bottom : end.Top;
        }
    }

    private void GotCursor()
    {
        hasCursor = true;
        if (model is not { } row)
            return;
        Draw();
        window.GotCursor(row.Id);
    }

    private void LostCursor()
    {
        if (!hasCursor)
            return;
        hasCursor = false;
        if (model is not { } row)
            return;
        Shell.FinishEditing(row.Id);
        // The core may have ended the edit before, while the cursor was still here. Then
        // nothing changed for the row now, and it would go on to show Markdown
        Draw();
    }

    /// <summary>
    /// Puts the cursor in the item, <paramref name="cursor"/> UTF-16 units into its Markdown.
    /// False when the editor cannot have the cursor yet: it is not laid out.
    /// </summary>
    public bool TakeCursor(int? cursor)
    {
        if (model is not { Row: var row })
            return false;
        hasCursor = true;
        DrawMarkdown(row);
        UpdateLayout();
        if (!editor.Focus(FocusState.Programmatic))
        {
            hasCursor = false;
            return false;
        }
        if (cursor is { } place)
            editor.Select(Math.Min(place, editor.Text.Length), 0);
        return true;
    }

    /// <summary>The user clicked beside the text of the item: the cursor goes to its end.</summary>
    private void Edit(int place)
    {
        if (!TakeCursor(place))
            Draw();
    }

    /// <summary>
    /// The user clicked the text of the item after <paramref name="letter"/> letters of its
    /// shown text. The core finds the place in the Markdown, and puts the cursor there.
    /// </summary>
    private void Touch(int? letter)
    {
        if (model is not { } row)
            return;
        if (letter is { } touched)
            window.Touched(row.Id, touched);
        else
            Edit(int.MaxValue);
    }

    // ---- The text of the item with its decoration ----

    private void DrawShown(Row row)
    {
        if (editor.Visibility == Visibility.Visible)
        {
            editor.Visibility = Visibility.Collapsed;
            paper.Visibility = Visibility.Visible;
        }
        // The core makes the parts of the body. The generated types compare as references,
        // so new parts of the same body would not be equal
        var wanted = (row.Item.Body, Zoom);
        if (drawn == wanted)
            return;
        drawn = wanted;
        shown.Children.Clear();
        runs.Clear();
        var size = FontSize * Zoom;
        // An item with no text is still as high as a line, and can be clicked
        paper.MinHeight = Math.Round(size * LineHeight);
        foreach (var part in ShownText.Of(row.BodyModel).Parts)
        {
            UIElement drawnPart = part switch
            {
                ShownParagraph paragraph => Block(paragraph, size),
                ShownTable table => Table(table, size),
                ShownPicture picture => Picture(picture, size),
            };
            shown.Children.Add(drawnPart);
        }
    }

    /// <summary>How far to the right a part stands that is <paramref name="indent"/> steps in.</summary>
    private Thickness Indented(int indent) => new(indent * 16 * Zoom, 0, 0, 0);

    /// <summary>
    /// A table with a line around each cell, as macOS draws a table. The columns share the
    /// width of the sticky, and the text of a cell wraps in it.
    /// </summary>
    private Grid Table(ShownTable table, double size)
    {
        var grid = new Grid { Margin = Indented(table.Indent) };
        var columns = table.Rows.Max(row => row.Cells.Count);
        for (var column = 0; column < columns; column++)
            grid.ColumnDefinitions.Add(new ColumnDefinition());
        for (var at = 0; at < table.Rows.Count; at++)
        {
            grid.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
            var cells = table.Rows[at].Cells;
            for (var column = 0; column < columns; column++)
            {
                var box = new Border
                {
                    BorderBrush = Faint,
                    BorderThickness = new Thickness(0.5),
                    Padding = new Thickness(4 * Zoom),
                };
                if (column < cells.Count)
                {
                    box.Child = Block(cells[column], size);
                }
                Grid.SetRow(box, at);
                Grid.SetColumn(box, column);
                grid.Children.Add(box);
            }
        }
        return grid;
    }

    /// <summary>
    /// An image of the assets folder, which the journal server serves, at the size that the
    /// page asks for. An image whose page asks for no size fits the width of the sticky, and
    /// is never larger than its file. Its words stand there until its file is here, and stay
    /// when the file cannot be had.
    /// </summary>
    private Grid Picture(ShownPicture picture, double size)
    {
        var holder = new Grid
        {
            Margin = Indented(picture.Indent),
            HorizontalAlignment = HorizontalAlignment.Left,
        };
        var words = new TextBlock
        {
            Text = picture.Words,
            FontFamily = TextFont,
            FontSize = size,
            Foreground = Ink,
            TextWrapping = TextWrapping.Wrap,
        };
        holder.Children.Add(words);
        if (Shell.AssetAddress(picture.Path) is not { } address)
            return holder;
        var file = new BitmapImage(address);
        var image = new Image
        {
            Source = file,
            Stretch =
                picture.Width is not null && picture.Height is not null
                    ? Stretch.Fill
                    : Stretch.Uniform,
            HorizontalAlignment = HorizontalAlignment.Left,
        };
        if (picture.Width is { } width)
            image.Width = width * Zoom;
        if (picture.Height is { } height)
            image.Height = height * Zoom;
        // The image has no size until its file is here: the words stand alone till then
        file.ImageOpened += (_, _) =>
        {
            if (picture.Width is null && picture.Height is null)
                image.MaxWidth = file.PixelWidth;
            words.Visibility = Visibility.Collapsed;
        };
        file.ImageFailed += (_, _) => image.Visibility = Visibility.Collapsed;
        holder.Children.Add(image);
        return holder;
    }

    private RichTextBlock Block(ShownParagraph paragraph, double size)
    {
        var block = new RichTextBlock
        {
            // A click puts the cursor in the item. To select its letters is to edit it
            IsTextSelectionEnabled = false,
            TextWrapping = TextWrapping.Wrap,
            FontFamily = TextFont,
            FontSize = size * paragraph.Scale,
            Foreground = Ink,
            Margin = Indented(paragraph.Indent),
        };
        var letters = new Paragraph();
        var place = 0;
        foreach (var piece in paragraph.Runs)
        {
            var run = Run(piece, size * paragraph.Scale);
            runs.Add((block, run, place));
            if (piece.Link is { } link)
            {
                var anchor = new Hyperlink { IsTabStop = false };
                anchor.Inlines.Add(run);
                anchor.Click += (_, _) =>
                {
                    followed = Environment.TickCount64;
                    Shell.Follow(link, Input.Held());
                };
                letters.Inlines.Add(anchor);
            }
            else
            {
                letters.Inlines.Add(run);
            }
            if (Paper(piece.Style) is { } behind)
            {
                var marker = new TextHighlighter { Background = behind, Foreground = Ink };
                marker.Ranges.Add(new TextRange { StartIndex = place, Length = piece.Text.Length });
                block.TextHighlighters.Add(marker);
            }
            place += piece.Text.Length;
        }
        block.Blocks.Add(letters);
        block.Tapped += (_, e) =>
        {
            e.Handled = true;
            var at = LetterOf(block, paragraph, e.GetPosition(block));
            // The click of a link comes on its own way, before this or after it. A click on
            // a link does not put the cursor in the item
            DispatcherQueue.TryEnqueue(() =>
            {
                if (Environment.TickCount64 - followed > 500)
                    Touch(at);
            });
        };
        return block;
    }

    private static Run Run(ShownRun piece, double size)
    {
        var style = piece.Style;
        // Raised and lowered letters are only smaller, and stay on the baseline. A Run has no
        // setting to move them, and an InlineUIContainer would break PlaceOf, links and
        // highlights. The README says more
        var run = new Run { Text = piece.Text, FontSize = size * piece.Scale };
        if (style.HasFlag(RunStyle.Bold))
            run.FontWeight = FontWeights.Bold;
        if (style.HasFlag(RunStyle.Italic))
            run.FontStyle = FontStyle.Italic;
        var lines = TextDecorations.None;
        if (style.HasFlag(RunStyle.Strike))
            lines |= TextDecorations.Strikethrough;
        if (style.HasFlag(RunStyle.Underline))
            lines |= TextDecorations.Underline;
        if (lines != TextDecorations.None)
            run.TextDecorations = lines;
        if (style.HasFlag(RunStyle.Code))
            run.FontFamily = CodeFont;
        if (style.HasFlag(RunStyle.Quote))
            run.Foreground = Quoted;
        return run;
    }

    /// <summary>The color behind a piece of text, when its style has one.</summary>
    private static SolidColorBrush? Paper(RunStyle style)
    {
        if (style.HasFlag(RunStyle.Spoiler))
            return Hidden;
        if (style.HasFlag(RunStyle.Highlight))
            return Marked;
        return style.HasFlag(RunStyle.Code) ? CodePaper : null;
    }

    /// <summary>
    /// How many letters of the shown text of the item stand before a click on the paragraph,
    /// as the core counts them. None when the paragraph has no letters of the core.
    /// </summary>
    private int? LetterOf(
        RichTextBlock block,
        ShownParagraph paragraph,
        Windows.Foundation.Point point
    )
    {
        var pointer = block.GetPositionFromPoint(point);
        var place = 0;
        foreach (var (owner, run, start) in runs)
        {
            if (owner != block || run.ContentStart.Offset > pointer.Offset)
                continue;
            place =
                start + Math.Clamp(pointer.Offset - run.ContentStart.Offset, 0, run.Text.Length);
        }
        return paragraph.LetterAt(place);
    }
}
