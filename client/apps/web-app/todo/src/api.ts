export type ConnectionState = 'connecting' | 'connected' | 'disconnected';

/**
 * A task as the app shows it. The server has more fields of an item, such as the Markdown
 * text. The app does not ask for them: it draws the text from `ast`.
 */
export interface Item {
  id: string;
  /** The type of the page of the item: `journal` or `doc` */
  type: string;
  dirPath: string;
  name: string;
  /** The URL of the item in the PetaJournal web app, from the root of the server */
  url: string;
  ast: string;
  properties: Record<string, string | string[]>;
}

/** The fields of `Item`, as the server is asked for them. */
const ITEM_FIELDS: readonly (keyof Item)[] = [
  'id',
  'type',
  'dirPath',
  'name',
  'url',
  'ast',
  'properties',
];

/**
 * The address of the journal server: the origin of the page, or another server named with
 * `?host=` and `?port=` of the page URL.
 *
 * The journal server serves this app at `/apps/todo/` (ADR 073), so by default the API is at
 * the origin of the page. It serves the PetaJournal web app at the same address (ADR 010),
 * so a link to a page starts with this address too.
 */
export function serverUrlFrom(
  page: Pick<Location, 'search' | 'origin'>,
): string {
  const params = new URLSearchParams(page.search);
  const host = params.get('host');
  const port = params.get('port');
  if (host === null && port === null) {
    return page.origin;
  }
  return `http://${host || 'localhost'}:${port || '8087'}`;
}

export const serverUrl = serverUrlFrom(window.location);

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

/**
 * Fetches the tasks. Most tasks are done, so without the done tasks the server leaves them
 * out: a done task is a task with the property `end`.
 */
export async function getTasks(includeDone: boolean): Promise<Item[]> {
  const query = includeDone ? 'types=task' : 'types=task&withoutProperty=end';
  const data = await fetchJson<{ items: Item[] }>(
    `/api/items?${query}&fields=${ITEM_FIELDS.join(',')}`,
  );
  return data.items;
}

/**
 * The values with which the links in a text are made, from the configuration of the server
 * (ADR 061). The templates have `{}` where the name goes.
 */
export interface LinkConfig {
  wikilinkHref: string;
  hashtagHref: string;
  pagesBase: string;
  wikilinkFragmentPattern: string;
}

/** The values for links, from the configuration of the server. */
export async function getLinkConfig(): Promise<LinkConfig> {
  const config = await fetchJson<LinkConfig>('/api/config');
  return {
    wikilinkHref: config.wikilinkHref,
    hashtagHref: config.hashtagHref,
    pagesBase: config.pagesBase,
    wikilinkFragmentPattern: config.wikilinkFragmentPattern,
  };
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
