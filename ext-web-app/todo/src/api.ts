import { petajournal } from './proto/bundle.js';

const { ApiRequest, ApiResponse } = petajournal;

type Block = petajournal.IBlock;

export type ConnectionState = 'connecting' | 'connected' | 'disconnected' | 'error';

let ws: WebSocket | null = null;
let requestId = 0;
const pending = new Map<string, { resolve: (r: petajournal.IApiResponse) => void; reject: (e: Error) => void }>();
let stateCallback: ((state: ConnectionState) => void) | null = null;
let serverEventCallback: ((event: petajournal.IServerEvent) => void) | null = null;

/** Registers a callback invoked whenever the WebSocket connection state changes. */
export function onConnectionStateChange(cb: (state: ConnectionState) => void): void {
  stateCallback = cb;
}

/** Registers a callback invoked when the server pushes an event (e.g. page updates). */
export function onServerEvent(cb: (event: petajournal.IServerEvent) => void): void {
  serverEventCallback = cb;
}

function buildWebSocketUrl(): string {
  const params = new URLSearchParams(window.location.search);
  const host = params.get('host') || 'localhost';
  const port = params.get('port') || '8080';
  return `ws://${host}:${port}/ws`;
}

/** Opens a WebSocket connection to the server. Resolves when connected. */
export function connect(): Promise<void> {
  return new Promise((resolve, reject) => {
    if (ws && ws.readyState === WebSocket.OPEN) {
      resolve();
      return;
    }
    stateCallback?.('connecting');
    ws = new WebSocket(buildWebSocketUrl());
    ws.binaryType = 'arraybuffer';

    ws.onopen = () => { stateCallback?.('connected'); resolve(); };
    ws.onerror = (e) => { stateCallback?.('error'); reject(e); };

    ws.onmessage = (event) => {
      const data = new Uint8Array(event.data as ArrayBuffer);
      const response = ApiResponse.decode(data);

      // Handle server-pushed events (no requestId)
      if (!response.requestId && response.serverEvent) {
        serverEventCallback?.(response.serverEvent);
        return;
      }

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

function sendRequest(payload: Partial<petajournal.IApiRequest>): Promise<petajournal.IApiResponse> {
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

/** Fetches all blocks whose status matches one of the given statuses. */
export async function getBlocksByStatus(statuses: string[]): Promise<Block[]> {
  const resp = await sendRequest({ getBlocksByStatus: { statuses } });
  if (!resp.success) throw new Error(resp.errorMessage || 'Request failed');
  return resp.getBlocksByStatus?.blocks || [];
}

/** Updates a block's status and returns the updated block. */
export async function updateBlockStatus(blockId: string, status: string): Promise<Block> {
  const resp = await sendRequest({ updateBlockStatus: { blockId, status } });
  if (!resp.success) throw new Error(resp.errorMessage || 'Request failed');
  return resp.updateBlockStatus!.block!;
}

export type { Block };
