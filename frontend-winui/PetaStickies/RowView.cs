using Microsoft.UI;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Documents;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using PetaBorder;
using PetaTypes;
using Windows.UI;
using Windows.UI.Text;
using Border = Microsoft.UI.Xaml.Controls.Border;

namespace PetaStickies;

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
    private (string Ast, string Body, double Zoom)? drawn;

    /// <summary>The runs on the paper, each with the place of its first letter in the text.</summary>
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
        editor.TextChanging += (_, _) => Typed();
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
    private double Zoom => window.Look.Zoom;

    /// <summary>The room above an item and below it: half of the room between two items.</summary>
    private double HalfGap => Math.Round(RowGap / 2 * Zoom);

    /// <summary>The Markdown in the editor, with the line breaks of the core.</summary>
    private string Markdown => editor.Text.Replace("\r\n", "\n").Replace('\r', '\n');

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
            Shell.FinishEditing(left.Id);
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
        marks.Children.Add(Bullet(row.IsCollapsed, side, Math.Max(line, first)));
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
            Shell.Typed(body, row.Id);
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
        if (e.Key == Windows.System.VirtualKey.Y && held == Held.Control)
        {
            // The history is the one of the core: the text box must not bring back its own
            e.Handled = true;
            return;
        }
        if (Input.KeyOf(e.Key) is not { } key || Keyboard.InEditor(key, held) is not { } pressed)
            return;
        var (first, last) = CursorLines();
        var state = new EditorState(
            Markdown,
            editor.SelectionStart,
            editor.SelectionLength,
            first,
            last
        );
        // Fold and unfold are taken also when there is nothing to fold: the text box would
        // put the cursor at the start or the end of a paragraph otherwise
        e.Handled =
            Shell.Pressed(pressed, row.Id, state) || pressed is EditorKey.Fold or EditorKey.Unfold;
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

    /// <summary>The user clicked the text of the item, near the letter at this place.</summary>
    private void Edit(int place)
    {
        if (!TakeCursor(place))
            Draw();
    }

    // ---- The text of the item with its decoration ----

    private void DrawShown(Row row)
    {
        var item = row.Item;
        if (editor.Visibility == Visibility.Visible)
        {
            editor.Visibility = Visibility.Collapsed;
            paper.Visibility = Visibility.Visible;
        }
        var wanted = (item.Ast, item.Body, Zoom);
        if (drawn == wanted)
            return;
        drawn = wanted;
        shown.Children.Clear();
        runs.Clear();
        var size = FontSize * Zoom;
        // An item with no text is still as high as a line, and can be clicked
        paper.MinHeight = Math.Round(size * LineHeight);
        var start = 0;
        foreach (var paragraph in ShownText.Of(item).Paragraphs)
        {
            shown.Children.Add(Block(paragraph, size, start));
            start += paragraph.Text.Length + 1;
        }
    }

    private RichTextBlock Block(ShownParagraph paragraph, double size, int start)
    {
        var block = new RichTextBlock
        {
            // A click puts the cursor in the item. To select its letters is to edit it
            IsTextSelectionEnabled = false,
            TextWrapping = TextWrapping.Wrap,
            FontFamily = TextFont,
            FontSize = size * paragraph.Scale,
            Foreground = Ink,
            Margin = new Thickness(paragraph.Indent * 16 * Zoom, 0, 0, 0),
        };
        var letters = new Paragraph();
        var place = 0;
        foreach (var piece in paragraph.Runs)
        {
            var run = Run(piece, size * paragraph.Scale);
            runs.Add((block, run, start + place));
            if (piece.Link is { } link)
            {
                var anchor = new Hyperlink { IsTabStop = false };
                anchor.Inlines.Add(run);
                anchor.Click += (_, _) =>
                {
                    followed = Environment.TickCount64;
                    Shell.Follow(link);
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
            var at = PlaceOf(block, e.GetPosition(block));
            // The click of a link comes on its own way, before this or after it. A click on
            // a link does not put the cursor in the item
            DispatcherQueue.TryEnqueue(() =>
            {
                if (Environment.TickCount64 - followed > 500)
                    Edit(at);
            });
        };
        return block;
    }

    private static Run Run(ShownRun piece, double size)
    {
        var style = piece.Style;
        var run = new Run { Text = piece.Text };
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
        if (style.HasFlag(RunStyle.Small))
        {
            run.FontSize = size * 0.85;
            run.Foreground = Pale;
        }
        if (style.HasFlag(RunStyle.Superscript))
            Typography.SetVariants(run, FontVariants.Superscript);
        if (style.HasFlag(RunStyle.Subscript))
            Typography.SetVariants(run, FontVariants.Subscript);
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
    /// How far into the shown text a click was. In the Markdown that is not the same place,
    /// but one near it.
    /// </summary>
    private int PlaceOf(RichTextBlock block, Windows.Foundation.Point point)
    {
        var pointer = block.GetPositionFromPoint(point);
        var place = int.MaxValue;
        foreach (var (owner, run, start) in runs)
        {
            if (owner != block || run.ContentStart.Offset > pointer.Offset)
                continue;
            place =
                start + Math.Clamp(pointer.Offset - run.ContentStart.Offset, 0, run.Text.Length);
        }
        return place;
    }
}
