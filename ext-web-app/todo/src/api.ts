export type ConnectionState = 'connecting' | 'connected' | 'disconnected' | 'error';

export interface Block {
  id: string;
  pageId: string;
  dirPath: string;
  name: string;
  type: string;
  createdDate: number;
  modifiedDate: number;
  idx: string;
  level: number;
  body: string;
  html: string;
  collapsed: boolean;
  status: string;
  types: string[];
}

let eventSource: EventSource | null = null;
let stateCallback: ((state: ConnectionState) => void) | null = null;
let serverEventCallback: ((data: { pageId: string }) => void) | null = null;

function buildBaseUrl(): string {
  const params = new URLSearchParams(window.location.search);
  const host = params.get('host') || 'localhost';
  const port = params.get('port') || '8080';
  return `http://${host}:${port}`;
}

/** Registers a callback invoked whenever the connection state changes. */
export function onConnectionStateChange(cb: (state: ConnectionState) => void): void {
  stateCallback = cb;
}

/** Registers a callback invoked when the server pushes a pagesUpdated event. */
export function onServerEvent(cb: (data: { pageId: string }) => void): void {
  serverEventCallback = cb;
}

/** Opens an SSE connection. Resolves when connected. */
export function connect(): Promise<void> {
  return new Promise((resolve, reject) => {
    if (eventSource) {
      resolve();
      return;
    }
    stateCallback?.('connecting');
    eventSource = new EventSource(`${buildBaseUrl()}/api/events`);

    eventSource.onopen = () => {
      stateCallback?.('connected');
      resolve();
    };

    eventSource.addEventListener('pagesUpdated', (event: MessageEvent) => {
      try {
        const data = JSON.parse(event.data);
        serverEventCallback?.(data);
      } catch {
        // ignore parse errors
      }
    });

    eventSource.onerror = () => {
      stateCallback?.('error');
      eventSource?.close();
      eventSource = null;
      stateCallback?.('disconnected');
      reject(new Error('SSE connection error'));
    };
  });
}

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const resp = await fetch(`${buildBaseUrl()}${path}`, init);
  if (!resp.ok) {
    const text = await resp.text().catch(() => '');
    throw new Error(text || `HTTP ${resp.status}`);
  }
  return await resp.json();
}

/** Fetches all blocks whose status matches one of the given statuses. */
export async function getBlocksByStatus(statuses: string[]): Promise<Block[]> {
  const data = await fetchJson<{ blocks: Block[] }>(
    `/api/blocks?statuses=${statuses.join(',')}`
  );
  return data.blocks;
}

/** Updates a block's status and returns the updated block. */
export async function updateBlockStatus(blockId: string, status: string): Promise<Block> {
  const data = await fetchJson<{ block: Block }>(`/api/blocks/${blockId}/status`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ status }),
  });
  return data.block;
}
