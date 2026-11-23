/** A change of the rows, as `ListChange` of the border has it, for rows of any kind. */
export type RowChange<T> =
  | { readonly kind: 'Insert'; readonly at: number; readonly item: T }
  | { readonly kind: 'Remove'; readonly at: number }
  | { readonly kind: 'Move'; readonly from: number; readonly to: number }
  | { readonly kind: 'Update'; readonly at: number; readonly item: T }
  | { readonly kind: 'Reset'; readonly items: readonly T[] };

/**
 * The rows after the changes, applied in order. A row that no change touches stays the same
 * value, so React does not draw it again.
 */
export function rowsAfter<T>(
  rows: readonly T[],
  changes: readonly RowChange<T>[],
): readonly T[] {
  if (changes.length === 0) {
    return rows;
  }
  const after = [...rows];
  for (const change of changes) {
    switch (change.kind) {
      case 'Insert':
        after.splice(change.at, 0, change.item);
        break;
      case 'Remove':
        after.splice(change.at, 1);
        break;
      case 'Move':
        after.splice(change.to, 0, ...after.splice(change.from, 1));
        break;
      case 'Update':
        after[change.at] = change.item;
        break;
      case 'Reset':
        after.splice(0, after.length, ...change.items);
        break;
    }
  }
  return after;
}
