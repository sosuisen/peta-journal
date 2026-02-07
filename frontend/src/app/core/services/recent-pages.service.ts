import { Injectable, inject, signal, effect } from '@angular/core';
import { ApiService } from './api.service';
import { WebSocketService } from './websocket.service';
import { Page } from '../models/page.model';

@Injectable({ providedIn: 'root' })
export class RecentPagesService {
  private readonly api = inject(ApiService);
  private readonly ws = inject(WebSocketService);
  private readonly _recentPages = signal<Page[]>([]);
  readonly recentPages = this._recentPages.asReadonly();

  constructor() {
    effect(() => {
      if (this.ws.isConnected()) {
        this.refresh();
      }
    });
  }

  async refresh(): Promise<void> {
    try {
      const result = await this.api.getRecentPages(20);
      this._recentPages.set(result.pages.filter(p => !p.dirPath.startsWith('journals/')).slice(0, 10));
    } catch {
      // WebSocket may not be ready yet
    }
  }
}
