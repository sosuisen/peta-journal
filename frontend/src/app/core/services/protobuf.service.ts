import { Injectable, inject } from '@angular/core';
import { firstValueFrom, filter, map, timeout } from 'rxjs';
import { WebSocketService } from './websocket.service';
// @ts-ignore
import { petajournal } from '../proto/bundle.js';

@Injectable({ providedIn: 'root' })
export class ProtobufService {
  private readonly ws = inject(WebSocketService);
  private requestCounter = 0;

  async send<T>(payload: Record<string, unknown>): Promise<T> {
    const requestId = `req-${++this.requestCounter}-${Date.now()}`;
    const request = petajournal.ApiRequest.create({ requestId, ...payload });
    const encoded = petajournal.ApiRequest.encode(request).finish();

    await this.ws.waitForConnection();
    this.ws.send(encoded);

    return firstValueFrom(
      this.ws.messages$.pipe(
        map((buf: ArrayBuffer) => petajournal.ApiResponse.decode(new Uint8Array(buf))),
        filter((resp: InstanceType<typeof petajournal.ApiResponse>) => resp.requestId === requestId),
        timeout(15000),
        map((resp: InstanceType<typeof petajournal.ApiResponse>) => {
          if (!resp.success) {
            throw new Error(resp.errorMessage || 'Request failed');
          }
          return resp as T;
        })
      )
    );
  }
}
