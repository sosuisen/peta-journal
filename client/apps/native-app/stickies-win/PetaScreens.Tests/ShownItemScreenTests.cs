using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Documents;
using Microsoft.VisualStudio.TestTools.UnitTesting.AppContainer;
using Windows.System;

namespace PetaScreens.Tests;

/// <summary>
/// How a sticky draws an item that is not edited: its Markdown is shown as rich text. These
/// need a real journal server, so they are skipped without <c>PETA_E2E_SERVER</c>.
/// </summary>
[TestClass]
public sealed class ShownItemScreenTests
{
    private const int Port = 18116;

    // 表は、セルごとに枠のある格子で描く。
    [UITestMethod]
    public async Task A_table_is_drawn_as_a_grid_of_its_cells()
    {
        await using var screen = await Screen.OpenWritten(
            Port,
            "| H1 | H2 |\n|---|---|\n| a | b |"
        );

        var grid = Screen
            .Under<Grid>(screen.Items()[0])
            .Single(grid => grid.ColumnDefinitions.Count == 2);
        Assert.HasCount(2, grid.RowDefinitions);
        Assert.AreEqual(
            "H1|H2|a|b",
            string.Join("|", Screen.Under<RichTextBlock>(grid).Select(Letters))
        );
    }

    // 文字の飾りは、太字、斜体、取り消し線、下線、コードの字体で描く。
    [UITestMethod]
    public async Task The_styles_of_the_letters_are_drawn()
    {
        await using var screen = await Screen.OpenWritten(
            Port,
            "**bold** *italic* ~~struck~~ __under__ `code` ==marked== > quoted"
        );

        var runs = Runs(screen.Items()[0]);
        Assert.AreEqual(FontWeights.Bold, runs["bold"].FontWeight);
        Assert.AreEqual(Windows.UI.Text.FontStyle.Italic, runs["italic"].FontStyle);
        Assert.AreEqual(
            Windows.UI.Text.TextDecorations.Strikethrough,
            runs["struck"].TextDecorations
        );
        Assert.AreEqual(Windows.UI.Text.TextDecorations.Underline, runs["under"].TextDecorations);
        Assert.AreEqual("Consolas", runs["code"].FontFamily.Source);
        var block = Screen.Under<RichTextBlock>(screen.Items()[0]).Single();
        // The code and the marked letters have a color behind them
        Assert.HasCount(2, block.TextHighlighters);
    }

    // 貼った画像は、カーソルが項目を離れると、言葉の代わりに画像で描く。
    [UITestMethod]
    public async Task A_pasted_image_is_drawn_when_the_cursor_leaves_the_item()
    {
        await using var screen = await Screen.Open(Port);
        await screen.TwoItems();
        await Screen.PutImageOnClipboard();
        Screen.Press(VirtualKey.V, Held.Control);
        await Screen.Until("the second item links the image", () =>
            screen.Bodies()[1].Contains("](../assets/", StringComparison.Ordinal)
        );

        await screen.Edit(0);

        await Screen.Until("the image is drawn in place of its words", () =>
            Screen.Under<Image>(screen.Items()[1]).SingleOrDefault() is { ActualWidth: > 0 } image
            && image.Visibility == Visibility.Visible
            && Screen
                .Under<TextBlock>(screen.Items()[1])
                .All(words => words.Visibility == Visibility.Collapsed)
        );
    }

    // ファイルのない画像は、言葉だけを描く。
    [UITestMethod]
    public async Task An_image_without_its_file_shows_its_words()
    {
        await using var screen = await Screen.OpenWritten(Port, "![gone](../assets/none.png)");

        await Screen.Until("the image is hidden and the words stay", () =>
            Screen.Under<Image>(screen.Items()[0]).Single().Visibility == Visibility.Collapsed
        );
        Assert.AreEqual(
            "[gone]",
            Screen
                .Under<TextBlock>(screen.Items()[0])
                .Single(words => words.Visibility == Visibility.Visible)
                .Text
        );
    }

    // ジャーナルでは、前の日の項目の上に、その日付を見出しとして描く。
    [UITestMethod]
    public async Task A_journal_shows_the_day_above_the_items_of_a_day_before()
    {
        await using var screen = await Screen.Open(Port, url: null);
        screen.MakeJournal("2026-01-02", "a day before");

        screen.Shell.Open("/today");

        await Screen.Until("the day before is drawn with its date", () =>
            screen.Items().FirstOrDefault(item => Words(item) == "a day before") is { } item
            && Screen
                .Under<TextBlock>(item)
                .Any(header => header.Text.Contains("2026", StringComparison.Ordinal))
        );
    }

    /// <summary>The letters of the rich text under the element.</summary>
    private static string Words(DependencyObject element) =>
        string.Concat(
            Screen
                .Under<RichTextBlock>(element)
                .SelectMany(block => Inlines(block))
                .Select(run => run.Text)
        );

    /// <summary>The letters of the rich text block.</summary>
    private static string Letters(RichTextBlock block) =>
        string.Concat(Inlines(block).Select(run => run.Text));

    /// <summary>The runs of the item by their letters.</summary>
    private static Dictionary<string, Run> Runs(RowView item) =>
        Screen
            .Under<RichTextBlock>(item)
            .SelectMany(Inlines)
            .Where(run => run.Text.Trim().Length > 0)
            .ToDictionary(run => run.Text.Trim());

    private static IEnumerable<Run> Inlines(RichTextBlock block) =>
        block.Blocks.OfType<Paragraph>().SelectMany(paragraph => Flat(paragraph.Inlines));

    private static IEnumerable<Run> Flat(InlineCollection inlines) =>
        inlines.SelectMany(inline =>
            inline switch
            {
                Run run => [run],
                Span span => Flat(span.Inlines),
                _ => Enumerable.Empty<Run>(),
            }
        );
}
