import { Injectable, inject } from '@angular/core';
import { firstValueFrom, filter, map, timeout, share } from 'rxjs';
import { WebSocketService } from './websocket.service';
// @ts-ignore
import { petajournal } from '../proto/bundle.js';

@Injectable({ providedIn: 'root' })
export class ProtobufService {
  private readonly ws = inject(WebSocketService);
  private requestCounter = 0;

  readonly events$ = this.ws.messages$.pipe(
    map((buf: ArrayBuffer) => petajournal.ApiResponse.decode(new Uint8Array(buf))),
    filter((resp: InstanceType<typeof petajournal.ApiResponse>) => !resp.requestId && !!resp.serverEvent),
    map((resp: InstanceType<typeof petajournal.ApiResponse>) => resp.serverEvent!),
    share()
  );

  /**
   * Encodes and sends a protobuf API request over WebSocket, then waits for the matching response.
   * @param payload - The request payload fields (excluding `requestId`, which is auto-generated).
   * @returns The decoded API response cast to `T`.
   * @throws Error if the server returns a failure or the request times out.
   */
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
