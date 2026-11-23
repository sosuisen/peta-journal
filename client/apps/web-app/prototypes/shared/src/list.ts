// Applies the changes of the rows to a list, in order, as the native shells do (ADR 076).

import type { ListChange, Row } from './types';

/** Applies `changes` to `rows` in place. A row that does not change stays the same object. */
export function applyInPlace(rows: Row[], changes: readonly ListChange[]): void {
  for (const change of changes) {
    if ('Insert' in change) {
      rows.splice(change.Insert.at, 0, change.Insert.item);
    } else if ('Remove' in change) {
      rows.splice(change.Remove.at, 1);
    } else if ('Move' in change) {
      const [moved] = rows.splice(change.Move.from, 1);
      if (moved !== undefined) {
        rows.splice(change.Move.to, 0, moved);
      }
    } else if ('Update' in change) {
      rows[change.Update.at] = change.Update.item;
    } else {
      rows.splice(0, rows.length, ...change.Reset.items);
    }
  }
}
