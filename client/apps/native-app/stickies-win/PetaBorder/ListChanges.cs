using System.Collections.ObjectModel;
using PetaTypes;

namespace PetaBorder;

/// <summary>The changes to the rows of the outline, as the core sends them (ADR 076).</summary>
public static class ListChanges
{
    /// <summary>
    /// Applies the changes to <paramref name="rows"/>, one after another. A moved row is moved,
    /// not taken out and put back: a list keeps the view of the row.
    /// </summary>
    /// <param name="make">What the list keeps for a row that comes.</param>
    /// <param name="update">Gives a new state of its row to what the list keeps at a place.</param>
    public static void Apply<T>(
        ObservableCollection<T> rows,
        IEnumerable<ListChange> changes,
        Func<Row, T> make,
        Action<int, Row> update
    )
    {
        ArgumentNullException.ThrowIfNull(rows);
        ArgumentNullException.ThrowIfNull(changes);
        ArgumentNullException.ThrowIfNull(make);
        ArgumentNullException.ThrowIfNull(update);
        foreach (var change in changes)
        {
            switch (change)
            {
                case ListChange.Insert insert:
                    rows.Insert((int)insert.At, make(insert.Item));
                    break;
                case ListChange.Remove remove:
                    rows.RemoveAt((int)remove.At);
                    break;
                case ListChange.Move move:
                    rows.Move((int)move.From, (int)move.To);
                    break;
                case ListChange.Update changed:
                    update((int)changed.At, changed.Item);
                    break;
                case ListChange.Reset reset:
                    rows.Clear();
                    foreach (var row in reset.Items)
                        rows.Add(make(row));
                    break;
            }
        }
    }
}
