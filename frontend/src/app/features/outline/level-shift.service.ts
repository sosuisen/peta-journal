import { Injectable } from '@angular/core';
import { ApiService } from '../../core/services/api.service';
import { ConfigService } from '../../core/services/config.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { UndoService } from '../../core/services/undo.service';
import { Item } from '../../core/models/item.model';
import { getSubtree } from './move-utils';
import { canIndent, lastDescendantOutside, parentIndexOf, transclusionContextOf } from './outline-tree';
import { OutlineView } from './outline-view';

/**
 * Indents and outdents items. The change is shown at once and saved as a relative
 * level shift, so key presses that overtake each other still add up on the server.
 */
@Injectable({ providedIn: 'root' })
export class LevelShiftService {
  constructor(
    private readonly state: OutlineStateService,
    private readonly api: ApiService,
    private readonly undo: UndoService,
    private readonly config: ConfigService,
  ) {}

  /**
   * Indents an item and its children by one level, when the item above allows it.
   * @param item - The item to indent
   * @param view - The outline, reloaded when the save fails
   */
  async indent(item: Item, view: OutlineView): Promise<void> {
    const items = this.state.isOwnItem(item.id) ? this.state.items() : this.state.visibleItems();
    if (!canIndent(items, item.id)) return;

    const tCtx = transclusionContextOf(item, this.state.items(), this.state.transcludedTrees());
    if (tCtx) {
      await this.shiftLevels(getSubtree(this.state.visibleItems(), item.id), 1, 'indent', view, tCtx.levelOffset);
      return;
    }
    await this.shiftLevels(this.state.getItemWithChildren(item.id), 1, 'indent', view);
  }

  /**
   * Outdents an item and its children by one level.
   *
   * - If the previous item is at level-1, outdent in place (both modes).
   * - Otherwise, find the parent (an item above at level-1):
   *   - logicalOutdent=true: move group to after parent's last child, then outdent.
   *     Siblings below stay with the original parent.
   *   - logicalOutdent=false: outdent in place. Siblings below don't move but
   *     their parent effectively changes to the outdented item.
   *
   * @param item - The item to outdent
   * @param view - The outline, reloaded when the save fails
   */
  async outdent(item: Item, view: OutlineView): Promise<void> {
    if (item.level <= 1) return;

    // For transcluded descendants, prevent outdenting beyond transclusion root level
    const tCtx = transclusionContextOf(item, this.state.items(), this.state.transcludedTrees());
    if (tCtx && item.level <= tCtx.rootDisplayLevel + 1) return; // Can't go to same level or above root

    if (tCtx && !this.state.isOwnItem(item.id)) {
      // Transcluded descendant: simple in-place outdent on source page
      await this.shiftLevels(getSubtree(this.state.visibleItems(), item.id), -1, 'outdent', view, tCtx.levelOffset);
      return;
    }

    const items = this.state.items();
    const idx = items.findIndex(b => b.id === item.id);
    if (idx < 0) return;
    const prevItem = idx > 0 ? items[idx - 1] : undefined;
    const parentIdx = parentIndexOf(items, idx);
    if (parentIdx < 0) return; // No parent found, cannot outdent

    const group = this.state.getItemWithChildren(item.id);
    const needsMove = !prevItem || prevItem.level !== item.level - 1;

    if (needsMove && this.config.logicalOutdent()) {
      await this.outdentBelowParent(item, group, items[parentIdx]);
    } else {
      // In-place outdent: decrease level of group
      await this.shiftLevels(group, -1, 'outdent', view);
    }
  }

  /**
   * Logical outdent: the group is placed after the parent's last child and then outdented,
   * so the siblings below it stay with the original parent.
   */
  private async outdentBelowParent(item: Item, group: Item[], parent: Item): Promise<void> {
    const groupIds = group.map(b => b.id);
    const lastChildId = lastDescendantOutside(this.state.items(), parent.id, new Set(groupIds));
    const beforeSnapshots = group.map(b => ({ ...b }));

    for (const b of group) {
      this.state.updateItem(b.id, { level: b.level - 1 });
    }
    await this.api.shiftItemLevels(groupIds, -1);

    if (lastChildId !== parent.id) {
      // Move after the last child
      const moved = await this.api.moveItems(groupIds, item.pageId, `after:${lastChildId}`);
      this.state.replaceItems(groupIds, moved, `after:${lastChildId}`);
      this.undo.begin('outdent');
      for (let i = 0; i < beforeSnapshots.length; i++) {
        this.undo.pushUpdate(beforeSnapshots[i], { ...moved[i] });
      }
    } else {
      // The parent has no other child: the group only changes its level
      this.undo.begin('outdent');
      for (const b of beforeSnapshots) {
        this.undo.pushUpdate(b, { ...b, level: b.level - 1 });
      }
    }
    this.undo.commit();
  }

  /**
   * Changes the level of a group of items on screen, records the change for undo and saves it.
   * @param group - The items whose level changes
   * @param delta - The amount added to each level; negative for outdent
   * @param label - The undo label
   * @param view - The outline, reloaded when the save fails
   * @param levelOffset - Shown level minus stored level, for items inside a transclusion
   */
  async shiftLevels(group: Item[], delta: number, label: string, view: OutlineView, levelOffset = 0): Promise<void> {
    this.undo.begin(label);
    for (const b of group) {
      this.undo.pushUpdate({ ...b }, { ...b, level: b.level + delta });
      this.state.updateItem(b.id, { level: b.level - levelOffset + delta });
    }
    this.undo.commit();
    await this.saveLevelShift(group, delta, view);
  }

  /**
   * Saves an indent or outdent that is already shown on screen.
   * The whole group goes in one relative request, and the response is not written back
   * to the state: a response to an older key press would undo a newer one.
   * If the save fails, the affected pages are reloaded so the screen matches the server again.
   */
  private async saveLevelShift(group: Item[], delta: number, view: OutlineView): Promise<void> {
    if (group.length === 0) return;
    try {
      await this.api.shiftItemLevels(group.map(b => b.id), delta);
    } catch (e) {
      console.error('Failed to change item levels:', e);
      for (const pageId of new Set(group.map(b => b.pageId))) {
        view.reloadPageFromFile(pageId);
      }
    }
  }
}
