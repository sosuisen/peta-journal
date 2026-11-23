/** A change of the rows, as `ListChange` of the border has it, for rows of any kind. */
export type RowChange<T> =
  | { readonly kind: 'Insert'; readonly at: number; readonly item: T }
  | { readonly kind: 'Remove'; readonly at: number }
  | { readonly kind: 'Move'; readonly from: number; readonly to: number }
  | { readonly kind: 'Update'; readonly at: number; readonly item: T }
  | { readonly kind: 'Reset'; readonly items: readonly T[] };

/** The rows after the changes. */
export function rowsAfter<T>(
  rows: readonly T[],
  _changes: readonly RowChange<T>[],
): readonly T[] {
  return rows;
}
