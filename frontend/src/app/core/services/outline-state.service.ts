import { Injectable, signal, computed } from '@angular/core';
import { Item } from '../models/item.model';

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
  readonly items = this._items.asReadonly();
  readonly focusedItemId = this._focusedItemId.asReadonly();
  readonly editingItemIds = this._editingItemIds.asReadonly();
  readonly pendingCursorPosition = this._pendingCursorPosition.asReadonly();
  readonly selectedItemIds = this._selectedItemIds.asReadonly();
  readonly itemSelectionMode = this._itemSelectionMode.asReadonly();

  readonly visibleItems = computed(() => {
    const items = this._items();
    const collapsed = this._collapsedItemIds();
    const result: Item[] = [];
    let skipLevel = -1;

    for (const item of items) {
      if (skipLevel >= 0 && item.level > skipLevel) {
        continue;
      }
      skipLevel = -1;
      result.push(item);
      if (collapsed.has(item.id)) {
        skipLevel = item.level;
      }
    }
    return result;
  });

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
   * Inserts a single item after the specified item, or at the beginning if not found.
   * @param item - The item to insert.
   * @param afterItemId - The ID of the item to insert after.
   */
  addItem(item: Item, afterItemId: string): void {
    this._items.update(items => {
      const idx = afterItemId ? items.findIndex(b => b.id === afterItemId) : -1;
      const newItems = [...items];
      newItems.splice(idx + 1, 0, item);
      return newItems;
    });
  }

  /**
   * Partially updates a item's properties by merging the given updates.
   * @param itemId - The ID of the item to update.
   * @param updates - The partial item properties to merge.
   */
  updateItem(itemId: string, updates: Partial<Item>): void {
    this._items.update(items =>
      items.map(b => b.id === itemId ? { ...b, ...updates } : b)
    );
  }

  /**
   * Removes a single item by its ID.
   * @param itemId - The ID of the item to remove.
   */
  deleteItem(itemId: string): void {
    this._items.update(items => items.filter(b => b.id !== itemId));
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
   * Finds a item by its ID.
   * @param itemId - The ID of the item to find.
   * @returns The matching item, or undefined if not found.
   */
  getItemById(itemId: string): Item | undefined {
    return this._items().find(b => b.id === itemId);
  }

  /**
   * Checks whether the item has any immediate children (next item at a deeper level).
   * @param itemId - The ID of the item to check.
   * @returns True if the item has children, false otherwise.
   */
  hasChildren(itemId: string): boolean {
    const items = this._items();
    const idx = items.findIndex(b => b.id === itemId);
    if (idx < 0 || idx === items.length - 1) return false;
    return items[idx + 1].level > items[idx].level;
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
   * Inserts multiple items after the specified item, or at the beginning if not found.
   * @param items - The items to insert.
   * @param afterItemId - The ID of the item to insert after.
   */
  addItems(items: Item[], afterItemId: string): void {
    this._items.update(current => {
      const idx = afterItemId ? current.findIndex(b => b.id === afterItemId) : -1;
      const newItems = [...current];
      newItems.splice(idx + 1, 0, ...items);
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
  replaceItems(oldItemIds: string[], newItems: Item[], afterItemId: string): void {
    const idSet = new Set(oldItemIds);
    this._items.update(current => {
      const filtered = current.filter(b => !idSet.has(b.id));
      const idx = afterItemId ? filtered.findIndex(b => b.id === afterItemId) : -1;
      const result = [...filtered];
      result.splice(idx + 1, 0, ...newItems);
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
}
