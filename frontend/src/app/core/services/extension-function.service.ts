import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { ApiService } from './api.service';
import { OutlineStateService } from './outline-state.service';

interface FunctionInfo {
  name: string;
  url: string;
}

@Injectable({ providedIn: 'root' })
export class ExtensionFunctionService {
  private readonly http = inject(HttpClient);
  private readonly api = inject(ApiService);
  private readonly outlineState = inject(OutlineStateService);
  private iframes: HTMLIFrameElement[] = [];

  /**
   * Loads registered extension functions from the server and creates hidden iframes for each.
   * Also sets up a message listener to handle cross-origin messages from extensions.
   */
  async initialize(): Promise<void> {
    try {
      const functions = await firstValueFrom(
        this.http.get<FunctionInfo[]>('http://localhost:8080/api/functions')
      );
      for (const fn of functions) {
        const iframe = document.createElement('iframe');
        iframe.src = `http://localhost:8080${fn.url}`;
        iframe.style.display = 'none';
        document.body.appendChild(iframe);
        this.iframes.push(iframe);
      }
      window.addEventListener('message', (event) => this.onMessage(event));
    } catch (e) {
      console.warn('Failed to load extension functions:', e);
    }
  }

  /**
   * Broadcasts a `PageCreate` event to all loaded extension iframes.
   * @param event - The page creation event details.
   */
  emitPageCreate(event: { pageId: string; fileName: string; dirName: string; pageType: string }): void {
    const message = { type: 'PageCreate', ...event };
    for (const iframe of this.iframes) {
      iframe.contentWindow?.postMessage(message, '*');
    }
  }

  private async onMessage(event: MessageEvent): Promise<void> {
    const data = event.data;
    if (!data || typeof data !== 'object') return;

    if (data.type === 'CreateItem') {
      console.log('[ext-func] CreateItem received:', data.body?.substring(0, 50));
      await this.handleCreateItem(data, event.source as Window);
    }
  }

  private async handleCreateItem(
    data: { pageId: string; afterItemId: string; level: number; body: string; requestId?: string },
    source: Window | null
  ): Promise<void> {
    try {
      const { pageId, afterItemId, body } = data;
      let { level } = data;

      // Find the previous item to check canPlace
      const items = this.outlineState.items();
      let prevItem;
      if (afterItemId) {
        prevItem = items.find(b => b.id === afterItemId);
      } else {
        // afterItemId empty means append at end
        prevItem = items.length > 0 ? items[items.length - 1] : undefined;
      }

      // Adjust level with canPlace constraint
      while (level > 1 && !this.canPlace(level, prevItem)) {
        level--;
      }

      const newItem = await this.api.createItem(pageId, afterItemId || (prevItem?.id ?? ''), level, body);
      this.outlineState.addItem(newItem, afterItemId || (prevItem?.id ?? ''));

      // Send result back to iframe
      if (source) {
        source.postMessage({
          type: 'CreateItemResult',
          itemId: newItem.id,
          requestId: data.requestId
        }, '*');
      }
      console.log('[ext-func] Created item:', newItem.id, newItem.body?.substring(0, 50));
    } catch (e) {
      console.error('[ext-func] Failed to create item:', e);
      if (source) {
        source.postMessage({
          type: 'CreateItemError',
          error: String(e),
          requestId: data.requestId
        }, '*');
      }
    }
  }

  private canPlace(itemLevel: number, prevItem: { level: number } | undefined): boolean {
    if (!prevItem) return itemLevel === 1;
    return prevItem.level === itemLevel || prevItem.level === itemLevel - 1;
  }
}
