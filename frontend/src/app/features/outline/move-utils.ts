import { Block } from '../../core/models/block.model';

export interface MoveDownTarget {
  afterBlockId: string;
}

/**
 * Gets a block and all its descendants from a flat ordered block list.
 */
export function getSubtree(allBlocks: Block[], blockId: string): Block[] {
  const idx = allBlocks.findIndex(b => b.id === blockId);
  if (idx === -1) return [];
  const block = allBlocks[idx];
  const result = [block];
  for (let i = idx + 1; i < allBlocks.length; i++) {
    if (allBlocks[i].level <= block.level) break;
    result.push(allBlocks[i]);
  }
  return result;
}

/**
 * Determines the target position (afterBlockId) for moving a block down.
 *
 * Scans visible blocks below the moving block and finds the first item matching:
 * - Same level: move after that item's subtree.
 * - One level shallower (blockLevel - 1): move to just under that item.
 *
 * Collapsed (hidden) items are excluded from the scan.
 * When a parent moves, all its children move together.
 *
 * @param visibleBlocks - The currently visible blocks (collapsed children filtered out)
 * @param allBlocks - All blocks including collapsed children
 * @param blockId - The ID of the block to move
 * @returns The target position, or null if movement is not possible
 */
export function findMoveDownTarget(
  visibleBlocks: Block[],
  allBlocks: Block[],
  blockId: string,
): MoveDownTarget | null {
  const idx = visibleBlocks.findIndex(b => b.id === blockId);
  if (idx === -1) return null;

  const blockLevel = visibleBlocks[idx].level;

  for (let i = idx + 1; i < visibleBlocks.length; i++) {
    const candidate = visibleBlocks[i];

    if (candidate.level === blockLevel) {
      // Same level: move after candidate's subtree
      const subtree = getSubtree(allBlocks, candidate.id);
      if (subtree.length === 0) return null;
      return { afterBlockId: subtree[subtree.length - 1].id };
    }

    if (candidate.level === blockLevel - 1) {
      // One level shallower: move to just under candidate
      return { afterBlockId: candidate.id };
    }
  }

  return null;
}
