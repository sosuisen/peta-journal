import { Item } from '../../core/models/item.model';
import { ItemPosition } from '../../core/services/api.service';

export interface MoveTarget {
  position: ItemPosition;
}

/**
 * Gets a item and all its descendants from a flat ordered item list.
 */
export function getSubtree(items: Item[], itemId: string): Item[] {
  const idx = items.findIndex(b => b.id === itemId);
  if (idx === -1) return [];
  const item = items[idx];
  const result = [item];
  for (let i = idx + 1; i < items.length; i++) {
    if (items[i].level <= item.level) break;
    result.push(items[i]);
  }
  return result;
}

/**
 * Determines the target position for moving a item down.
 *
 * Scans items below the moving item and finds the first item matching:
 * - 2+ levels shallower (< itemLevel - 1): movement not allowed.
 * - One level shallower (itemLevel - 1): move to just under that item.
 * - Same level: move after that item's subtree.
 *
 * When a parent moves, all its children move together.
 *
 * @param items - All items in order
 * @param itemId - The ID of the item to move
 * @returns The target position, or null if movement is not possible
 */
export function findMoveDownTarget(
  items: Item[],
  itemId: string,
): MoveTarget | null {
  const idx = items.findIndex(b => b.id === itemId);
  if (idx === -1) return null;

  const itemLevel = items[idx].level;

  for (let i = idx + 1; i < items.length; i++) {
    const candidate = items[i];

    if (candidate.level === itemLevel) {
      // Same level: move after candidate's subtree
      const subtree = getSubtree(items, candidate.id);
      if (subtree.length === 0) return null;
      return { position: `after:${subtree[subtree.length - 1].id}` };
    }

    if (candidate.level === itemLevel - 1) {
      // One level shallower: move to just under candidate
      return { position: `after:${candidate.id}` };
    }

    if (candidate.level < itemLevel - 1) {
      // 2+ levels shallower: movement not allowed
      return null;
    }
  }

  return null;
}

/**
 * Determines the target position for moving a item up.
 *
 * Two conditions checked in order:
 * 1. If the item directly above is one-level-shallower, check one more above.
 *    If that item's level >= itemLevel - 1 (shallower by at most 1, same, or deeper),
 *    move under it. Otherwise movement is not allowed.
 * 2. Scan upward for a same-level item. Move above it.
 *
 * When a parent moves, all its children move together.
 *
 * @param items - All items in order
 * @param itemId - The ID of the item to move
 * @returns The target position, or null if movement is not possible
 */
export function findMoveUpTarget(
  items: Item[],
  itemId: string,
): MoveTarget | null {
  const idx = items.findIndex(b => b.id === itemId);
  if (idx === -1) return null;

  const itemLevel = items[idx].level;

  // Condition 1: If the item directly above is one-level-shallower,
  // check the item above that. Move under it only if its level is
  // >= itemLevel - 1 (one-level-shallower, same level, or deeper).
  if (idx >= 1 && items[idx - 1].level === itemLevel - 1) {
    if (idx >= 2) {
      const twoAbove = items[idx - 2];
      if (twoAbove.level >= itemLevel - 1) {
        return { position: `after:${twoAbove.id}` };
      }
    }
    return null;
  }

  // Condition 2: Scan upward for a same-level item. Move above it.
  for (let i = idx - 1; i >= 0; i--) {
    if (items[i].level === itemLevel) {
      if (i <= 0) return { position: 'first' };
      return { position: `after:${items[i - 1].id}` };
    }
  }

  return null;
}

/**
 * Finds a valid placement position for an item with the given level
 * in a target page's item list.
 *
 * For 'from-end': scans from the last item backwards.
 * For 'from-start': scans from the first position forwards.
 *
 * The placement rule is: level <= aboveItem.level + 1.
 *
 * @param targetItems - Items on the target page in order
 * @param level - The level of the item to place
 * @param direction - Scan direction
 * @returns The position where the item can be placed, or null if not possible
 */
export function findCrossPagePlacement(
  targetItems: Item[],
  level: number,
  direction: 'from-end' | 'from-start',
): ItemPosition | null {
  if (direction === 'from-end') {
    for (let i = targetItems.length - 1; i >= 0; i--) {
      if (level <= targetItems[i].level + 1) {
        return `after:${targetItems[i].id}`;
      }
    }
    return level === 1 ? 'first' : null;
  } else {
    if (level === 1) return 'first';
    for (let i = 0; i < targetItems.length; i++) {
      if (level <= targetItems[i].level + 1) {
        return `after:${targetItems[i].id}`;
      }
    }
    return null;
  }
}
