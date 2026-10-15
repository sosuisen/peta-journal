import { Injectable } from '@angular/core';
import { ApiService } from '../../core/services/api.service';
import { AutoSaveService } from '../../core/services/auto-save.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { Item } from '../../core/models/item.model';
import { ensureCodeFenceClosed, hasUnclosedCodeFence } from './item-editor';

/**
 * The life of an edit: what was typed and not yet saved, how it reaches the server,
 * and when the blur of an editor may end the edit.
 *
 * Typed text goes to the server in two ways. The auto-save sends it after a short
 * pause; a handler that needs the text saved now sends it itself and cancels the
 * auto-save, so the same text is never sent twice.
 */
@Injectable({ providedIn: 'root' })
export class ItemEditService {
  /** Shown item IDs whose body was typed into and not yet saved by {@link saveShownItem}. */
  private readonly dirtyItemIds = new Set<string>();
  /**
   * Number of handlers that are saving the edited item and choosing the next edited
   * item themselves (clicking another item, moving an item, splitting it with Enter).
   * While it is above zero the blur of the editor is ignored, see {@link finishEdit}.
   */
  private editHandovers = 0;

  constructor(
    private readonly state: OutlineStateService,
    private readonly api: ApiService,
    private readonly autoSave: AutoSaveService,
  ) {}

  /** The item an edit of the shown item is saved to: a transclusion root edits its source item. */
  saveTargetId(item: Item): string {
    return item.transcludeItemId ?? item.id;
  }

  /** True while an editor is open or typed text has not been saved yet. */
  isEditInProgress(): boolean {
    return this.state.editingItemIds().size > 0 || this.dirtyItemIds.size > 0;
  }

  /** Records typed text: the state shows it at once and the auto-save sends it later. */
  recordTyping(item: Item, newBody: string): void {
    this.state.updateItem(item.id, { body: newBody });
    this.dirtyItemIds.add(item.id);
    this.autoSave.scheduleItemUpdate(this.saveTargetId(item), newBody);
  }

  /** Marks an item as needing a save on the next edit end, e.g. after a paste changed its body. */
  markDirty(itemId: string): void {
    this.dirtyItemIds.add(itemId);
  }

  /**
   * Ends the edit of an item when its editor loses focus: saves typed text, or a body
   * whose code fence is still open, and switches the item to view mode.
   * Does nothing while a handover runs or when another handler already ended the edit.
   */
  async finishEdit(item: Item): Promise<void> {
    if (this.editHandovers > 0) return;
    if (!this.state.editingItemIds().has(item.id)) return;

    if (this.dirtyItemIds.has(item.id)) {
      this.discardPendingEdit(item);
      await this.saveShownItem(item);
    } else {
      const current = this.state.getItemById(item.id);
      if (current && hasUnclosedCodeFence(current.body)) {
        await this.saveShownItem(item);
      }
    }
    this.state.removeEditingItem(item.id);
  }

  /**
   * Runs a handler that saves the edited item and decides which item is edited next.
   * The blur the handler causes must not end the edit a second time, so
   * {@link finishEdit} ignores it while the handler runs.
   */
  async handOverEdit<T>(handler: () => Promise<T>): Promise<T> {
    this.editHandovers++;
    try {
      return await handler();
    } finally {
      this.editHandovers--;
    }
  }

  /**
   * Saves the body the shown item has in the state. When the item is a transclusion root,
   * the AST and types returned for the source are copied back to the shown item.
   * @returns The saved item, or undefined when the item is no longer in the state
   */
  async saveShownItem(item: Item): Promise<Item | undefined> {
    const current = this.state.getItemById(item.id);
    if (!current) return undefined;
    const targetId = this.saveTargetId(item);
    const updated = await this.saveBody(targetId, current.body);
    if (targetId !== item.id) {
      this.state.updateItem(item.id, { ast: updated.ast, types: updated.types });
    }
    return updated;
  }

  /** Saves typed text right away and closes the editor; a clean item is left alone. */
  async saveItemIfDirty(item: Item): Promise<void> {
    if (!this.dirtyItemIds.has(item.id)) return;
    this.discardPendingEdit(item);
    if (await this.saveShownItem(item)) {
      this.state.removeEditingItem(item.id);
    }
  }

  /**
   * Sends typed text that is still waiting for the auto-save to the server right now.
   * Used before an operation whose server response replaces the item in the state,
   * so the response carries the typed text instead of an older body.
   */
  async flushPendingEdit(item: Item): Promise<void> {
    if (!this.dirtyItemIds.has(item.id)) return;
    this.dirtyItemIds.delete(item.id);
    await this.autoSave.flushItem(this.saveTargetId(item));
  }

  /** Drops typed text that is still waiting for the auto-save, because it is saved another way. */
  discardPendingEdit(item: Item): void {
    this.dirtyItemIds.delete(item.id);
    this.autoSave.cancelItem(this.saveTargetId(item));
  }

  /** Drops the pending edit of the focused item, e.g. before an undo replaces that item. */
  discardPendingEditOfFocusedItem(): void {
    const focusedId = this.state.focusedItemId();
    if (!focusedId || !this.dirtyItemIds.has(focusedId)) return;
    const focused = this.state.getItemById(focusedId);
    this.dirtyItemIds.delete(focusedId);
    this.autoSave.cancelItem(focused ? this.saveTargetId(focused) : focusedId);
  }

  /** Saves a body to the server and shows the server's rendering of it. An open code fence is closed first. */
  async saveBody(itemId: string, body: string): Promise<Item> {
    const normalized = ensureCodeFenceClosed(body);
    this.state.updateItem(itemId, { body: normalized });
    const updated = await this.api.updateItem(itemId, { body: normalized });
    this.state.updateItem(itemId, { body: updated.body, ast: updated.ast, types: updated.types, properties: updated.properties });
    return updated;
  }
}
