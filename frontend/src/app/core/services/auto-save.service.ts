import { Injectable, inject, OnDestroy } from '@angular/core';
import { ApiService } from './api.service';
import { Block } from '../models/block.model';

@Injectable({ providedIn: 'root' })
export class AutoSaveService implements OnDestroy {
  private readonly api = inject(ApiService);
  private pendingEdits = new Map<string, { body: string; timer: ReturnType<typeof setTimeout> }>();
  private readonly debounceMs = 300;

  scheduleBlockUpdate(blockId: string, body: string): void {
    const existing = this.pendingEdits.get(blockId);
    if (existing) clearTimeout(existing.timer);

    const timer = setTimeout(async () => {
      this.pendingEdits.delete(blockId);
      await this.api.updateBlock(blockId, { body });
    }, this.debounceMs);

    this.pendingEdits.set(blockId, { body, timer });
  }

  cancelBlock(blockId: string): void {
    const existing = this.pendingEdits.get(blockId);
    if (existing) {
      clearTimeout(existing.timer);
      this.pendingEdits.delete(blockId);
    }
  }

  async flushBlock(blockId: string): Promise<Block | null> {
    const existing = this.pendingEdits.get(blockId);
    if (existing) {
      clearTimeout(existing.timer);
      this.pendingEdits.delete(blockId);
      return await this.api.updateBlock(blockId, { body: existing.body });
    }
    return null;
  }

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
