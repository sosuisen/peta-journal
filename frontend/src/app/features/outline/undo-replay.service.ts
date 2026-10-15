import { Injectable } from '@angular/core';
import { ApiService, ItemPosition } from '../../core/services/api.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { UndoService, UndoEntry, UndoOp, CursorState } from '../../core/services/undo.service';
import { Item } from '../../core/models/item.model';
import { positionByIndex } from './outline-tree';

export type UndoDirection = 'undo' | 'redo';

/** What the outline does around a replay that only the component can do. */
export interface UndoEditor {
  /** Drops typed text that is still waiting for the auto-save: the replay replaces the item anyway. */
  discardPendingEdit(): void;
  /** Focuses the editor of the item the cursor is restored to, once it is rendered. */
  focusEditor(itemId: string): void;
}

/**
 * Replays the entries recorded by {@link UndoService}: applies them to the outline state,
 * restores the cursor, and then brings the server up to date in the background.
 * Server errors are logged; the screen already shows the replayed state.
 */
@Injectable({ providedIn: 'root' })
export class UndoReplayService {
  constructor(
    private readonly state: OutlineStateService,
    private readonly api: ApiService,
    private readonly undo: UndoService,
  ) {}

  /**
   * Undoes or redoes the most recent entry.
   * @returns True when an entry was replayed
   */
  async replay(dir: UndoDirection, editor: UndoEditor): Promise<boolean> {
    if (this.undo.isPending()) return false;
    editor.discardPendingEdit();

    const entry = dir === 'undo' ? this.undo.popUndo() : this.undo.popRedo();
    if (!entry) return false;

    // Undo applies the inverse ops in reverse order; redo applies the ops as recorded
    const ops = dir === 'undo' ? [...entry.ops].reverse() : entry.ops;
    for (const op of ops) {
      this.applyToState(op, dir);
    }
    this.restoreCursor(entry, dir, editor);

    if (dir === 'undo') this.undo.pushRedo(entry);
    else this.undo.pushUndo(entry);

    this.syncToServer(ops, dir);
    return true;
  }

  /** Applies one recorded operation to the state, in the undo or the redo direction. */
  private applyToState(op: UndoOp, dir: UndoDirection): void {
    switch (op.type) {
      case 'update': {
        const target = dir === 'undo' ? op.before : op.after;
        this.state.updateItem(target.id, target);
        if (op.before.collapsed !== op.after.collapsed) {
          this.state.toggleCollapsed(target.id);
        }
        break;
      }
      case 'create':
        if (dir === 'undo') this.state.deleteItem(op.item.id);
        else this.state.addItem(op.item, this.insertionPosition(op.item));
        break;
      case 'delete':
        if (dir === 'undo') this.state.addItem(op.item, this.insertionPosition(op.item));
        else this.state.deleteItem(op.item.id);
        break;
    }
  }

  /**
   * Opens the editor where the cursor was before (undo) or after (redo) the entry.
   * Entries without a recorded cursor, such as body edits, keep editing the first updated item.
   */
  private restoreCursor(entry: UndoEntry, dir: UndoDirection, editor: UndoEditor): void {
    const cursor = this.cursorOf(entry, dir);
    if (!cursor) {
      this.state.removeAllEditingItems();
      return;
    }
    this.state.addEditingItem(cursor.itemId);
    this.state.setFocusedItem(cursor.itemId, cursor.position);
    editor.focusEditor(cursor.itemId);
  }

  private cursorOf(entry: UndoEntry, dir: UndoDirection): CursorState | null {
    const recorded = dir === 'undo' ? entry.cursorBefore : entry.cursorAfter;
    if (recorded) return recorded;
    const firstUpdate = entry.ops.find(op => op.type === 'update');
    if (firstUpdate && firstUpdate.type === 'update') {
      const target = dir === 'undo' ? firstUpdate.before : firstUpdate.after;
      return { itemId: target.id, position: target.body.length };
    }
    return null;
  }

  private insertionPosition(item: Item): ItemPosition {
    return positionByIndex(this.state.items(), item);
  }

  /** Sends the replayed ops to the server, in the order they were applied. */
  private async syncToServer(ops: UndoOp[], dir: UndoDirection): Promise<void> {
    try {
      for (const op of ops) {
        if (op.type === 'update') {
          const target = dir === 'undo' ? op.before : op.after;
          const updated = await this.api.updateItem(target.id, {
            body: target.body,
            level: target.level,
            collapsed: target.collapsed,
          });
          this.state.updateItem(updated.id, { idx: updated.idx, level: updated.level });
        } else if ((op.type === 'create') === (dir === 'redo')) {
          // Redo a create, or undo a delete: the item exists again
          await this.recreateOnServer(op);
        } else {
          // Undo a create, or redo a delete: the item is gone
          await this.api.deleteItem(op.item.id);
        }
      }
    } catch (e) {
      console.error('Failed to sync undo/redo to server:', e);
    }
  }

  /** Creates the item of a create/delete op again on the server and records the new server item in the op. */
  private async recreateOnServer(op: Extract<UndoOp, { type: 'create' | 'delete' }>): Promise<void> {
    const pos = this.insertionPosition(op.item);
    const newItem = await this.api.createItem(op.item.pageId, pos, op.item.level, op.item.body);
    // Remap: update local state with real server item
    this.state.deleteItem(op.item.id);
    this.state.addItem(newItem, pos);
    op.item = newItem;
  }
}
