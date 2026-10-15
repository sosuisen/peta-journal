import { describe, it, expect } from 'vitest';
import { Item } from '../../core/models/item.model';
import {
  canIndent,
  editContextOf,
  hasSiblingBelow,
  idsOfRangeWithSubtrees,
  lastDescendantOutside,
  levelForNewItemAfter,
  parentIndexOf,
  positionAfterSubtree,
  positionBefore,
  positionByIndex,
  selectedRootItems,
  selectionStartIndex,
  transclusionContextOf,
  transclusionScopedItems,
} from './outline-tree';

/** Minimal item; `extra` sets the fields a test cares about. */
function b(id: string, level: number, extra: Partial<Item> = {}): Item {
  return {
    id,
    pageId: 'page1',
    dirPath: '',
    name: '',
    type: 'doc',
    createdDate: 0,
    modifiedDate: 0,
    idx: '',
    level,
    body: '',
    ast: '',
    collapsed: false,
    types: [],
    properties: {},
    ...extra,
  };
}

const ids = (items: Item[]) => items.map(i => i.id);

// ---------------------------------------------------------------------------
// Placing new items
// ---------------------------------------------------------------------------
describe('hasSiblingBelow', () => {
  it('is true when a same-level item follows within the parent', () => {
    // * a (1)
    //     * b (2)
    //         * c (3)
    //     * d (2)
    const items = [b('a', 1), b('b', 2), b('c', 3), b('d', 2)];
    expect(hasSiblingBelow(items, 'b')).toBe(true);
  });

  it('stops at the end of the parent', () => {
    // * a (1)
    //     * b (2)
    // * c (1)
    //     * d (2)
    const items = [b('a', 1), b('b', 2), b('c', 1), b('d', 2)];
    expect(hasSiblingBelow(items, 'b')).toBe(false);
  });
});

describe('positionBefore', () => {
  it('is after the item above, or first for the top item', () => {
    const items = [b('a', 1), b('b', 1)];
    expect(positionBefore(items, 'b')).toBe('after:a');
    expect(positionBefore(items, 'a')).toBe('first');
  });
});

describe('positionAfterSubtree', () => {
  // * a (1, collapsed)
  //     * b (2)
  //         * c (3)
  // * d (1)
  const items = [b('a', 1, { collapsed: true }), b('b', 2), b('c', 3), b('d', 1)];

  it('skips the hidden subtree of a collapsed item', () => {
    expect(positionAfterSubtree(items, items[0], 'a')).toBe('after:c');
  });

  it('uses the anchor for an expanded item', () => {
    expect(positionAfterSubtree(items, items[3], 'source-of-d')).toBe('after:source-of-d');
  });
});

describe('levelForNewItemAfter', () => {
  it('goes one deeper when the item shows children', () => {
    const items = [b('a', 1), b('b', 2)];
    expect(levelForNewItemAfter(items, items[0])).toBe(2);
  });

  it('stays on the level of a collapsed parent or a childless item', () => {
    const items = [b('a', 1, { collapsed: true }), b('b', 2), b('c', 1)];
    expect(levelForNewItemAfter(items, items[0])).toBe(1);
    expect(levelForNewItemAfter(items, items[2])).toBe(1);
  });
});

describe('positionByIndex', () => {
  const items = [b('a', 1, { idx: 'z0' }), b('b', 1, { idx: 'z2' }), b('x', 1, { idx: 'z1', pageId: 'other' })];

  it('places the item after the last one of its page with a smaller index', () => {
    expect(positionByIndex(items, b('r', 1, { idx: 'z1' }))).toBe('after:a');
    expect(positionByIndex(items, b('r', 1, { idx: 'z3' }))).toBe('after:b');
  });

  it('is first when nothing on the page sorts before it', () => {
    expect(positionByIndex(items, b('r', 1, { idx: 'a0' }))).toBe('first');
  });
});

// ---------------------------------------------------------------------------
// Indent and outdent
// ---------------------------------------------------------------------------
describe('canIndent', () => {
  it('allows nesting under a sibling or a parent above', () => {
    // * a (1)
    // * b (1)        ← under a
    //     * c (2)    ← under b: becomes level 3, b is level 2? no: c's prev is b (level 1 = 3 - 2) → rejected
    const items = [b('a', 1), b('b', 1), b('c', 2)];
    expect(canIndent(items, 'b')).toBe(true);
    expect(canIndent(items, 'c')).toBe(false);
  });

  it('allows an indent when a same-level sibling exists above within the parent', () => {
    // * a (1)
    //     * b (2)
    //         * c (3)
    //     * d (2)    ← prev is c (3), but sibling b makes d nest under b
    const items = [b('a', 1), b('b', 2), b('c', 3), b('d', 2)];
    expect(canIndent(items, 'd')).toBe(true);
  });

  it('rejects the first item and an item with no sibling above', () => {
    // * a (1)
    //     * b (2)
    //         * c (3)
    //             * d (4)
    //     * e (2)  -- no: e is below; test f at level 2 right after a deep chain of another parent
    const items = [b('a', 1), b('b', 2), b('c', 3), b('d', 4), b('e', 1), b('f', 2)];
    expect(canIndent(items, 'a')).toBe(false);
    expect(canIndent(items, 'f')).toBe(false);
  });
});

describe('parentIndexOf', () => {
  // * a (1)
  //     * b (2)
  //         * c (3)
  //     * d (2)
  const items = [b('a', 1), b('b', 2), b('c', 3), b('d', 2)];

  it('finds the nearest item one level up', () => {
    expect(parentIndexOf(items, 3)).toBe(0);
    expect(parentIndexOf(items, 2)).toBe(1);
  });

  it('is -1 for a top-level item', () => {
    expect(parentIndexOf(items, 0)).toBe(-1);
  });
});

describe('lastDescendantOutside', () => {
  // * p (1)
  //     * a (2)
  //     * b (2)
  //         * c (3)
  //     * d (2)
  // * q (1)
  const items = [b('p', 1), b('a', 2), b('b', 2), b('c', 3), b('d', 2), b('q', 1)];

  it('skips the excluded group and returns the last remaining descendant', () => {
    expect(lastDescendantOutside(items, 'p', new Set(['d']))).toBe('c');
    expect(lastDescendantOutside(items, 'p', new Set(['b', 'c']))).toBe('d');
  });

  it('returns the parent itself when every descendant is excluded', () => {
    expect(lastDescendantOutside(items, 'p', new Set(['a', 'b', 'c', 'd']))).toBe('p');
  });
});

// ---------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------
describe('selectionStartIndex', () => {
  // * a (1)
  //     * b (2)
  //         * c (3)
  // * d (1)
  const items = [b('a', 1), b('b', 2), b('c', 3), b('d', 1)];

  it('starts at the lower index when both are on the anchor level', () => {
    expect(selectionStartIndex(items, 3, 0)).toBe(0);
  });

  it('climbs to the ancestor on the anchor level when the other end is nested deeper', () => {
    expect(selectionStartIndex(items, 3, 2)).toBe(0);
  });

  it('does not climb for a deeper anchor', () => {
    expect(selectionStartIndex(items, 2, 3)).toBe(2);
  });
});

describe('idsOfRangeWithSubtrees', () => {
  it('includes collapsed children that are not visible', () => {
    // own:     a (1, collapsed) > b (2), c (1)
    // visible: a, c
    const own = [b('a', 1, { collapsed: true }), b('b', 2), b('c', 1)];
    const visible = [own[0], own[2]];
    expect([...idsOfRangeWithSubtrees(visible, own, 0, 1)].sort()).toEqual(['a', 'b', 'c']);
  });

  it('keeps a transcluded descendant that is not among the own items', () => {
    const own = [b('a', 1)];
    const visible = [own[0], b('t', 2)];
    expect([...idsOfRangeWithSubtrees(visible, own, 1, 1)]).toEqual(['t']);
  });
});

describe('selectedRootItems', () => {
  // * a (1)
  //     * b (2)
  // * c (1)
  //     * d (2)
  const items = [b('a', 1), b('b', 2), b('c', 1), b('d', 2)];

  it('drops items whose parent is selected too', () => {
    expect(ids(selectedRootItems(items, new Set(['a', 'b', 'd'])))).toEqual(['a', 'd']);
  });
});

// ---------------------------------------------------------------------------
// Transclusion
// ---------------------------------------------------------------------------
describe('transclusion', () => {
  // own page: a (1), root (2, transcludes s), z (1)
  // source tree s: s (3, page src) > s1 (4)
  // visible:  a (1), root (2, shows s), s1 (3), z (1)
  const source = [b('s', 3, { pageId: 'src' }), b('s1', 4, { pageId: 'src' })];
  const own = [b('a', 1), b('root', 2, { transcludeItemId: 's' }), b('z', 1)];
  const trees = new Map([['s', source]]);
  const ownIds = new Set(ids(own));
  const shownS1 = b('s1', 3, { pageId: 'src' });
  const visible = [own[0], own[1], shownS1, own[2]];

  it('editContextOf: a root edits the source page with the level offset of the tree', () => {
    expect(editContextOf(own[1], ownIds, trees)).toEqual({ pageId: 'src', levelOffset: -1, inTransclusion: true });
  });

  it('editContextOf: a descendant keeps its page and gets the same offset', () => {
    expect(editContextOf(shownS1, ownIds, trees)).toEqual({ pageId: 'src', levelOffset: -1, inTransclusion: true });
  });

  it('editContextOf: an ordinary item has no offset', () => {
    expect(editContextOf(own[0], ownIds, trees)).toEqual({ pageId: 'page1', levelOffset: 0, inTransclusion: false });
  });

  it('editContextOf: a root whose source is missing still counts as a transclusion', () => {
    expect(editContextOf(own[1], ownIds, new Map())).toEqual({ pageId: 'page1', levelOffset: 0, inTransclusion: true });
  });

  it('transclusionContextOf: root and descendant resolve to the same root', () => {
    const expected = { rootItemId: 'root', rootDisplayLevel: 2, levelOffset: -1 };
    expect(transclusionContextOf(own[1], own, trees)).toEqual(expected);
    expect(transclusionContextOf(shownS1, own, trees)).toEqual(expected);
    expect(transclusionContextOf(own[0], own, trees)).toBeNull();
  });

  it('transclusionScopedItems: the visible descendants of the root, without the root', () => {
    const ctx = transclusionContextOf(own[1], own, trees)!;
    expect(ids(transclusionScopedItems(visible, ctx))).toEqual(['s1']);
  });
});
