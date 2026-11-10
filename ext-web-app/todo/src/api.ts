export type ConnectionState = 'connecting' | 'connected' | 'disconnected';

export interface Item {
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
  ast: string;
  collapsed: boolean;
  types: string[];
  properties: Record<string, string | string[]>;
}

/**
 * The address of the journal server, from `?host=` and `?port=` of the page URL.
 *
 * The journal server serves the API and the PetaJournal web app at the same address
 * (ADR 010), so a link to a page starts with this address too.
 */
export function serverUrlFrom(search: string): string {
  const params = new URLSearchParams(search);
  const host = params.get('host') || 'localhost';
  const port = params.get('port') || '8087';
  return `http://${host}:${port}`;
}

export const serverUrl = serverUrlFrom(window.location.search);

const RETRY_MS = 3000;

export interface ConnectionListener {
  onStateChange(state: ConnectionState): void;
  /** A page changed on the server. */
  onPagesUpdated(): void;
}

/**
 * Connects to the event stream of the server, and connects again after an error, until the
 * returned function is called.
 */
export function subscribe(listener: ConnectionListener): () => void {
  let source: EventSource | undefined;
  let retry: ReturnType<typeof setTimeout> | undefined;

  const connect = () => {
    listener.onStateChange('connecting');
    source = new EventSource(`${serverUrl}/api/events`);
    source.onopen = () => listener.onStateChange('connected');
    source.onmessage = (event: MessageEvent<string>) => {
      if (isPagesUpdated(event.data)) listener.onPagesUpdated();
    };
    source.onerror = () => {
      // Without the close the browser would connect again by itself, at times of its own
      source?.close();
      listener.onStateChange('disconnected');
      retry = setTimeout(connect, RETRY_MS);
    };
  };

  connect();
  return () => {
    clearTimeout(retry);
    source?.close();
  };
}

function isPagesUpdated(data: string): boolean {
  try {
    return (JSON.parse(data) as { type?: unknown }).type === 'pagesUpdated';
  } catch {
    // The stream also carries data that is not for this app
    return false;
  }
}

/** The ID this tab names itself with. The server refuses a change without it (ADR 015). */
export const clientId = crypto.randomUUID();

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const resp = await fetch(`${serverUrl}${path}`, init);
  if (!resp.ok) {
    const text = await resp.text().catch(() => '');
    throw new Error(text || `HTTP ${resp.status}`);
  }
  return (await resp.json()) as T;
}

/** Fetches all items whose type matches one of the given types. */
export async function getItemsByTypes(types: string[]): Promise<Item[]> {
  const data = await fetchJson<{ items: Item[] }>(
    `/api/items?types=${types.join(',')}`,
  );
  return data.items;
}

/**
 * Fetches the tasks. Most tasks are done, so without the done tasks the server leaves them
 * out: a done task is a task with the property `end`.
 */
export async function getTasks(includeDone: boolean): Promise<Item[]> {
  const data = await fetchJson<{ items: Item[] }>(
    '/api/items?types=task&withoutProperty=end',
  );
  return data.items;
}

/**
 * The folder of the tag pages, from the configuration of the server. It is empty for the
 * pages root, and ends with `/` otherwise.
 */
export async function getTagsDir(): Promise<string> {
  const config = await fetchJson<{ tagsDir: string }>('/api/config');
  return config.tagsDir;
}

/** Cycles a task's state via the cycle-task endpoint. */
export async function cycleTask(itemId: string): Promise<Item> {
  const data = await fetchJson<{ item: Item }>(
    `/api/items/${itemId}/cycle-task`,
    {
      method: 'POST',
      headers: { 'X-Client-Id': clientId },
    },
  );
  return data.item;
}
