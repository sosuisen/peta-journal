import { ApiService } from '../../core/services/api.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { ItemEditService } from './item-edit.service';

/**
 * Replaces the items of a page with the server's version after its markdown file changed
 * on disk. A page is not replaced while the user is editing, so their typing is not
 * overwritten; it is reloaded once editing stops. Each outline has its own instance,
 * because the pages waiting for a reload belong to that outline.
 */
export class FileReloadService {
  /** Pages changed on disk while the user was editing. */
  private readonly pending = new Set<string>();

  constructor(
    private readonly state: OutlineStateService,
    private readonly api: ApiService,
    private readonly edits: ItemEditService,
  ) {}

  /** True when a page waits for the user to stop editing. */
  hasPending(): boolean {
    return this.pending.size > 0;
  }

  /**
   * Reloads one page of the outline, or remembers it when the user is editing.
   * Pages that the outline does not show are ignored.
   */
  async reloadPage(pageId: string): Promise<void> {
    if (!this.state.items().some(i => i.pageId === pageId)) return;  // not shown in this outline
    if (this.edits.isEditInProgress()) {
      this.pending.add(pageId);
      return;
    }
    await this.replacePage(pageId);
  }

  /**
   * Replaces the items of one page with the server's version right now, in place, so the
   * other pages of a journal stay where they are. The transcluded trees are merged for the
   * same reason.
   */
  async replacePage(pageId: string): Promise<void> {
    try {
      const result = await this.api.getPageById(pageId);
      const items = this.state.items();
      const first = items.findIndex(i => i.pageId === pageId);
      const before = first < 0 ? items : items.slice(0, first);
      const after = first < 0 ? [] : items.slice(first).filter(i => i.pageId !== pageId);
      this.state.setItems([...before, ...result.items, ...after]);
      if (result.transcludedTrees) {
        this.state.mergeTranscludedTrees(result.transcludedTrees);
      }
    } catch (e) {
      console.error('Failed to reload page:', pageId, e);
    }
  }

  /** Reloads the pages that waited for the editing to stop; does nothing while it goes on. */
  flushPending(): void {
    if (this.edits.isEditInProgress()) return;
    const ids = [...this.pending];
    this.pending.clear();
    for (const id of ids) this.reloadPage(id);
  }
}
