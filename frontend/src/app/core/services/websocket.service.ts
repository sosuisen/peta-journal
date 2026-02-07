import { Injectable, signal, computed, OnDestroy } from '@angular/core';
import { Subject } from 'rxjs';

export type ConnectionState = 'connecting' | 'connected' | 'disconnected' | 'error';

@Injectable({ providedIn: 'root' })
export class WebSocketService implements OnDestroy {
  private ws: WebSocket | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private url = 'ws://localhost:8080/ws';

  private readonly _connectionState = signal<ConnectionState>('disconnected');
  readonly connectionState = this._connectionState.asReadonly();
  readonly isConnected = computed(() => this._connectionState() === 'connected');

  private readonly _messages$ = new Subject<ArrayBuffer>();
  readonly messages$ = this._messages$.asObservable();

  connect(): void {
    if (this.ws && (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)) {
      return;
    }
    this._connectionState.set('connecting');
    this.ws = new WebSocket(this.url);
    this.ws.binaryType = 'arraybuffer';

    this.ws.onopen = () => {
      this._connectionState.set('connected');
      this.reconnectAttempts = 0;
    };

    this.ws.onmessage = (event: MessageEvent) => {
      if (event.data instanceof ArrayBuffer) {
        this._messages$.next(event.data);
      }
    };

    this.ws.onerror = () => {
      this._connectionState.set('error');
    };

    this.ws.onclose = () => {
      this._connectionState.set('disconnected');
      this.ws = null;
      this.scheduleReconnect();
    };
  }

  send(data: Uint8Array): void {
    if (this.ws?.readyState !== WebSocket.OPEN) {
      throw new Error('WebSocket is not connected');
    }
    this.ws.send(data);
  }

  waitForConnection(timeoutMs = 10000): Promise<void> {
    if (this.ws?.readyState === WebSocket.OPEN) {
      return Promise.resolve();
    }
    return new Promise((resolve, reject) => {
      const start = Date.now();
      const check = () => {
        if (this.ws?.readyState === WebSocket.OPEN) {
          resolve();
        } else if (Date.now() - start > timeoutMs) {
          reject(new Error('WebSocket connection timeout'));
        } else {
          setTimeout(check, 50);
        }
      };
      check();
    });
  }

  disconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.reconnectAttempts = this.maxReconnectAttempts;
    this.ws?.close();
    this.ws = null;
    this._connectionState.set('disconnected');
  }

  private scheduleReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) return;
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
    this.reconnectAttempts++;
    this.reconnectTimer = setTimeout(() => this.connect(), delay);
  }

  ngOnDestroy(): void {
    this.disconnect();
    this._messages$.complete();
  }
}
