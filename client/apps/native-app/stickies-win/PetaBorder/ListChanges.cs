using System.Collections.ObjectModel;
using PetaTypes;

namespace PetaBorder;

/// <summary>
/// A change of a list of any kind (ADR 088). The border has a type of its own for the changes
/// of each list, all of the same shape; each is made into this, and
/// <see cref="ListChanges.Apply{T, TItem}"/> applies them all.
/// </summary>
public abstract record Change<TItem>
{
    private Change() { }

    public sealed record Insert(int At, TItem Item) : Change<TItem>;

    public sealed record Remove(int At) : Change<TItem>;

    public sealed record Move(int From, int To) : Change<TItem>;

    public sealed record Update(int At, TItem Item) : Change<TItem>;

    public sealed record Reset(IReadOnlyList<TItem> Items) : Change<TItem>;
}

/// <summary>The changes to a list, as the core sends them (ADR 076).</summary>
public static class ListChanges
{
    /// <summary>
    /// Applies the changes of the rows of the outline to <paramref name="rows"/>, one after
    /// another, as <see cref="Apply{T, TItem}"/> does.
    /// </summary>
    public static void Apply<T>(
        ObservableCollection<T> rows,
        IEnumerable<RowChange> changes,
        Func<Row, T> make,
        Action<int, Row> update
    )
    {
        ArgumentNullException.ThrowIfNull(changes);
        Apply(rows, changes.Select(Of), make, update);
    }

    /// <summary>
    /// Applies the changes to <paramref name="list"/>, one after another. A moved item is
    /// moved, not taken out and put back: a list keeps the view of the item.
    /// </summary>
    /// <param name="make">What the list keeps for an item that comes.</param>
    /// <param name="update">Gives a new state of its item to what the list keeps at a place.</param>
    public static void Apply<T, TItem>(
        ObservableCollection<T> list,
        IEnumerable<Change<TItem>> changes,
        Func<TItem, T> make,
        Action<int, TItem> update
    )
    {
        ArgumentNullException.ThrowIfNull(list);
        ArgumentNullException.ThrowIfNull(changes);
        ArgumentNullException.ThrowIfNull(make);
        ArgumentNullException.ThrowIfNull(update);
        foreach (var change in changes)
        {
            switch (change)
            {
                case Change<TItem>.Insert insert:
                    list.Insert(insert.At, make(insert.Item));
                    break;
                case Change<TItem>.Remove remove:
                    list.RemoveAt(remove.At);
                    break;
                case Change<TItem>.Move move:
                    list.Move(move.From, move.To);
                    break;
                case Change<TItem>.Update changed:
                    update(changed.At, changed.Item);
                    break;
                case Change<TItem>.Reset reset:
                    list.Clear();
                    foreach (var item in reset.Items)
                        list.Add(make(item));
                    break;
            }
        }
    }

    /// <summary>A change of the headings of the page, as <see cref="Change{TItem}"/>.</summary>
    internal static Change<Heading> Of(HeadingChange change) =>
        change switch
        {
            HeadingChange.Insert insert => new Change<Heading>.Insert((int)insert.At, insert.Item),
            HeadingChange.Remove remove => new Change<Heading>.Remove((int)remove.At),
            HeadingChange.Move move => new Change<Heading>.Move((int)move.From, (int)move.To),
            HeadingChange.Update update => new Change<Heading>.Update((int)update.At, update.Item),
            HeadingChange.Reset reset => new Change<Heading>.Reset(reset.Items),
        };

    private static Change<Row> Of(RowChange change) =>
        change switch
        {
            RowChange.Insert insert => new Change<Row>.Insert((int)insert.At, insert.Item),
            RowChange.Remove remove => new Change<Row>.Remove((int)remove.At),
            RowChange.Move move => new Change<Row>.Move((int)move.From, (int)move.To),
            RowChange.Update update => new Change<Row>.Update((int)update.At, update.Item),
            RowChange.Reset reset => new Change<Row>.Reset(reset.Items),
        };
}
