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
  emitPageCreate(event: { pageId: string; fileName: string; dirName: string }): void {
    const message = { type: 'PageCreate', ...event };
    for (const iframe of this.iframes) {
      iframe.contentWindow?.postMessage(message, '*');
    }
  }

  private async onMessage(event: MessageEvent): Promise<void> {
    const data = event.data;
    if (!data || typeof data !== 'object') return;

    if (data.type === 'CreateBlock') {
      await this.handleCreateBlock(data, event.source as Window);
    }
  }

  private async handleCreateBlock(
    data: { pageId: string; afterBlockId: string; level: number; body: string; status?: string; requestId?: string },
    source: Window | null
  ): Promise<void> {
    try {
      const { pageId, afterBlockId, body } = data;
      let { level } = data;

      // Find the previous block to check canPlace
      const blocks = this.outlineState.blocks();
      let prevBlock;
      if (afterBlockId) {
        prevBlock = blocks.find(b => b.id === afterBlockId);
      } else {
        // afterBlockId empty means append at end
        prevBlock = blocks.length > 0 ? blocks[blocks.length - 1] : undefined;
      }

      // Adjust level with canPlace constraint
      while (level > 1 && !this.canPlace(level, prevBlock)) {
        level--;
      }

      const newBlock = await this.api.createBlock(pageId, afterBlockId || (prevBlock?.id ?? ''), level, body);
      this.outlineState.addBlock(newBlock, afterBlockId || (prevBlock?.id ?? ''));

      // Set status if provided
      if (data.status && data.status !== 'normal') {
        const updated = await this.api.updateBlockStatus(newBlock.id, data.status);
        this.outlineState.updateBlock(newBlock.id, { status: updated.status });
      }

      // Send result back to iframe
      if (source) {
        source.postMessage({
          type: 'CreateBlockResult',
          blockId: newBlock.id,
          requestId: data.requestId
        }, '*');
      }
    } catch (e) {
      console.error('Failed to create block from extension:', e);
      if (source) {
        source.postMessage({
          type: 'CreateBlockError',
          error: String(e),
          requestId: data.requestId
        }, '*');
      }
    }
  }

  private canPlace(blockLevel: number, prevBlock: { level: number } | undefined): boolean {
    if (!prevBlock) return blockLevel === 1;
    return prevBlock.level === blockLevel || prevBlock.level === blockLevel - 1;
  }
}
