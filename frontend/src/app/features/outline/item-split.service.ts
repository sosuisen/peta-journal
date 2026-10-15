import { Injectable } from '@angular/core';
import { ApiService, ItemPosition } from '../../core/services/api.service';
import { HmtidService } from '../../core/services/hmtid.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { UndoService } from '../../core/services/undo.service';
import { Item } from '../../core/models/item.model';
import { ItemEditService } from './item-edit.service';
import { LevelShiftService } from './level-shift.service';
import {
  EditContext, editContextOf, hasSiblingBelow, levelForNewItemAfter, positionAfterSubtree, positionBefore,
} from './outline-tree';
import { OutlineView } from './outline-view';

/**
 * What Enter does in an editor: splits the item at the cursor, or inserts an empty item
 * before or after it. An empty last child is outdented instead.
 */
@Injectable({ providedIn: 'root' })
export class ItemSplitService {
  constructor(
    private readonly state: OutlineStateService,
    private readonly api: ApiService,
    private readonly undo: UndoService,
    private readonly edits: ItemEditService,
    private readonly levels: LevelShiftService,
    private readonly hmtid: HmtidService,
  ) {}

  /**
   * Handles Enter at a cursor position in an item.
   * @param item - The item where Enter was pressed
   * @param cursorPos - The cursor position at the time of the key press
   * @param view - The outline, for focusing editors and reloading transclusions
   */
  async handleEnter(item: Item, cursorPos: number, view: OutlineView): Promise<void> {
    this.edits.discardPendingEdit(item);

    if (item.body.length === 0 && item.level > 1 && !hasSiblingBelow(this.state.visibleItems(), item.id)) {
      // Empty last child: outdent instead of creating a new item
      await this.levels.outdent(item, view);
      view.focusEditor(item.id);
      return;
    }

    await this.edits.handOverEdit(async () => {
      if (cursorPos === 0 && item.body.length > 0) {
        await this.insertEmptyItemBefore(item, cursorPos, view);
      } else if (cursorPos === item.body.length) {
        await this.appendEmptyItemAfter(item, cursorPos, view);
      } else {
        await this.splitItemAt(item, cursorPos, view);
      }
    });
  }

  /** Enter at the beginning: a new empty item goes before the current one, which keeps its text. */
  private async insertEmptyItemBefore(item: Item, cursorPos: number, view: OutlineView): Promise<void> {
    const ctx = this.editContextOf(item);
    // Inside a transclusion the new item goes to the top of the source page
    const position: ItemPosition = ctx.inTransclusion ? 'first' : positionBefore(this.state.items(), item.id);
    const [newItem] = await Promise.all([
      this.api.createItem(ctx.pageId, position, item.level - ctx.levelOffset, ''),
      this.edits.saveBody(this.edits.saveTargetId(item), item.body),
    ]);
    await this.showCreatedItem(newItem, position, ctx, view);

    this.undo.begin('enter', { itemId: item.id, position: cursorPos });
    this.undo.pushCreate(newItem);
    this.undo.commit({ itemId: newItem.id, position: 0 });

    this.state.addEditingItem(newItem.id);
    view.focusEditor(newItem.id);
    this.state.removeEditingItem(item.id);
  }

  /**
   * Enter at the end: the new item is shown at once with a client-side ID and
   * created on the server in the background.
   */
  private async appendEmptyItemAfter(item: Item, cursorPos: number, view: OutlineView): Promise<void> {
    const ctx = this.editContextOf(item);
    const newLevel = this.levelForNewItemAfter(item, ctx);
    const position = positionAfterSubtree(this.state.items(), item, this.edits.saveTargetId(item));
    const newId = this.hmtid.nextId();
    const optimisticItem: Item = {
      id: newId,
      pageId: ctx.pageId,
      dirPath: item.dirPath,
      name: item.name,
      type: item.type,
      createdDate: Date.now(),
      modifiedDate: Date.now(),
      idx: '',
      level: newLevel,
      body: '',
      ast: '',
      collapsed: false,
      types: [],
      properties: {},
    };
    this.api.createItem(ctx.pageId, position, newLevel - ctx.levelOffset, '', newId).then(async serverItem => {
      if (ctx.inTransclusion) {
        await view.reloadPage();
      } else {
        this.state.updateItem(newId, { idx: serverItem.idx });
      }
    });
    this.state.addItem(optimisticItem, position);

    this.undo.begin('enter', { itemId: item.id, position: cursorPos });
    this.undo.pushUpdate({ ...item }, { ...item });
    this.undo.pushCreate(optimisticItem);
    this.undo.commit({ itemId: newId, position: 0 });

    this.edits.discardPendingEdit(item);
    this.state.addEditingItem(newId);
    this.state.setFocusedItem(newId);
    view.focusEditor(newId);

    await this.edits.saveBody(this.edits.saveTargetId(item), item.body.substring(0, cursorPos));
    this.state.removeEditingItem(item.id);
  }

  /** Enter in the middle: the text after the cursor moves to a new item below. */
  private async splitItemAt(item: Item, cursorPos: number, view: OutlineView): Promise<void> {
    const ctx = this.editContextOf(item);
    const newLevel = this.levelForNewItemAfter(item, ctx);
    const position = positionAfterSubtree(this.state.items(), item, this.edits.saveTargetId(item));
    const beforeText = item.body.substring(0, cursorPos);
    const afterText = item.body.substring(cursorPos);
    const originalItem = { ...item };

    const [updatedOriginal, newItem] = await Promise.all([
      this.edits.saveBody(this.edits.saveTargetId(item), beforeText),
      this.api.createItem(ctx.pageId, position, newLevel - ctx.levelOffset, afterText),
    ]);
    await this.showCreatedItem(newItem, position, ctx, view);

    this.undo.begin('enter', { itemId: item.id, position: cursorPos });
    this.undo.pushUpdate(originalItem, updatedOriginal);
    this.undo.pushCreate(newItem);
    this.undo.commit({ itemId: newItem.id, position: 0 });

    this.state.addEditingItem(newItem.id);
    this.state.setFocusedItem(newItem.id);
    view.focusEditor(newItem.id);
    this.state.removeEditingItem(item.id);
  }

  /** Puts a newly created item into the state. Inside a transclusion the page is reloaded so the trees are fresh. */
  private async showCreatedItem(newItem: Item, position: ItemPosition, ctx: EditContext, view: OutlineView): Promise<void> {
    if (ctx.inTransclusion) {
      await view.reloadPage();
    } else {
      this.state.addItem(newItem, position);
    }
  }

  /** The shown level of a new item after this one; a transclusion reads the neighbour from the visible list. */
  private levelForNewItemAfter(item: Item, ctx: EditContext): number {
    return levelForNewItemAfter(ctx.inTransclusion ? this.state.visibleItems() : this.state.items(), item);
  }

  private editContextOf(item: Item): EditContext {
    return editContextOf(item, this.state.ownItemIds(), this.state.transcludedTrees());
  }
}
