import { Injectable, signal, computed, OnDestroy } from '@angular/core';
import { Subject } from 'rxjs';

export type ConnectionState = 'connecting' | 'connected' | 'disconnected' | 'error';

@Injectable({ providedIn: 'root' })
export class SseService implements OnDestroy {
  private eventSource: EventSource | null = null;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  private readonly _connectionState = signal<ConnectionState>('disconnected');
  readonly connectionState = this._connectionState.asReadonly();
  readonly isConnected = computed(() => this._connectionState() === 'connected');

  private readonly _pagesUpdated$ = new Subject<{ pageId: string; source?: 'app' | 'file' }>();
  readonly pagesUpdated$ = this._pagesUpdated$.asObservable();

  connect(): void {
    if (this.eventSource) return;

    this._connectionState.set('connecting');
    this.eventSource = new EventSource('/api/events');

    this.eventSource.onopen = () => {
      this._connectionState.set('connected');
      this.reconnectAttempts = 0;
    };

    this.eventSource.onmessage = (event: MessageEvent) => {
      try {
        const data = JSON.parse(event.data);
        if (data.type === 'pagesUpdated') {
          this._pagesUpdated$.next(data);
        }
      } catch {
        // ignore parse errors
      }
    };

    this.eventSource.onerror = () => {
      this._connectionState.set('error');
      this.eventSource?.close();
      this.eventSource = null;
      this.scheduleReconnect();
    };
  }

  disconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.reconnectAttempts = this.maxReconnectAttempts;
    this.eventSource?.close();
    this.eventSource = null;
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
    this._pagesUpdated$.complete();
  }
}
