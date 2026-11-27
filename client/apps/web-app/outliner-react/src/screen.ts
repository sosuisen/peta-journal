/**
 * A change of a list of any kind, as each change type of the border has it: `RowChange` for
 * the rows of the outline and `HeadingChange` for the headings of the page, `PageChange` for the pages of the sidebar
 * (ADR 088). The types have the same shape, so one function applies
 * them all.
 */
export type ListChange<T> =
  | { readonly kind: 'Insert'; readonly at: number; readonly item: T }
  | { readonly kind: 'Remove'; readonly at: number }
  | { readonly kind: 'Move'; readonly from: number; readonly to: number }
  | { readonly kind: 'Update'; readonly at: number; readonly item: T }
  | { readonly kind: 'Reset'; readonly items: readonly T[] };

/**
 * The list after the changes, applied in order. An item that no change touches stays the
 * same value, so React does not draw it again.
 */
export function listAfter<T>(
  rows: readonly T[],
  changes: readonly ListChange<T>[],
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

/** A panel of the screen, as `Panel` of the border has it: its kind and its whole value. */
export interface PanelChange {
  readonly kind: string;
  readonly value: unknown;
}

/** The panels by kind. A panel that never came is missing. */
export type Panels<P extends PanelChange> = {
  readonly [K in P['kind']]?: Extract<P, { kind: K }>['value'];
};

/** The panels after the panels that came: each replaces the panel of its kind, whole. */
export function panelsAfter<P extends PanelChange>(
  panels: Panels<P>,
  changes: readonly P[],
): Panels<P> {
  if (changes.length === 0) {
    return panels;
  }
  const after: Record<string, unknown> = { ...panels };
  for (const change of changes) {
    after[change.kind] = change.value;
  }
  return after as Panels<P>;
}
