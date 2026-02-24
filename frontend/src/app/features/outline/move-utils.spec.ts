import { describe, it, expect } from 'vitest';
import { Block } from '../../core/models/block.model';
import { findMoveDownTarget, getSubtree } from './move-utils';

/** Helper: create a minimal Block with only the fields the algorithm uses. */
function b(id: string, level: number): Block {
  return {
    id,
    pageId: 'page1',
    dirPath: '',
    fileName: '',
    type: 'doc',
    createdDate: 0,
    modifiedDate: 0,
    idx: '',
    level,
    body: '',
    html: '',
    collapsed: false,
    status: 'normal',
    types: [],
    propertiesJson: '',
  };
}

// ---------------------------------------------------------------------------
// getSubtree
// ---------------------------------------------------------------------------
describe('getSubtree', () => {
  it('returns a single block when it has no children', () => {
    const blocks = [b('a', 1), b('b', 1)];
    expect(getSubtree(blocks, 'a')).toEqual([blocks[0]]);
  });

  it('returns block and all descendants', () => {
    const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 1)];
    expect(getSubtree(blocks, 'a').map(x => x.id)).toEqual(['a', 'b', 'c']);
  });

  it('returns empty array for unknown id', () => {
    expect(getSubtree([b('a', 1)], 'x')).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// findMoveDownTarget
// ---------------------------------------------------------------------------
describe('findMoveDownTarget', () => {
  // -------------------------------------------------------------------------
  // Spec examples
  // -------------------------------------------------------------------------
  describe('spec examples', () => {
    it('Example 1: same-level sibling found → move after its subtree', () => {
      // * a (1)
      //     * b (2)    ← move
      //     * c (2)
      //         * d (3)
      const blocks = [b('a', 1), b('b', 2), b('c', 2), b('d', 3)];
      expect(findMoveDownTarget(blocks, blocks, 'b'))
        .toEqual({ afterBlockId: 'd' });
    });

    it('Example 2: one-level-shallower found → move under it', () => {
      // * a (1)
      //     * b (2)    ← move
      // * c (1)
      //     * d (2)
      const blocks = [b('a', 1), b('b', 2), b('c', 1), b('d', 2)];
      expect(findMoveDownTarget(blocks, blocks, 'b'))
        .toEqual({ afterBlockId: 'c' });
    });

    it('Example 3: parent with children, shallower found → move under it', () => {
      // * a (1)
      //     * b (2)    ← move (with c, d)
      //         * c (3)
      //         * d (3)
      // * e (1)
      //     * f (2)
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 3), b('e', 1), b('f', 2)];
      expect(findMoveDownTarget(blocks, blocks, 'b'))
        .toEqual({ afterBlockId: 'e' });
    });
  });

  // -------------------------------------------------------------------------
  // Same level: move after subtree
  // -------------------------------------------------------------------------
  describe('same level found → move after subtree', () => {
    it('swaps two adjacent siblings (no children)', () => {
      // * a (1)    ← move
      // * b (1)
      const blocks = [b('a', 1), b('b', 1)];
      expect(findMoveDownTarget(blocks, blocks, 'a'))
        .toEqual({ afterBlockId: 'b' });
    });

    it('moves after same-level block with deep subtree', () => {
      // * a (1)    ← move
      // * b (1)
      //     * c (2)
      //         * d (3)
      const blocks = [b('a', 1), b('b', 1), b('c', 2), b('d', 3)];
      expect(findMoveDownTarget(blocks, blocks, 'a'))
        .toEqual({ afterBlockId: 'd' });
    });

    it('moves child after its sibling under same parent', () => {
      // * a (1)
      //     * b (2)    ← move
      //     * c (2)
      const blocks = [b('a', 1), b('b', 2), b('c', 2)];
      expect(findMoveDownTarget(blocks, blocks, 'b'))
        .toEqual({ afterBlockId: 'c' });
    });

    it('moves child after sibling with children', () => {
      // * a (1)
      //     * b (2)    ← move
      //     * c (2)
      //         * d (3)
      //     * e (2)
      const blocks = [b('a', 1), b('b', 2), b('c', 2), b('d', 3), b('e', 2)];
      expect(findMoveDownTarget(blocks, blocks, 'b'))
        .toEqual({ afterBlockId: 'd' });
    });

    it('first of three siblings moves after second', () => {
      // * a (1)    ← move
      // * b (1)
      // * c (1)
      const blocks = [b('a', 1), b('b', 1), b('c', 1)];
      expect(findMoveDownTarget(blocks, blocks, 'a'))
        .toEqual({ afterBlockId: 'b' });
    });
  });

  // -------------------------------------------------------------------------
  // One level shallower: move under it
  // -------------------------------------------------------------------------
  describe('one-level-shallower found → move under it', () => {
    it('level-3 block crosses level-2 boundary', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)    ← move
      //     * d (2)
      //         * e (3)
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 2), b('e', 3)];
      expect(findMoveDownTarget(blocks, blocks, 'c'))
        .toEqual({ afterBlockId: 'd' });
    });

    it('level-2 block crosses level-1 boundary', () => {
      // * a (1)
      //     * b (2)    ← move
      // * c (1)
      const blocks = [b('a', 1), b('b', 2), b('c', 1)];
      expect(findMoveDownTarget(blocks, blocks, 'b'))
        .toEqual({ afterBlockId: 'c' });
    });

    it('skips 2+ levels shallower, finds one-level-shallower', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)    ← move
      // * d (1)
      //     * e (2)
      //         * f (3)
      //
      // Scan: d(1) skip (3-1≠1), e(2) match (2=3-1)
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 1), b('e', 2), b('f', 3)];
      expect(findMoveDownTarget(blocks, blocks, 'c'))
        .toEqual({ afterBlockId: 'e' });
    });
  });

  // -------------------------------------------------------------------------
  // Movement not allowed
  // -------------------------------------------------------------------------
  describe('movement not allowed', () => {
    it('returns null when block is not found', () => {
      expect(findMoveDownTarget([b('a', 1)], [b('a', 1)], 'x')).toBeNull();
    });

    it('returns null when block is the only one', () => {
      const blocks = [b('a', 1)];
      expect(findMoveDownTarget(blocks, blocks, 'a')).toBeNull();
    });

    it('returns null when only deeper blocks exist below', () => {
      // * a (1)
      //     * b (2)    ← move
      //         * c (3)
      const blocks = [b('a', 1), b('b', 2), b('c', 3)];
      expect(findMoveDownTarget(blocks, blocks, 'b')).toBeNull();
    });

    it('returns null for last same-level block', () => {
      // * a (1)
      // * b (1)
      // * c (1)    ← move
      const blocks = [b('a', 1), b('b', 1), b('c', 1)];
      expect(findMoveDownTarget(blocks, blocks, 'c')).toBeNull();
    });

    it('returns null when gap is 2+ levels shallower with no match', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)    ← move
      // * d (1)
      //
      // Scan: d(1), 1≠3 and 1≠2 → skip. No more → null
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 1)];
      expect(findMoveDownTarget(blocks, blocks, 'c')).toBeNull();
    });
  });

  // -------------------------------------------------------------------------
  // Collapsed blocks (visibleBlocks ≠ allBlocks)
  // -------------------------------------------------------------------------
  describe('collapsed blocks', () => {
    it('collapsed parent swaps with sibling', () => {
      // allBlocks:  a(1), b(2), c(1)
      // a collapsed → visible: a(1), c(1)
      const all = [b('a', 1), b('b', 2), b('c', 1)];
      const visible = [all[0], all[2]]; // a, c
      expect(findMoveDownTarget(visible, all, 'a'))
        .toEqual({ afterBlockId: 'c' });
    });

    it('uses allBlocks for subtree of collapsed target', () => {
      // allBlocks:  a(1), b(1), c(2), d(1)
      // b collapsed → visible: a(1), b(1), d(1)
      //
      // Move a: b(1) same level → subtree of b in allBlocks = [b, c]
      const all = [b('a', 1), b('b', 1), b('c', 2), b('d', 1)];
      const visible = [all[0], all[1], all[3]]; // a, b, d
      expect(findMoveDownTarget(visible, all, 'a'))
        .toEqual({ afterBlockId: 'c' });
    });

    it('collapsed children are excluded from scan', () => {
      // allBlocks:  a(1), b(2), c(1), d(2), e(3)
      // c collapsed → visible: a(1), b(2), c(1)
      //
      // Move b: c(1) = blockLevel-1 → afterBlockId='c'
      const all = [b('a', 1), b('b', 2), b('c', 1), b('d', 2), b('e', 3)];
      const visible = [all[0], all[1], all[2]]; // a, b, c
      expect(findMoveDownTarget(visible, all, 'b'))
        .toEqual({ afterBlockId: 'c' });
    });
  });

  // -------------------------------------------------------------------------
  // Deeply nested
  // -------------------------------------------------------------------------
  describe('deeply nested', () => {
    it('level-4 block finds one-level-shallower across boundary', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)
      //             * d (4)    ← move
      //         * e (3)
      //             * f (4)
      //
      // Scan: e(3) = 4-1 → afterBlockId='e'
      const blocks = [
        b('a', 1), b('b', 2), b('c', 3), b('d', 4), b('e', 3), b('f', 4),
      ];
      expect(findMoveDownTarget(blocks, blocks, 'd'))
        .toEqual({ afterBlockId: 'e' });
    });

    it('level-4 sibling swap', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)
      //             * d (4)    ← move
      //             * e (4)
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 4), b('e', 4)];
      expect(findMoveDownTarget(blocks, blocks, 'd'))
        .toEqual({ afterBlockId: 'e' });
    });
  });

  // -------------------------------------------------------------------------
  // Scan order (first match wins)
  // -------------------------------------------------------------------------
  describe('scan order', () => {
    it('same-level found before shallower → same-level wins', () => {
      // * a (1)
      //     * b (2)    ← move
      //     * c (2)
      // * d (1)
      //
      // Scan: c(2) same level (first!) → afterBlockId='c'
      const blocks = [b('a', 1), b('b', 2), b('c', 2), b('d', 1)];
      expect(findMoveDownTarget(blocks, blocks, 'b'))
        .toEqual({ afterBlockId: 'c' });
    });

    it('shallower found before same-level → shallower wins', () => {
      // * a (1)
      //     * b (2)    ← move
      // * c (1)
      //     * d (2)
      //
      // Scan: c(1) = blockLevel-1 (first!) → afterBlockId='c'
      const blocks = [b('a', 1), b('b', 2), b('c', 1), b('d', 2)];
      expect(findMoveDownTarget(blocks, blocks, 'b'))
        .toEqual({ afterBlockId: 'c' });
    });

    it('deeper children are skipped until match is found', () => {
      // * a (1)
      //     * b (2)    ← move
      //         * c (3)
      //         * d (3)
      //     * e (2)
      //
      // Scan: c(3) skip, d(3) skip, e(2) same level → afterBlockId='e'
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 3), b('e', 2)];
      expect(findMoveDownTarget(blocks, blocks, 'b'))
        .toEqual({ afterBlockId: 'e' });
    });
  });
});
