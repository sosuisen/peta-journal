import { Block } from '../../core/models/block.model';

export interface MoveTarget {
  afterBlockId: string;
}

/**
 * Gets a block and all its descendants from a flat ordered block list.
 */
export function getSubtree(blocks: Block[], blockId: string): Block[] {
  const idx = blocks.findIndex(b => b.id === blockId);
  if (idx === -1) return [];
  const block = blocks[idx];
  const result = [block];
  for (let i = idx + 1; i < blocks.length; i++) {
    if (blocks[i].level <= block.level) break;
    result.push(blocks[i]);
  }
  return result;
}

/**
 * Determines the target position (afterBlockId) for moving a block down.
 *
 * Scans blocks below the moving block and finds the first item matching:
 * - 2+ levels shallower (< blockLevel - 1): movement not allowed.
 * - One level shallower (blockLevel - 1): move to just under that item.
 * - Same level: move after that item's subtree.
 *
 * When a parent moves, all its children move together.
 *
 * @param blocks - All blocks in order
 * @param blockId - The ID of the block to move
 * @returns The target position, or null if movement is not possible
 */
export function findMoveDownTarget(
  blocks: Block[],
  blockId: string,
): MoveTarget | null {
  const idx = blocks.findIndex(b => b.id === blockId);
  if (idx === -1) return null;

  const blockLevel = blocks[idx].level;

  for (let i = idx + 1; i < blocks.length; i++) {
    const candidate = blocks[i];

    if (candidate.level === blockLevel) {
      // Same level: move after candidate's subtree
      const subtree = getSubtree(blocks, candidate.id);
      if (subtree.length === 0) return null;
      return { afterBlockId: subtree[subtree.length - 1].id };
    }

    if (candidate.level === blockLevel - 1) {
      // One level shallower: move to just under candidate
      return { afterBlockId: candidate.id };
    }

    if (candidate.level < blockLevel - 1) {
      // 2+ levels shallower: movement not allowed
      return null;
    }
  }

  return null;
}

/**
 * Determines the target position (afterBlockId) for moving a block up.
 *
 * Two conditions checked in order:
 * 1. If the item directly above is one-level-shallower, check one more above.
 *    If that item's level >= blockLevel - 1 (shallower by at most 1, same, or deeper),
 *    move under it. Otherwise movement is not allowed.
 * 2. Scan upward for a same-level item. Move above it.
 *
 * When a parent moves, all its children move together.
 *
 * @param blocks - All blocks in order
 * @param blockId - The ID of the block to move
 * @returns The target position, or null if movement is not possible
 */
export function findMoveUpTarget(
  blocks: Block[],
  blockId: string,
): MoveTarget | null {
  const idx = blocks.findIndex(b => b.id === blockId);
  if (idx === -1) return null;

  const blockLevel = blocks[idx].level;

  // Condition 1: If the item directly above is one-level-shallower,
  // check the item above that. Move under it only if its level is
  // >= blockLevel - 1 (one-level-shallower, same level, or deeper).
  if (idx >= 1 && blocks[idx - 1].level === blockLevel - 1) {
    if (idx >= 2) {
      const twoAbove = blocks[idx - 2];
      if (twoAbove.level >= blockLevel - 1) {
        return { afterBlockId: twoAbove.id };
      }
    }
    return null;
  }

  // Condition 2: Scan upward for a same-level item. Move above it.
  for (let i = idx - 1; i >= 0; i--) {
    if (blocks[i].level === blockLevel) {
      if (i <= 0) return { afterBlockId: '' };
      return { afterBlockId: blocks[i - 1].id };
    }
  }

  return null;
}
