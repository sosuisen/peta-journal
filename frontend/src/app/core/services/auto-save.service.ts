import { Injectable, inject, OnDestroy } from '@angular/core';
import { ApiService } from './api.service';
import { Item } from '../models/item.model';
import { ensureCodeFenceClosed } from '../../features/outline/item-editor';

@Injectable({ providedIn: 'root' })
export class AutoSaveService implements OnDestroy {
  private readonly api = inject(ApiService);
  private pendingEdits = new Map<string, { body: string; timer: ReturnType<typeof setTimeout> }>();
  private readonly debounceMs = 300;

  /**
   * Schedules a debounced update for a item's body content.
   * Replaces any previously pending edit for the same item.
   * @param itemId - The ID of the item to update.
   * @param body - The new body content for the item.
   */
  scheduleItemUpdate(itemId: string, body: string): void {
    const existing = this.pendingEdits.get(itemId);
    if (existing) clearTimeout(existing.timer);

    const timer = setTimeout(async () => {
      this.pendingEdits.delete(itemId);
      await this.api.updateItem(itemId, { body: ensureCodeFenceClosed(body) });
    }, this.debounceMs);

    this.pendingEdits.set(itemId, { body, timer });
  }

  /**
   * Cancels a pending edit for a item without saving it.
   * @param itemId - The ID of the item whose pending edit should be cancelled.
   */
  cancelItem(itemId: string): void {
    const existing = this.pendingEdits.get(itemId);
    if (existing) {
      clearTimeout(existing.timer);
      this.pendingEdits.delete(itemId);
    }
  }

  /**
   * Immediately saves a pending edit for a item, bypassing the debounce timer.
   * @param itemId - The ID of the item to flush.
   * @returns The updated item if there was a pending edit, or `null` otherwise.
   */
  async flushItem(itemId: string): Promise<Item | null> {
    const existing = this.pendingEdits.get(itemId);
    if (existing) {
      clearTimeout(existing.timer);
      this.pendingEdits.delete(itemId);
      return await this.api.updateItem(itemId, { body: ensureCodeFenceClosed(existing.body) });
    }
    return null;
  }

  /**
   * Immediately saves all pending edits, clearing the internal queue.
   * @returns A promise that resolves when all pending edits have been saved.
   */
  async flushAll(): Promise<void> {
    const entries = [...this.pendingEdits.entries()];
    this.pendingEdits.clear();
    for (const [itemId, { body, timer }] of entries) {
      clearTimeout(timer);
      await this.api.updateItem(itemId, { body });
    }
  }

  ngOnDestroy(): void {
    this.flushAll();
  }
}
