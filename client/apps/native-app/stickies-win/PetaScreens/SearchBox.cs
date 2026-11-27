using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using PetaBorder;
using PetaTypes;
using Windows.System;
using Windows.UI;
using Border = Microsoft.UI.Xaml.Controls.Border;

namespace PetaScreens;

/// <summary>The search box with what it found under it. An entry that is chosen opens its
/// page.</summary>
internal sealed class SearchBox : StackPanel
{
    private static readonly SolidColorBrush Ink = new(Color.FromArgb(255, 0x1F, 0x1F, 0x1F));
    private static readonly SolidColorBrush Pale = new(Color.FromArgb(255, 0x6E, 0x6E, 0x6E));
    private static readonly SolidColorBrush Clear = new(Colors.Transparent);
    private static readonly SolidColorBrush Chosen = new(Color.FromArgb(0x40, 0x00, 0x78, 0xD4));

    private readonly Shell shell;
    private readonly TextBox field = new() { PlaceholderText = "Search pages" };
    private readonly StackPanel entries = new() { Spacing = 2 };
    private readonly ScrollViewer scroll;
    private readonly TextBlock nothing = new()
    {
        Text = "Nothing found",
        FontFamily = RowView.TextFont,
        FontSize = 12,
        Foreground = Pale,
    };

    /// <summary>The input method is composing text in the field.</summary>
    private bool composing;

    /// <param name="onClose">Escape was pressed with nothing left to hide.</param>
    public SearchBox(Shell shell, Action onClose)
    {
        this.shell = shell;
        Spacing = 4;
        // More entries than this are scrolled: they would push the items out of the window
        scroll = new ScrollViewer { Content = entries, MaxHeight = 260 };
        field.TextChanging += (_, _) => Typed();
        field.TextCompositionStarted += (_, _) => composing = true;
        field.TextCompositionEnded += (_, _) =>
        {
            composing = false;
            Typed();
        };
        field.PreviewKeyDown += (_, e) =>
        {
            if (composing || e.Key == Input.InputMethod || KeyOf(e.Key) is not { } key)
                return;
            e.Handled = true;
            if (key == SearchKey.Escape && !shell.View.Search.ShowDropdown)
                onClose();
            else
                shell.Search(key);
        };
        Children.Add(field);
        Children.Add(scroll);
        Children.Add(nothing);
    }

    private static SearchKey? KeyOf(VirtualKey key) =>
        key switch
        {
            VirtualKey.Down => SearchKey.Down,
            VirtualKey.Up => SearchKey.Up,
            VirtualKey.Enter => SearchKey.Enter,
            VirtualKey.Escape => SearchKey.Escape,
            _ => null,
        };

    private void Typed()
    {
        // The core put this text here itself
        if (!composing && field.Text != shell.View.Search.Query)
            shell.Search(field.Text);
    }

    /// <summary>The box was opened to type into it.</summary>
    public void TakeCursor(int tries = 10)
    {
        // The field is not laid out yet when it was just made or shown
        DispatcherQueue.TryEnqueue(() =>
        {
            if (!field.Focus(FocusState.Programmatic) && tries > 0)
                TakeCursor(tries - 1);
        });
    }

    public void Draw(SearchView search)
    {
        // The core empties the query when a page was chosen. Text that is still being
        // composed is not in the core yet
        if (!composing && field.Text != search.Query)
            field.Text = search.Query;
        entries.Children.Clear();
        scroll.Visibility = search.ShowDropdown ? Visibility.Visible : Visibility.Collapsed;
        nothing.Visibility =
            search.ShowDropdown
            && search.Results.Count == 0
            && !search.Searching
            && search.Query.Length > 0
                ? Visibility.Visible
                : Visibility.Collapsed;
        if (!search.ShowDropdown)
            return;
        // The core counts an entry by its place in the results
        for (var at = 0; at < search.Results.Count; at++)
        {
            var chosen = search.SelectedIndex == (ulong)at;
            var row = Entry(search.Results[at], (ulong)at, chosen);
            entries.Children.Add(row);
            // The arrow keys may choose an entry that is out of sight
            if (chosen)
                row.Loaded += (_, _) => row.StartBringIntoView();
        }
    }

    private Border Entry(SearchEntry entry, ulong index, bool chosen)
    {
        var lines = new StackPanel();
        lines.Children.Add(
            new TextBlock
            {
                Text = entry.Label,
                FontFamily = RowView.TextFont,
                Foreground = Ink,
                TextTrimming = TextTrimming.CharacterEllipsis,
            }
        );
        // The snippet of a page found by its name is the name again
        if (entry.Snippet.Length > 0 && entry.Snippet != entry.Title)
        {
            lines.Children.Add(
                new TextBlock
                {
                    Text = entry.Snippet,
                    FontFamily = RowView.TextFont,
                    FontSize = 12,
                    Foreground = Pale,
                    TextTrimming = TextTrimming.CharacterEllipsis,
                    MaxLines = 1,
                }
            );
        }
        var row = new Border
        {
            Child = lines,
            Padding = new Thickness(4, 2, 4, 2),
            CornerRadius = new CornerRadius(3),
            Background = chosen ? Chosen : Clear,
        };
        row.Tapped += (_, e) =>
        {
            e.Handled = true;
            shell.Pick(index);
        };
        return row;
    }
}
