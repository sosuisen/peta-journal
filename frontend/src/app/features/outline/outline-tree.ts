import { Item } from '../../core/models/item.model';
import { ItemPosition } from '../../core/services/api.service';
import { getSubtree } from './move-utils';

/**
 * Pure functions over the outline tree: a flat list of items whose nesting is given by
 * `level`. Nothing here touches state or the server, so every rule is a plain function
 * of the lists it is given.
 *
 * Two lists appear throughout:
 * - the own items: the items of the pages shown in the outline, in order;
 * - the visible items: the own items plus transcluded descendants, minus collapsed subtrees.
 */

/**
 * Where an edit of an item shown in the outline is really stored.
 * A transclusion root shows the body of its source item, and a transcluded
 * descendant belongs to the source page, so both are saved there and the
 * levels they show are offset from the levels stored.
 */
export interface EditContext {
  /** The page new items are created on. */
  pageId: string;
  /** Shown level minus stored level; 0 outside a transclusion. */
  levelOffset: number;
  /** True for a transclusion root or a transcluded descendant. */
  inTransclusion: boolean;
}

/** The transclusion root an item is shown under, with the level offset of that tree. */
export interface TransclusionContext {
  rootItemId: string;
  rootDisplayLevel: number;
  levelOffset: number;
}

// ---- Placing new items ----

/** True when a sibling at the same level follows the item within the same parent. */
export function hasSiblingBelow(items: Item[], itemId: string): boolean {
  const idx = items.findIndex(b => b.id === itemId);
  if (idx < 0) return false;
  const level = items[idx].level;
  for (let i = idx + 1; i < items.length; i++) {
    if (items[i].level < level) break;
    if (items[i].level === level) return true;
  }
  return false;
}

/** The position right before an item: after the item above it, or first. */
export function positionBefore(items: Item[], itemId: string): ItemPosition {
  const idx = items.findIndex(b => b.id === itemId);
  return idx > 0 ? `after:${items[idx - 1].id}` : 'first';
}

/**
 * The position after an item. A collapsed item hides its subtree, so the position is
 * after its last descendant; otherwise it is after the given anchor, normally the item itself.
 * @param items - The own items, which hold the descendants of a collapsed item
 * @param item - The item to place after
 * @param anchorId - The id to use when the item is expanded (its stored id for a transclusion root)
 */
export function positionAfterSubtree(items: Item[], item: Item, anchorId: string): ItemPosition {
  const descendants = item.collapsed ? getSubtree(items, item.id).slice(1) : [];
  const afterId = descendants.length > 0 ? descendants[descendants.length - 1].id : anchorId;
  return `after:${afterId}`;
}

/**
 * The shown level of an item created right after the given one: one deeper when the
 * item is expanded and already has children, otherwise the same level.
 * @param items - The list the item's next neighbour is read from
 */
export function levelForNewItemAfter(items: Item[], item: Item): number {
  const idx = items.findIndex(b => b.id === item.id);
  const next = idx >= 0 ? items[idx + 1] : undefined;
  return !item.collapsed && next && next.level === item.level + 1 ? item.level + 1 : item.level;
}

/**
 * Finds the position an item restored by undo goes back to: after the last item of its
 * page whose fractional index sorts before the restored item's index.
 */
export function positionByIndex(items: Item[], item: Item): ItemPosition {
  let bestId = '';
  for (const b of items) {
    if (b.pageId === item.pageId && b.idx < item.idx) {
      bestId = b.id;
    }
  }
  return bestId ? `after:${bestId}` : 'first';
}

// ---- Indent and outdent ----

/**
 * Whether an item at the given level can follow prevItem.
 * The item above must be a sibling (same level) or the parent (one level up);
 * with nothing above, only level 1 fits.
 */
export function canPlace(itemLevel: number, prevItem: Item | undefined): boolean {
  if (!prevItem) return itemLevel === 1;
  return prevItem.level === itemLevel || prevItem.level === itemLevel - 1;
}

/**
 * Whether an item can be indented by one level: the item above must accept the new level,
 * or a sibling at the item's current level must exist above it within the same parent.
 */
export function canIndent(items: Item[], itemId: string): boolean {
  const idx = items.findIndex(b => b.id === itemId);
  if (idx <= 0) return false;
  const item = items[idx];
  if (canPlace(item.level + 1, items[idx - 1])) return true;
  // Exception: a sibling above at the same level makes the indent a nesting under that sibling
  for (let i = idx - 1; i >= 0; i--) {
    if (items[i].level < item.level) break; // exited current subtree
    if (items[i].level === item.level) return true;
  }
  return false;
}

/** The index of the parent of the item at idx: the nearest item above it one level up, or -1. */
export function parentIndexOf(items: Item[], idx: number): number {
  const level = items[idx].level;
  for (let i = idx - 1; i >= 0; i--) {
    if (items[i].level === level - 1) return i;
  }
  return -1;
}

/**
 * The id of the last descendant of a parent that is not among the excluded ids,
 * or the parent's own id when there is none. Used to place an outdented group
 * after the parent's remaining children.
 */
export function lastDescendantOutside(items: Item[], parentId: string, excludedIds: Set<string>): string {
  let lastId = parentId;
  for (const child of getSubtree(items, parentId).slice(1)) {
    if (!excludedIds.has(child.id)) {
      lastId = child.id;
    }
  }
  return lastId;
}

// ---- Selection ----

/**
 * The first index of a selection between the anchor and another item. When the
 * selection reaches into a subtree nested deeper than the anchor, it starts at the
 * ancestor on the anchor's level so that whole subtrees are selected.
 */
export function selectionStartIndex(items: Item[], anchorIdx: number, otherIdx: number): number {
  let start = Math.min(anchorIdx, otherIdx);
  const anchorLevel = items[anchorIdx].level;
  while (start > 0 && items[start].level > anchorLevel) {
    start--;
  }
  return start;
}

/**
 * The ids of the visible items from start to end (inclusive) together with their whole
 * subtrees, read from the own items so that collapsed children are included too.
 */
export function idsOfRangeWithSubtrees(visibleItems: Item[], ownItems: Item[], start: number, end: number): Set<string> {
  const ids = new Set<string>();
  for (let i = start; i <= end; i++) {
    ids.add(visibleItems[i].id);
    for (const b of getSubtree(ownItems, visibleItems[i].id)) {
      ids.add(b.id);
    }
  }
  return ids;
}

/** The selected items whose parent is not selected, in outline order. */
export function selectedRootItems(items: Item[], selectedIds: Set<string>): Item[] {
  const roots: Item[] = [];
  for (let idx = 0; idx < items.length; idx++) {
    const item = items[idx];
    if (!selectedIds.has(item.id)) continue;
    let parentSelected = false;
    for (let i = idx - 1; i >= 0; i--) {
      if (items[i].level < item.level) {
        parentSelected = selectedIds.has(items[i].id);
        break;
      }
    }
    if (!parentSelected) roots.push(item);
  }
  return roots;
}

// ---- Transclusion ----

/**
 * Resolves the page, the level offset and the transclusion state for editing an item.
 * @param item - The item as shown
 * @param ownIds - The ids of the own items
 * @param trees - Source item id to the source item and its descendants
 */
export function editContextOf(item: Item, ownIds: Set<string>, trees: Map<string, Item[]>): EditContext {
  const isOwn = ownIds.has(item.id);
  if (isOwn && item.transcludeItemId) {
    // Transclusion root: edits go to the source page
    const tree = trees.get(item.transcludeItemId);
    if (tree && tree.length > 0) {
      return { pageId: tree[0].pageId, levelOffset: item.level - tree[0].level, inTransclusion: true };
    }
    return { pageId: item.pageId, levelOffset: 0, inTransclusion: true };
  }
  if (!isOwn) {
    // Transcluded descendant: it already belongs to the source page
    let levelOffset = 0;
    for (const tree of trees.values()) {
      const found = tree.find(t => t.id === item.id);
      if (found) {
        levelOffset = item.level - found.level;
        break;
      }
    }
    return { pageId: item.pageId, levelOffset, inTransclusion: true };
  }
  return { pageId: item.pageId, levelOffset: 0, inTransclusion: false };
}

/**
 * The transclusion root an item belongs to, or null outside a transclusion.
 * A transclusion root is its own context; a transcluded descendant gets the context
 * of the root that shows it.
 * @param item - The item as shown
 * @param ownItems - The own items, where the roots live
 * @param trees - Source item id to the source item and its descendants
 */
export function transclusionContextOf(item: Item, ownItems: Item[], trees: Map<string, Item[]>): TransclusionContext | null {
  const isOwn = ownItems.some(b => b.id === item.id);

  if (isOwn && item.transcludeItemId) {
    const tree = trees.get(item.transcludeItemId);
    if (tree && tree.length > 0) {
      return { rootDisplayLevel: item.level, levelOffset: item.level - tree[0].level, rootItemId: item.id };
    }
  }

  if (!isOwn) {
    for (const [sourceId, tree] of trees) {
      if (tree.some(t => t.id === item.id) && tree.length > 0) {
        const root = ownItems.find(b => b.transcludeItemId === sourceId);
        if (root) {
          return { rootDisplayLevel: root.level, levelOffset: root.level - tree[0].level, rootItemId: root.id };
        }
      }
    }
  }

  return null;
}

/** The visible descendants of a transclusion root, without the root itself. */
export function transclusionScopedItems(visibleItems: Item[], ctx: TransclusionContext): Item[] {
  const rootIdx = visibleItems.findIndex(b => b.id === ctx.rootItemId);
  if (rootIdx < 0) return [];
  const scoped: Item[] = [];
  for (let i = rootIdx + 1; i < visibleItems.length; i++) {
    if (visibleItems[i].level <= ctx.rootDisplayLevel) break;
    scoped.push(visibleItems[i]);
  }
  return scoped;
}
