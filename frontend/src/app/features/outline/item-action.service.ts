import { Injectable } from '@angular/core';
import { renderToHtml } from 'flatmarkdown-ast2html';
import { ApiService } from '../../core/services/api.service';
import { ClipboardService } from '../../core/services/clipboard.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { UndoService } from '../../core/services/undo.service';
import { Item } from '../../core/models/item.model';
import { ItemEditService } from './item-edit.service';
import { isDailyJournalItem } from './journal-day-move.service';
import { OutlineView } from './outline-view';

/** The zoom of an item as stored in its properties; 1 when unset. */
export function zoomOf(item: Item): number {
  return parseFloat(item.properties['zoom'] as string) || 1;
}

/**
 * One-shot commands on items that change them on screen, save them and record the
 * change for undo where it can be undone: task state, zoom, collapsing, transclusion,
 * archiving, and deleting or copying several items at once.
 */
@Injectable({ providedIn: 'root' })
export class ItemActionService {
  constructor(
    private readonly state: OutlineStateService,
    private readonly api: ApiService,
    private readonly undo: UndoService,
    private readonly edits: ItemEditService,
    private readonly clipboard: ClipboardService,
  ) {}

  /**
   * Cycles the task state: normal → task → running → done → normal.
   * An item that just became a task on a daily journal gets the journal date as its plan.
   */
  async cycleTask(item: Item): Promise<void> {
    try {
      // For transclusion items, target the source item
      const targetId = this.edits.saveTargetId(item);
      const originalItem = { ...item };
      const wasTask = item.types?.includes('task');
      let updated = await this.api.cycleTask(targetId);

      // When an item just became a task on a daily journal, set plan to the journal date
      if (!wasTask && updated.types?.includes('task') && isDailyJournalItem(item)) {
        updated = await this.api.upsertItemProperty(targetId, 'plan', 'date', item.name);
      }

      const change = { types: updated.types, properties: updated.properties };
      this.state.updateItem(targetId, change);
      this.undo.begin('cycle-task');
      this.undo.pushUpdate(originalItem, { ...originalItem, ...change });
      this.undo.commit();
    } catch (e) {
      console.error('Failed to cycle task:', e);
    }
  }

  /** Shows and saves a zoom factor for the item. */
  async setZoom(item: Item, zoom: number): Promise<void> {
    const props = { ...item.properties, zoom: String(zoom) };
    this.state.updateItem(item.id, { properties: props });
    try {
      await this.api.upsertItemProperty(item.id, 'zoom', 'number', String(zoom));
    } catch (e: any) {
      console.error('Failed to update zoom:', e?.message || e);
    }
  }

  /**
   * Collapses or expands an item with children. The screen changes at once and is put
   * back when the save fails.
   */
  async toggleCollapsed(item: Item): Promise<void> {
    if (!this.state.hasChildren(item.id)) return;
    try {
      const originalItem = { ...item };
      const newCollapsed = !item.collapsed;
      // Update local state immediately for responsive UI
      this.state.toggleCollapsed(item.id);
      this.state.updateItem(item.id, { collapsed: newCollapsed });
      // Persist to backend
      await this.api.updateItem(item.id, { collapsed: newCollapsed });

      this.undo.begin('collapse');
      this.undo.pushUpdate(originalItem, { ...originalItem, collapsed: newCollapsed });
      this.undo.commit();
    } catch (e) {
      console.error('Failed to toggle collapse:', e);
      // Revert on error
      this.state.toggleCollapsed(item.id);
      this.state.updateItem(item.id, { collapsed: item.collapsed });
    }
  }

  /**
   * Makes the item a transclusion of another item, when the pasted text is an item id.
   * @param item - The item that becomes the transclusion
   * @param pastedText - The clipboard text, expected to hold an item id
   * @param view - The outline, reloaded to get the transcluded tree
   */
  async transclude(item: Item, pastedText: string, view: OutlineView): Promise<void> {
    const sourceId = pastedText.trim();
    if (!sourceId || !/^[0-9A-Za-z_-]{10,}$/.test(sourceId)) return;
    try {
      const updated = await this.api.setTransclusion(item.id, sourceId);
      this.state.updateItem(item.id, updated);
      // Reload page to get transcluded trees
      await view.reloadPage();
    } catch (e: any) {
      console.error('Failed to set transclusion:', e?.message || e);
    }
  }

  /** Turns a transclusion back into a plain item. */
  async removeTransclusion(item: Item, view: OutlineView): Promise<void> {
    try {
      const updated = await this.api.removeTransclusion(item.id);
      this.state.updateItem(item.id, updated);
      // Reload page to refresh transcluded trees
      await view.reloadPage();
    } catch (e) {
      console.error('Failed to remove transclusion:', e);
    }
  }

  /**
   * Sends an item (and its children) to the page's archive (#item-archive).
   * Appends #archived tag to the item body and moves it under the #item-archive item.
   * If no #item-archive item exists, creates one at the end of the page.
   */
  async archive(item: Item): Promise<void> {
    try {
      const group = this.state.getItemWithChildren(item.id);
      if (group.length === 0) return;
      const groupIds = group.map(g => g.id);

      // Append #archived tag to the item body (skip if already present)
      if (!this.tagsOf(item).includes('archived')) {
        const newBody = item.body.trimEnd() + ' #archived';
        await this.edits.saveBody(item.id, newBody);
      }

      // Find or create the #item-archive item (identified by its tag).
      // Re-read items from state to pick up any updates from saveBody above
      let archiveItem = this.state.items().find(it => this.tagsOf(it).includes('item-archive'));
      if (!archiveItem) {
        // Create #item-archive at end of page
        const currentItems = this.state.items();
        const lastItem = currentItems[currentItems.length - 1];
        const created = await this.api.createItem(
          item.pageId,
          `after:${lastItem.id}`,
          1,
          '#item-archive',
        );
        archiveItem = created;
        this.state.addItem(archiveItem, `after:${lastItem.id}`);
      }

      // Move the archived items as the first child of #item-archive
      const levelDelta = (archiveItem.level + 1) - item.level;
      const moved = await this.api.moveItems(groupIds, item.pageId, `after:${archiveItem.id}`, levelDelta);
      this.state.replaceItems(groupIds, moved, `after:${archiveItem.id}`);
    } catch (e: any) {
      console.error('Failed to archive item:', e?.message || e);
    }
  }

  /**
   * Deletes items from the screen first and then from the server, as one undo entry.
   * @param itemIds - The items to delete, in page order
   */
  async deleteItems(itemIds: string[]): Promise<void> {
    if (itemIds.length === 0) return;

    // Snapshot before deletion
    const deletedItems = itemIds.map(id => this.state.getItemById(id)).filter((b): b is Item => !!b);

    this.undo.begin('delete-selection');
    for (const b of deletedItems) {
      this.undo.pushDelete({ ...b });
    }
    this.undo.commit();

    // Delete from state first for responsiveness
    this.state.deleteItems(itemIds);

    // Delete from backend
    try {
      await Promise.all(itemIds.map(id => this.api.deleteItem(id)));
    } catch (e) {
      console.error('Failed to delete selected items:', e);
    }
  }

  /**
   * Copies items to the system clipboard as tree markdown and HTML, and keeps the flat
   * markdown so that a paste back into this app keeps the structure.
   * @param itemIds - The items to copy, in page order
   */
  async copyItems(itemIds: string[]): Promise<void> {
    if (itemIds.length === 0) return;

    try {
      const result = await this.api.exportTreeMarkdown(itemIds);

      // Write to system clipboard
      const clipboardItem = new ClipboardItem({
        'text/plain': new Blob([result.treeMarkdown], { type: 'text/plain' }),
        'text/html': new Blob([renderToHtml(result.ast)], { type: 'text/html' }),
      });
      await navigator.clipboard.write([clipboardItem]);

      // Store flat markdown in memory
      this.clipboard.store(result.flatMarkdown, result.treeMarkdown);
    } catch (e) {
      console.error('Failed to copy selected items:', e);
    }
  }

  /** The hashtags of an item, as kept in its tag property. */
  private tagsOf(item: Item): string[] {
    const tags = item.properties['tag'];
    return Array.isArray(tags) ? tags : [];
  }
}
