import { Injectable, inject, signal, effect, DestroyRef } from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { ApiService } from './api.service';
import { SseService } from './sse.service';
import { Page } from '../models/page.model';

@Injectable({ providedIn: 'root' })
export class RecentPagesService {
  private readonly api = inject(ApiService);
  private readonly sse = inject(SseService);
  private readonly destroyRef = inject(DestroyRef);
  private readonly _recentPages = signal<Page[]>([]);
  readonly recentPages = this._recentPages.asReadonly();

  constructor() {
    effect(() => {
      if (this.sse.isConnected()) {
        this.refresh();
      }
    });

    this.sse.pagesUpdated$.pipe(
      takeUntilDestroyed(this.destroyRef)
    ).subscribe(() => {
      this.refresh();
    });
  }

  async refresh(): Promise<void> {
    try {
      const result = await this.api.getRecentPages(10, 'doc');
      this._recentPages.set(result.pages);
    } catch {
      // SSE may not be ready yet
    }
  }
}
