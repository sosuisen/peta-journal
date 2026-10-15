import { Injectable } from '@angular/core';
import { ApiService } from '../../core/services/api.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { UndoService } from '../../core/services/undo.service';
import { Item } from '../../core/models/item.model';
import { ItemEditService } from './item-edit.service';
import { isDailyJournalItem } from './journal-day-move.service';
import { OutlineView } from './outline-view';

/**
 * Joins two neighbouring items into one: Backspace at the start of an item pulls it into
 * the item above, Delete at the end pulls the item below into it. A transclusion is never
 * merged, and the days of a journal are not merged into each other.
 */
@Injectable({ providedIn: 'root' })
export class ItemMergeService {
  constructor(
    private readonly state: OutlineStateService,
    private readonly api: ApiService,
    private readonly undo: UndoService,
    private readonly edits: ItemEditService,
  ) {}

  /**
   * Backspace at the start: the item's text goes to the end of the item above, which keeps
   * the editor, and the item is deleted.
   * @param item - The item where Backspace was pressed at position 0
   * @param view - The outline, which focuses the editor of the item above
   */
  async mergeIntoPrevious(item: Item, view: OutlineView): Promise<void> {
    // Prevent merge on transclusion items
    if (item.transcludeItemId) return;

    const items = this.state.visibleItems();
    const idx = items.findIndex(b => b.id === item.id);
    if (idx <= 0) return;

    const prevItem = items[idx - 1];

    // In journal daily pages, prevent merging across page boundaries
    if (isDailyJournalItem(item) && prevItem.pageId !== item.pageId) return;

    this.edits.discardPendingEdit(item);
    const originalPrev = { ...prevItem };
    const originalItem = { ...item };
    const cursorPos = prevItem.body.length;
    const mergedBody = prevItem.body + item.body;
    this.state.updateItem(prevItem.id, { body: mergedBody });
    this.state.deleteItem(item.id);
    this.state.addEditingItem(prevItem.id);
    view.focusEditor(prevItem.id);

    const updatedPrev = await this.edits.saveBody(prevItem.id, mergedBody);
    await this.api.deleteItem(item.id);

    this.undo.begin('backspace', { itemId: item.id, position: 0 });
    this.undo.pushUpdate(originalPrev, updatedPrev);
    this.undo.pushDelete(originalItem);
    this.undo.commit({ itemId: prevItem.id, position: cursorPos });
  }

  /**
   * Delete at the end: the text of the item below is appended to this item, and the item
   * below is deleted.
   * @param item - The item where Delete was pressed at the end
   */
  async mergeNextInto(item: Item): Promise<void> {
    const items = this.state.visibleItems();
    const idx = items.findIndex(b => b.id === item.id);
    if (idx >= items.length - 1) return;

    const nextItem = items[idx + 1];
    // Same limits as Backspace: a transclusion is not merged away, and journal days are not merged
    if (nextItem.transcludeItemId) return;
    if (isDailyJournalItem(item) && nextItem.pageId !== item.pageId) return;

    this.edits.discardPendingEdit(nextItem);

    const originalItem = { ...item };
    const originalNext = { ...nextItem };
    const cursorPos = item.body.length;
    const mergedBody = item.body + nextItem.body;
    this.state.updateItem(item.id, { body: mergedBody });
    this.state.deleteItem(nextItem.id);

    const updatedItem = await this.edits.saveBody(item.id, mergedBody);
    await this.api.deleteItem(nextItem.id);

    this.undo.begin('delete', { itemId: item.id, position: cursorPos });
    this.undo.pushUpdate(originalItem, updatedItem);
    this.undo.pushDelete(originalNext);
    this.undo.commit({ itemId: item.id, position: cursorPos });
  }
}
