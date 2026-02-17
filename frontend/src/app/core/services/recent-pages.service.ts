import { Injectable, inject, signal, effect, DestroyRef } from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { ApiService } from './api.service';
import { WebSocketService } from './websocket.service';
import { ProtobufService } from './protobuf.service';
import { Page } from '../models/page.model';

@Injectable({ providedIn: 'root' })
export class RecentPagesService {
  private readonly api = inject(ApiService);
  private readonly ws = inject(WebSocketService);
  private readonly proto = inject(ProtobufService);
  private readonly destroyRef = inject(DestroyRef);
  private readonly _recentPages = signal<Page[]>([]);
  readonly recentPages = this._recentPages.asReadonly();

  constructor() {
    effect(() => {
      if (this.ws.isConnected()) {
        this.refresh();
      }
    });

    this.proto.events$.pipe(
      takeUntilDestroyed(this.destroyRef)
    ).subscribe(event => {
      if (event.pagesUpdated) {
        this.refresh();
      }
    });
  }

  async refresh(): Promise<void> {
    try {
      const result = await this.api.getRecentPages(10, 'doc');
      this._recentPages.set(result.pages);
    } catch {
      // WebSocket may not be ready yet
    }
  }
}
