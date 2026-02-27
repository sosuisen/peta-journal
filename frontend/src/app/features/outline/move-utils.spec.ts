import { describe, it, expect } from 'vitest';
import { Block } from '../../core/models/block.model';
import { findMoveDownTarget, findMoveUpTarget, getSubtree } from './move-utils';

/** Helper: create a minimal Block with only the fields the algorithm uses. */
function b(id: string, level: number): Block {
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
    html: '',
    collapsed: false,
    status: 'normal',
    types: [],
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
  // Same level: move after subtree
  // -------------------------------------------------------------------------
  describe('same level found → move after subtree', () => {
    it('swaps two adjacent siblings (no children)', () => {
      // * a (1)    ← move
      // * b (1)
      const blocks = [b('a', 1), b('b', 1)];
      expect(findMoveDownTarget(blocks, 'a'))
        .toEqual({ afterBlockId: 'b' });
    });

    it('moves after same-level block with deep subtree', () => {
      // * a (1)    ← move
      // * b (1)
      //     * c (2)
      //         * d (3)
      const blocks = [b('a', 1), b('b', 1), b('c', 2), b('d', 3)];
      expect(findMoveDownTarget(blocks, 'a'))
        .toEqual({ afterBlockId: 'd' });
    });

    it('moves child after its sibling under same parent', () => {
      // * a (1)
      //     * b (2)    ← move
      //     * c (2)
      const blocks = [b('a', 1), b('b', 2), b('c', 2)];
      expect(findMoveDownTarget(blocks, 'b'))
        .toEqual({ afterBlockId: 'c' });
    });

    it('moves child after sibling with children', () => {
      // * a (1)
      //     * b (2)    ← move
      //     * c (2)
      //         * d (3)
      //     * e (2)
      const blocks = [b('a', 1), b('b', 2), b('c', 2), b('d', 3), b('e', 2)];
      expect(findMoveDownTarget(blocks, 'b'))
        .toEqual({ afterBlockId: 'd' });
    });

    it('first of three siblings moves after second', () => {
      // * a (1)    ← move
      // * b (1)
      // * c (1)
      const blocks = [b('a', 1), b('b', 1), b('c', 1)];
      expect(findMoveDownTarget(blocks, 'a'))
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
      expect(findMoveDownTarget(blocks, 'c'))
        .toEqual({ afterBlockId: 'd' });
    });

    it('level-2 block crosses level-1 boundary', () => {
      // * a (1)
      //     * b (2)    ← move
      // * c (1)
      const blocks = [b('a', 1), b('b', 2), b('c', 1)];
      expect(findMoveDownTarget(blocks, 'b'))
        .toEqual({ afterBlockId: 'c' });
    });

  });

  // -------------------------------------------------------------------------
  // Movement not allowed
  // -------------------------------------------------------------------------
  describe('movement not allowed', () => {
    it('returns null when block is not found', () => {
      expect(findMoveDownTarget([b('a', 1)], 'x')).toBeNull();
    });

    it('returns null when block is the only one', () => {
      const blocks = [b('a', 1)];
      expect(findMoveDownTarget(blocks, 'a')).toBeNull();
    });

    it('returns null when only deeper blocks exist below', () => {
      // * a (1)
      //     * b (2)    ← move
      //         * c (3)
      const blocks = [b('a', 1), b('b', 2), b('c', 3)];
      expect(findMoveDownTarget(blocks, 'b')).toBeNull();
    });

    it('returns null for last same-level block', () => {
      // * a (1)
      // * b (1)
      // * c (1)    ← move
      const blocks = [b('a', 1), b('b', 1), b('c', 1)];
      expect(findMoveDownTarget(blocks, 'c')).toBeNull();
    });

    it('returns null when 2+ levels shallower is found', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)    ← move
      // * d (1)
      //
      // Scan: d(1) is 2+ levels shallower than c(3) → cannot move
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 1)];
      expect(findMoveDownTarget(blocks, 'c')).toBeNull();
    });

    it('returns null when 2+ levels shallower is found before one-level-shallower', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)    ← move
      // * d (1)
      //     * e (2)
      //         * f (3)
      //
      // Scan: d(1) is 2+ levels shallower than c(3) → cannot move
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 1), b('e', 2), b('f', 3)];
      expect(findMoveDownTarget(blocks, 'c')).toBeNull();
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
      expect(findMoveDownTarget(blocks, 'd'))
        .toEqual({ afterBlockId: 'e' });
    });

    it('level-4 sibling swap', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)
      //             * d (4)    ← move
      //             * e (4)
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 4), b('e', 4)];
      expect(findMoveDownTarget(blocks, 'd'))
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
      expect(findMoveDownTarget(blocks, 'b'))
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
      expect(findMoveDownTarget(blocks, 'b'))
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
      expect(findMoveDownTarget(blocks, 'b'))
        .toEqual({ afterBlockId: 'e' });
    });
  });
});

// ---------------------------------------------------------------------------
// findMoveUpTarget
// ---------------------------------------------------------------------------
describe('findMoveUpTarget', () => {
  // -------------------------------------------------------------------------
  // Condition 1: directly above is one-level-shallower
  // -------------------------------------------------------------------------
  describe('condition 1: directly above is one-level-shallower', () => {
    it('two-above is same-level → move under it', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)
      //     * d (2)
      //         * e (3)    ← move
      //
      // Directly above e: d(2) = blockLevel-1. Two above: c(3) is the same level as e(3).
      // → move under c → afterBlockId = 'c'
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 2), b('e', 3)];
      expect(findMoveUpTarget(blocks, 'e'))
        .toEqual({ afterBlockId: 'c' });
    });

    it('two-above is one-level-shallower → move under it', () => {
      // * a (1)
      // * b (1)
      // * c (1)
      //     * d (2)    ← move
      //
      // Directly above d: c(1) = blockLevel-1. Two above: b(1) = blockLevel-1.
      // → move under b → afterBlockId = 'b'
      const blocks = [b('a', 1), b('b', 1), b('c', 1), b('d', 2)];
      expect(findMoveUpTarget(blocks, 'd'))
        .toEqual({ afterBlockId: 'b' });
    });

    it('two-above is deeper → move under it', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)
      // * d (1)
      //     * e (2)    ← move
      //
      // Directly above e: d(1) = blockLevel-1. Two above: c(3), level 3 >= 1.
      // → move under c → afterBlockId = 'c'
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 1), b('e', 2)];
      expect(findMoveUpTarget(blocks, 'e'))
        .toEqual({ afterBlockId: 'c' });
    });

    it('two-above is too shallow → cannot move', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)
      //             * d (4)    ← move
      //
      // Directly above d: c(3) = blockLevel-1. Two above: b(2), level 2 < 3.
      // → cannot move
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 4)];
      expect(findMoveUpTarget(blocks, 'd')).toBeNull();
    });
  });

  // -------------------------------------------------------------------------
  // Condition 2: same-level found → move above it
  // -------------------------------------------------------------------------
  describe('condition 2: same-level found → move above it', () => {
    it('swaps two adjacent siblings', () => {
      // * a (1)
      // * b (1)    ← move
      const blocks = [b('a', 1), b('b', 1)];
      expect(findMoveUpTarget(blocks, 'b'))
        .toEqual({ afterBlockId: '' });
    });

    it('moves above same-level block that has children', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)
      // * d (1)    ← move
      //
      // Directly above d: c(3) ≠ blockLevel-1(0). Scan: c≠1, b≠1, a(1)=1.
      // → above a → afterBlockId = ''
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 1)];
      expect(findMoveUpTarget(blocks, 'd'))
        .toEqual({ afterBlockId: '' });
    });

    it('moves child above its sibling under same parent', () => {
      // * a (1)
      //     * b (2)
      //     * c (2)    ← move
      //
      // Directly above c: b(2) ≠ blockLevel-1(1). b(2) = blockLevel → condition 2.
      // → above b → afterBlockId = 'a'
      const blocks = [b('a', 1), b('b', 2), b('c', 2)];
      expect(findMoveUpTarget(blocks, 'c'))
        .toEqual({ afterBlockId: 'a' });
    });

    it('skips deeper blocks to find same-level', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)
      //     * d (2)    ← move
      //
      // Directly above d: c(3) ≠ blockLevel-1(1). Scan: c(3)≠2, b(2)=2.
      // → above b → afterBlockId = 'a'
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 2)];
      expect(findMoveUpTarget(blocks, 'd'))
        .toEqual({ afterBlockId: 'a' });
    });

    it('last of three siblings moves above second', () => {
      // * a (1)
      // * b (1)
      // * c (1)    ← move
      //
      // Directly above c: b(1) = blockLevel(1) → condition 2 (not condition 1).
      // → above b → afterBlockId = 'a'
      const blocks = [b('a', 1), b('b', 1), b('c', 1)];
      expect(findMoveUpTarget(blocks, 'c'))
        .toEqual({ afterBlockId: 'a' });
    });
  });

  // -------------------------------------------------------------------------
  // Movement not allowed
  // -------------------------------------------------------------------------
  describe('movement not allowed', () => {
    it('returns null when block is not found', () => {
      expect(findMoveUpTarget([b('a', 1)], 'x')).toBeNull();
    });

    it('returns null when block is the only one', () => {
      const blocks = [b('a', 1)];
      expect(findMoveUpTarget(blocks, 'a')).toBeNull();
    });

    it('returns null when no same-level block exists above', () => {
      // * a (1)
      //     * b (2)    ← move
      //
      // Directly above b: a(1) = blockLevel-1. But idx=1, no two-above.
      // → null
      const blocks = [b('a', 1), b('b', 2)];
      expect(findMoveUpTarget(blocks, 'b')).toBeNull();
    });

    it('returns null for first same-level block', () => {
      // * a (1)    ← move
      // * b (1)
      // * c (1)
      const blocks = [b('a', 1), b('b', 1), b('c', 1)];
      expect(findMoveUpTarget(blocks, 'a')).toBeNull();
    });
  });

  // -------------------------------------------------------------------------
  // Deeply nested
  // -------------------------------------------------------------------------
  describe('deeply nested', () => {
    it('level-4: condition 1 with shallower directly above', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)
      //             * d (4)
      //         * e (3)
      //             * f (4)    ← move
      //
      // Directly above f: e(3) = blockLevel-1. Two above: d(4), level 4 >= 3.
      // → under d → afterBlockId = 'd'
      const blocks = [
        b('a', 1), b('b', 2), b('c', 3), b('d', 4), b('e', 3), b('f', 4),
      ];
      expect(findMoveUpTarget(blocks, 'f'))
        .toEqual({ afterBlockId: 'd' });
    });

    it('level-4 sibling swap via condition 2', () => {
      // * a (1)
      //     * b (2)
      //         * c (3)
      //             * d (4)
      //             * e (4)    ← move
      //
      // Directly above e: d(4) ≠ blockLevel-1(3). d(4)=blockLevel → cond2.
      // → above d → afterBlockId = 'c'
      const blocks = [b('a', 1), b('b', 2), b('c', 3), b('d', 4), b('e', 4)];
      expect(findMoveUpTarget(blocks, 'e'))
        .toEqual({ afterBlockId: 'c' });
    });
  });

  // -------------------------------------------------------------------------
  // Move to beginning
  // -------------------------------------------------------------------------
  describe('move to beginning', () => {
    it('returns empty afterBlockId when same-level is first block', () => {
      // * a (1)
      // * b (1)    ← move
      //
      // Directly above b: a(1) = blockLevel → cond2.
      // → above a → afterBlockId = ''
      const blocks = [b('a', 1), b('b', 1)];
      expect(findMoveUpTarget(blocks, 'b'))
        .toEqual({ afterBlockId: '' });
    });
  });
});
