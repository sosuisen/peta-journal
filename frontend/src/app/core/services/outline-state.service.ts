import { Injectable, signal, computed } from '@angular/core';
import { Item } from '../models/item.model';
import { ItemPosition } from './api.service';

@Injectable({ providedIn: 'root' })
export class OutlineStateService {
  private readonly _items = signal<Item[]>([]);
  private readonly _focusedItemId = signal<string | null>(null);
  private readonly _editingItemIds = signal<Set<string>>(new Set());
  private readonly _selectedItemIds = signal<Set<string>>(new Set());
  private readonly _collapsedItemIds = signal<Set<string>>(new Set());
  private readonly _itemSelectionMode = signal<boolean>(false);
  private readonly _pendingCursorPosition = signal<number | null>(null);
  private readonly _pendingMouseDrag = signal<boolean>(false);
  private readonly _scrollRequestItemId = signal<string | null>(null);
  private readonly _pendingEditItemId = signal<string | null>(null);
  private readonly _transcludedTrees = signal<Map<string, Item[]>>(new Map());
  readonly items = this._items.asReadonly();
  readonly focusedItemId = this._focusedItemId.asReadonly();
  readonly editingItemIds = this._editingItemIds.asReadonly();
  readonly pendingCursorPosition = this._pendingCursorPosition.asReadonly();
  readonly selectedItemIds = this._selectedItemIds.asReadonly();
  readonly itemSelectionMode = this._itemSelectionMode.asReadonly();
  readonly scrollRequestItemId = this._scrollRequestItemId.asReadonly();
  readonly pendingEditItemId = this._pendingEditItemId.asReadonly();
  readonly transcludedTrees = this._transcludedTrees.asReadonly();

  /**
   * IDs of the items that belong to the pages shown in this outline.
   * Transcluded descendants are shown too, but they belong to their source page
   * and are not in this set.
   */
  readonly ownItemIds = computed(() => new Set(this._items().map(item => item.id)));

  readonly headings = computed(() => {
    return this._items()
      .filter(item => item.types?.includes('heading'))
      .map(item => {
        let level = 1;
        try {
          const ast = JSON.parse(item.ast);
          const headingNode = ast.children?.find((n: any) => n.type === 'heading');
          if (headingNode) level = headingNode.level;
        } catch {}
        const text = item.body.replace(/^#+\s*/, '');
        return { id: item.id, text, level };
      });
  });

  readonly visibleItems = computed(() => {
    const items = this._items();
    const collapsed = this._collapsedItemIds();
    const trees = this._transcludedTrees();
    const result: Item[] = [];
    let skipLevel = -1;

    for (const item of items) {
      if (skipLevel >= 0 && item.level > skipLevel) {
        continue;
      }
      skipLevel = -1;

      // For transclusion items, overlay source root's types/properties/ast
      if (item.transcludeItemId && trees.has(item.transcludeItemId)) {
        const sourceTree = trees.get(item.transcludeItemId)!;
        if (sourceTree.length > 0) {
          const sourceRoot = sourceTree[0];
          result.push({ ...item, body: sourceRoot.body, types: sourceRoot.types, properties: sourceRoot.properties, ast: sourceRoot.ast });
        } else {
          result.push(item);
        }
      } else {
        result.push(item);
      }

      // Insert transcluded descendants after a transclusion item (unless collapsed)
      if (item.transcludeItemId && !collapsed.has(item.id) && trees.has(item.transcludeItemId)) {
        const sourceTree = trees.get(item.transcludeItemId)!;
        if (sourceTree.length > 0) {
          const sourceRootLevel = sourceTree[0].level;
          let tSkipLevel = -1;
          for (let i = 1; i < sourceTree.length; i++) {
            const desc = sourceTree[i];
            const mappedLevel = item.level + (desc.level - sourceRootLevel);
            if (tSkipLevel >= 0 && mappedLevel > tSkipLevel) {
              continue;
            }
            tSkipLevel = -1;
            result.push({ ...desc, level: mappedLevel, collapsed: collapsed.has(desc.id) });
            if (collapsed.has(desc.id)) {
              tSkipLevel = mappedLevel;
            }
          }
        }
      }

      if (collapsed.has(item.id)) {
        skipLevel = item.level;
      }
    }
    return result;
  });

  /**
   * Replaces the transcluded trees with the given ones.
   * @param trees - Source item ID to the source item and its descendants.
   */
  setTranscludedTrees(trees: Record<string, Item[]>): void {
    this._transcludedTrees.set(new Map(Object.entries(trees)));
    this.applyCollapsedFlags(trees);
  }

  /**
   * Adds trees to the ones already held, e.g. when another page is appended to a journal.
   * A tree that is already known is replaced by the new one.
   * @param trees - Source item ID to the source item and its descendants.
   */
  mergeTranscludedTrees(trees: Record<string, Item[]>): void {
    this._transcludedTrees.update(current => new Map([...current, ...Object.entries(trees)]));
    this.applyCollapsedFlags(trees);
  }

  /** Copies the collapsed flag of every transcluded item into the collapsed set. */
  private applyCollapsedFlags(trees: Record<string, Item[]>): void {
    this._collapsedItemIds.update(set => {
      const next = new Set(set);
      for (const items of Object.values(trees)) {
        for (const item of items) {
          if (item.collapsed) next.add(item.id);
          else next.delete(item.id);
        }
      }
      return next;
    });
  }


  /**
   * Replaces all items and rebuilds the collapsed item set from their collapsed flags.
   * @param items - The new complete list of items.
   */
  setItems(items: Item[]): void {
    this._items.set(items);
    const collapsed = new Set<string>();
    for (const b of items) {
      if (b.collapsed) collapsed.add(b.id);
    }
    this._collapsedItemIds.set(collapsed);
  }

  /**
   * Inserts a single item at the specified position.
   * @param item - The item to insert.
   * @param position - The position to insert at.
   */
  addItem(item: Item, position: ItemPosition): void {
    this._items.update(items => {
      const idx = this.resolveInsertionIndex(items, position);
      const newItems = [...items];
      newItems.splice(idx, 0, item);
      return newItems;
    });
  }

  /**
   * Partially updates a item's properties by merging the given updates.
   * @param itemId - The ID of the item to update.
   * @param updates - The partial item properties to merge.
   */
  updateItem(itemId: string, updates: Partial<Item>): void {
    const found = this._items().some(b => b.id === itemId);
    if (found) {
      this._items.update(items =>
        items.map(b => b.id === itemId ? { ...b, ...updates } : b)
      );
    } else {
      // Update within transcluded trees
      this._transcludedTrees.update(trees => {
        const next = new Map(trees);
        for (const [key, items] of next) {
          const idx = items.findIndex(b => b.id === itemId);
          if (idx >= 0) {
            const updated = [...items];
            updated[idx] = { ...updated[idx], ...updates };
            next.set(key, updated);
            break;
          }
        }
        return next;
      });
    }
  }

  /**
   * Removes a single item by its ID.
   * @param itemId - The ID of the item to remove.
   */
  deleteItem(itemId: string): void {
    if (this._items().some(b => b.id === itemId)) {
      this._items.update(items => items.filter(b => b.id !== itemId));
    } else {
      // Remove from transcluded trees
      this._transcludedTrees.update(trees => {
        const next = new Map(trees);
        for (const [key, items] of next) {
          const idx = items.findIndex(b => b.id === itemId);
          if (idx >= 0) {
            next.set(key, items.filter(b => b.id !== itemId));
            break;
          }
        }
        return next;
      });
    }
  }

  /**
   * Adds a item to the editing set (textarea visible).
   */
  addEditingItem(itemId: string): void {
    this._editingItemIds.update(s => { const n = new Set(s); n.add(itemId); return n; });
  }

  /**
   * Focuses a item and sets a pending cursor position for the next time the item is edited.
   */
  setFocusedItem(itemId: string, cursorPosition: number | null = null): void {
    this._pendingCursorPosition.set(cursorPosition);
    this._focusedItemId.set(itemId);
  }

  /**
   * Removes a item from the editing set (switches it to view mode).
   */
  removeEditingItem(itemId: string): void {
    this._editingItemIds.update(s => { const n = new Set(s); n.delete(itemId); return n; });
  }

  removeAllEditingItems(): void {
    this._editingItemIds.set(new Set());
  }

  /**
   * Returns and clears the pending cursor position (one-time read).
   * @returns The pending cursor position, or null if none was set.
   */
  consumePendingCursorPosition(): number | null {
    const pos = this._pendingCursorPosition();
    this._pendingCursorPosition.set(null);
    return pos;
  }

  setPendingMouseDrag(value: boolean): void {
    this._pendingMouseDrag.set(value);
  }

  consumePendingMouseDrag(): boolean {
    const value = this._pendingMouseDrag();
    this._pendingMouseDrag.set(false);
    return value;
  }

  requestScrollToItem(itemId: string): void {
    this._scrollRequestItemId.set(itemId);
  }

  clearScrollRequest(): void {
    this._scrollRequestItemId.set(null);
  }

  requestEditItem(itemId: string): void {
    this._pendingEditItemId.set(itemId);
  }

  clearPendingEditItem(): void {
    this._pendingEditItemId.set(null);
  }

  isCollapsed(itemId: string): boolean {
    return this._collapsedItemIds().has(itemId);
  }

  /**
   * Toggles the collapsed state of a item, hiding or showing its children.
   * @param itemId - The ID of the item to toggle.
   */
  toggleCollapsed(itemId: string): void {
    this._collapsedItemIds.update(set => {
      const next = new Set(set);
      if (next.has(itemId)) next.delete(itemId);
      else next.add(itemId);
      return next;
    });
  }

  /**
   * Checks whether an item belongs to a page shown in this outline (see {@link ownItemIds}).
   * @param itemId - The ID of the item to check.
   */
  isOwnItem(itemId: string): boolean {
    return this.ownItemIds().has(itemId);
  }

  /**
   * Finds a item by its ID.
   * @param itemId - The ID of the item to find.
   * @returns The matching item, or undefined if not found.
   */
  getItemById(itemId: string): Item | undefined {
    const found = this._items().find(b => b.id === itemId);
    if (found) return found;
    // Search in transcluded trees
    for (const tree of this._transcludedTrees().values()) {
      const t = tree.find(b => b.id === itemId);
      if (t) return t;
    }
    return undefined;
  }

  /**
   * Checks whether the item has any immediate children (next item at a deeper level).
   * @param itemId - The ID of the item to check.
   * @returns True if the item has children, false otherwise.
   */
  hasChildren(itemId: string): boolean {
    const items = this._items();
    const idx = items.findIndex(b => b.id === itemId);
    if (idx >= 0) {
      const item = items[idx];
      // Transclusion root: has children if source tree has descendants
      if (item.transcludeItemId) {
        const trees = this._transcludedTrees();
        const tree = trees.get(item.transcludeItemId);
        if (tree && tree.length > 1) return true;
      }
      if (idx < items.length - 1 && items[idx + 1].level > items[idx].level) return true;
      return false;
    }
    // Item not in _items — check transcluded trees (source descendant)
    const trees = this._transcludedTrees();
    for (const tree of trees.values()) {
      const tIdx = tree.findIndex(t => t.id === itemId);
      if (tIdx >= 0 && tIdx < tree.length - 1) {
        return tree[tIdx + 1].level > tree[tIdx].level;
      }
    }
    return false;
  }

  /**
   * Get all descendant items of the given item.
   * Children are all subsequent items with a level strictly greater than the parent's level,
   * stopping at the first item at the same or lower level.
   */
  getChildren(itemId: string): Item[] {
    const items = this._items();
    const idx = items.findIndex(b => b.id === itemId);
    if (idx < 0) return [];
    const parentLevel = items[idx].level;
    const children: Item[] = [];
    for (let i = idx + 1; i < items.length; i++) {
      if (items[i].level <= parentLevel) break;
      children.push(items[i]);
    }
    return children;
  }

  /**
   * Get the item and all its descendants as a group.
   */
  getItemWithChildren(itemId: string): Item[] {
    const item = this.getItemById(itemId);
    if (!item) return [];
    return [item, ...this.getChildren(itemId)];
  }

  /**
   * Inserts multiple items at the specified position.
   * @param items - The items to insert.
   * @param position - The position to insert at.
   */
  addItems(items: Item[], position: ItemPosition): void {
    this._items.update(current => {
      const idx = this.resolveInsertionIndex(current, position);
      const newItems = [...current];
      newItems.splice(idx, 0, ...items);
      return newItems;
    });
  }

  /**
   * Removes multiple items by their IDs.
   * @param itemIds - The IDs of the items to remove.
   */
  deleteItems(itemIds: string[]): void {
    const idSet = new Set(itemIds);
    this._items.update(items => items.filter(b => !idSet.has(b.id)));
  }

  /**
   * Atomically remove old items and insert new items at the target position.
   * This avoids the visual duplication that happens with separate delete + add calls.
   */
  replaceItems(oldItemIds: string[], newItems: Item[], position: ItemPosition): void {
    const idSet = new Set(oldItemIds);
    this._items.update(current => {
      const filtered = current.filter(b => !idSet.has(b.id));
      const idx = this.resolveInsertionIndex(filtered, position);
      const result = [...filtered];
      result.splice(idx, 0, ...newItems);
      return result;
    });
  }

  /**
   * Sets the set of currently selected item IDs.
   * @param ids - The set of item IDs to mark as selected.
   */
  setSelectedItemIds(ids: Set<string>): void {
    this._selectedItemIds.set(ids);
  }

  /**
   * Enables or disables item selection mode.
   * @param active - Whether item selection mode should be active.
   */
  setItemSelectionMode(active: boolean): void {
    this._itemSelectionMode.set(active);
  }

  /**
   * Clears all selected items and exits item selection mode.
   */
  clearSelection(): void {
    this._selectedItemIds.set(new Set());
    this._itemSelectionMode.set(false);
  }

  /**
   * Resolves an ItemPosition to a splice index within the items array.
   */
  private resolveInsertionIndex(items: Item[], position: ItemPosition): number {
    if (position === 'first') return 0;
    if (position === 'last') return items.length;
    // after:<id>
    const afterId = position.substring(6);
    const idx = items.findIndex(b => b.id === afterId);
    return idx >= 0 ? idx + 1 : items.length;
  }
}
