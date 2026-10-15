import { Injectable } from '@angular/core';
import { ApiService, ItemPosition } from '../../core/services/api.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { UndoService } from '../../core/services/undo.service';
import { Item } from '../../core/models/item.model';
import { ItemEditService } from './item-edit.service';
import { CrossDayMove, JournalDayMoveService, isDailyJournalItem } from './journal-day-move.service';
import { findMoveDownTarget, findMoveUpTarget, getSubtree } from './move-utils';
import { TransclusionContext, selectedRootItems, transclusionContextOf, transclusionScopedItems } from './outline-tree';
import { OutlineView } from './outline-view';

export type MoveDirection = 'up' | 'down';

/**
 * Moves an item with its children past the sibling tree above or below it. Inside a
 * transclusion the move stays within the transcluded tree; at the edge of a journal day
 * the item goes to the neighbouring day instead.
 */
@Injectable({ providedIn: 'root' })
export class ItemMoveService {
  constructor(
    private readonly state: OutlineStateService,
    private readonly api: ApiService,
    private readonly undo: UndoService,
    private readonly edits: ItemEditService,
    private readonly journalMoves: JournalDayMoveService,
  ) {}

  /**
   * Moves an item and its children up or down past the neighbouring sibling tree.
   * @param item - The item to move, as it is shown now
   * @param direction - Where the item goes
   * @param view - The outline, for keeping the editor open and reloading transclusions
   * @returns What a move to another journal day did, or null when the item stayed on its page
   */
  async move(item: Item, direction: MoveDirection, view: OutlineView): Promise<CrossDayMove | null> {
    await this.edits.flushPendingEdit(item);

    const tCtx = transclusionContextOf(item, this.state.items(), this.state.transcludedTrees());
    if (tCtx && !this.state.isOwnItem(item.id)) {
      await this.moveWithinTransclusion(item, tCtx, direction, view);
      return null;
    }

    if (isDailyJournalItem(item)) {
      const crossed = await this.journalMoves.moveAcrossDays(item, direction, view.pageId());
      if (crossed) return crossed;
    }

    const candidates = this.moveCandidates(item);
    const target = direction === 'up'
      ? findMoveUpTarget(candidates, item.id)
      : findMoveDownTarget(candidates, item.id);
    if (!target) return null;
    await this.moveGroupTo(item, target.position, view);
    return null;
  }

  /** Moves every selected subtree past the sibling tree above or below the selection. */
  async moveSelection(direction: MoveDirection): Promise<void> {
    const rootItems = selectedRootItems(this.state.items(), this.state.selectedItemIds());
    if (rootItems.length === 0) return;

    const edgeRoot = direction === 'up' ? rootItems[0] : rootItems[rootItems.length - 1];
    const target = direction === 'up'
      ? findMoveUpTarget(this.state.items(), edgeRoot.id)
      : findMoveDownTarget(this.state.items(), edgeRoot.id);
    if (!target) return;

    const allSelectedIds: string[] = [];
    for (const root of rootItems) {
      for (const b of this.state.getItemWithChildren(root.id)) {
        allSelectedIds.push(b.id);
      }
    }
    const beforeSnapshots = allSelectedIds
      .map(id => this.state.getItemById(id))
      .filter((b): b is Item => !!b)
      .map(b => ({ ...b }));

    try {
      const moved = await this.api.moveItems(allSelectedIds, edgeRoot.pageId, target.position);
      this.state.replaceItems(allSelectedIds, moved, target.position);

      this.undo.begin('move-selection');
      for (let i = 0; i < beforeSnapshots.length; i++) {
        this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
      }
      this.undo.commit();
    } catch (e) {
      console.error(`Failed to move selected items ${direction}:`, e);
    }
  }

  /**
   * The items a move target is searched in. In journal mode this is the item's own page,
   * so no position refers to another page, which the backend cannot resolve.
   */
  private moveCandidates(item: Item): Item[] {
    return isDailyJournalItem(item)
      ? this.state.items().filter(b => b.pageId === item.pageId)
      : this.state.items();
  }

  /** Moves an item with its children to a position on its own page, keeping its editor open. */
  private async moveGroupTo(item: Item, position: ItemPosition, view: OutlineView): Promise<void> {
    const statePosition = this.toFullListPosition(position, item.pageId);
    const group = this.state.getItemWithChildren(item.id);
    const groupIds = group.map(b => b.id);
    const beforeSnapshots = group.map(b => ({ ...b }));

    const moved = await this.api.moveItems(groupIds, item.pageId, position);

    this.undo.begin('move');
    for (let i = 0; i < beforeSnapshots.length; i++) {
      this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
    }
    this.undo.commit();

    // Reordering the list blurs the editor; the edit goes on in the moved item
    await this.edits.handOverEdit(async () => {
      this.state.replaceItems(groupIds, moved, statePosition);
      await view.focusEditor(item.id);
    });
  }

  /** Moves a transcluded descendant within the tree of its transclusion root. */
  private async moveWithinTransclusion(item: Item, tCtx: TransclusionContext, direction: MoveDirection, view: OutlineView): Promise<void> {
    const scopedItems = transclusionScopedItems(this.state.visibleItems(), tCtx);
    const target = direction === 'up'
      ? findMoveUpTarget(scopedItems, item.id)
      : findMoveDownTarget(scopedItems, item.id);
    if (!target) return;

    const groupIds = getSubtree(this.state.visibleItems(), item.id).map(b => b.id);
    await this.api.moveItems(groupIds, item.pageId, target.position);
    // Reload to reflect reordering
    await this.edits.handOverEdit(async () => {
      await view.reloadPage();
      await view.focusEditor(item.id);
    });
  }

  /**
   * Translates a page-scoped position to a full-list position for state updates.
   * 'first' becomes the position before the first item of the page in the full list.
   * 'after:X' positions are already valid in the full list.
   */
  private toFullListPosition(position: ItemPosition, pageId: string): ItemPosition {
    if (position !== 'first') return position;
    const allItems = this.state.items();
    const firstPageIdx = allItems.findIndex(b => b.pageId === pageId);
    if (firstPageIdx <= 0) return 'first';
    return `after:${allItems[firstPageIdx - 1].id}`;
  }
}
