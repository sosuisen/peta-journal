import { Injectable, inject, OnDestroy } from '@angular/core';
import { ApiService } from './api.service';
import { Block } from '../models/block.model';

@Injectable({ providedIn: 'root' })
export class AutoSaveService implements OnDestroy {
  private readonly api = inject(ApiService);
  private pendingEdits = new Map<string, { body: string; timer: ReturnType<typeof setTimeout> }>();
  private readonly debounceMs = 300;

  /**
   * Schedules a debounced update for a block's body content.
   * Replaces any previously pending edit for the same block.
   * @param blockId - The ID of the block to update.
   * @param body - The new body content for the block.
   */
  scheduleBlockUpdate(blockId: string, body: string): void {
    const existing = this.pendingEdits.get(blockId);
    if (existing) clearTimeout(existing.timer);

    const timer = setTimeout(async () => {
      this.pendingEdits.delete(blockId);
      await this.api.updateBlock(blockId, { body });
    }, this.debounceMs);

    this.pendingEdits.set(blockId, { body, timer });
  }

  /**
   * Cancels a pending edit for a block without saving it.
   * @param blockId - The ID of the block whose pending edit should be cancelled.
   */
  cancelBlock(blockId: string): void {
    const existing = this.pendingEdits.get(blockId);
    if (existing) {
      clearTimeout(existing.timer);
      this.pendingEdits.delete(blockId);
    }
  }

  /**
   * Immediately saves a pending edit for a block, bypassing the debounce timer.
   * @param blockId - The ID of the block to flush.
   * @returns The updated block if there was a pending edit, or `null` otherwise.
   */
  async flushBlock(blockId: string): Promise<Block | null> {
    const existing = this.pendingEdits.get(blockId);
    if (existing) {
      clearTimeout(existing.timer);
      this.pendingEdits.delete(blockId);
      return await this.api.updateBlock(blockId, { body: existing.body });
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
    for (const [blockId, { body, timer }] of entries) {
      clearTimeout(timer);
      await this.api.updateBlock(blockId, { body });
    }
  }

  ngOnDestroy(): void {
    this.flushAll();
  }
}
