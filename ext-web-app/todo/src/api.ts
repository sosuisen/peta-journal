import { petajournal } from './proto/bundle.js';

const { ApiRequest, ApiResponse } = petajournal;

type Block = petajournal.IBlock;

export type ConnectionState = 'connecting' | 'connected' | 'disconnected' | 'error';

let ws: WebSocket | null = null;
let requestId = 0;
const pending = new Map<string, { resolve: (r: petajournal.IApiResponse) => void; reject: (e: Error) => void }>();
let stateCallback: ((state: ConnectionState) => void) | null = null;

export function onConnectionStateChange(cb: (state: ConnectionState) => void): void {
  stateCallback = cb;
}

function getWsUrl(): string {
  const params = new URLSearchParams(window.location.search);
  const host = params.get('host') || 'localhost';
  const port = params.get('port') || '8080';
  return `ws://${host}:${port}/ws`;
}

export function connect(): Promise<void> {
  return new Promise((resolve, reject) => {
    if (ws && ws.readyState === WebSocket.OPEN) {
      resolve();
      return;
    }
    stateCallback?.('connecting');
    ws = new WebSocket(getWsUrl());
    ws.binaryType = 'arraybuffer';

    ws.onopen = () => { stateCallback?.('connected'); resolve(); };
    ws.onerror = (e) => { stateCallback?.('error'); reject(e); };

    ws.onmessage = (event) => {
      const data = new Uint8Array(event.data as ArrayBuffer);
      const response = ApiResponse.decode(data);
      const id = response.requestId;
      const p = pending.get(id);
      if (p) {
        pending.delete(id);
        p.resolve(response);
      }
    };

    ws.onclose = () => {
      ws = null;
      for (const [, p] of pending) {
        p.reject(new Error('WebSocket closed'));
      }
      pending.clear();
      stateCallback?.('disconnected');
    };
  });
}

function send(payload: Partial<petajournal.IApiRequest>): Promise<petajournal.IApiResponse> {
  return new Promise((resolve, reject) => {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      reject(new Error('WebSocket not connected'));
      return;
    }
    const id = String(++requestId);
    const request = ApiRequest.create({ requestId: id, ...payload });
    const bytes = ApiRequest.encode(request).finish();
    pending.set(id, { resolve, reject });
    ws.send(bytes);
  });
}

export async function getBlocksByStatus(statuses: string[]): Promise<Block[]> {
  const resp = await send({ getBlocksByStatus: { statuses } });
  if (!resp.success) throw new Error(resp.errorMessage || 'Request failed');
  return resp.getBlocksByStatus?.blocks || [];
}

export async function updateBlockStatus(blockId: string, status: string): Promise<Block> {
  const resp = await send({ updateBlockStatus: { blockId, status } });
  if (!resp.success) throw new Error(resp.errorMessage || 'Request failed');
  return resp.updateBlockStatus!.block!;
}

export type { Block };
