import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

// The module reads the address of the server from the page URL when it is loaded
vi.hoisted(() => {
  vi.stubGlobal('window', {
    location: { search: '?host=journal.test&port=9000' },
  });
});

import {
  type ConnectionState,
  clientId,
  cycleTask,
  getItemsByTypes,
  getTagsDir,
  getTasks,
  serverUrl,
  serverUrlFrom,
  subscribe,
} from './api';

/** The event stream of the browser, driven by the test. */
class FakeEventSource {
  static opened: FakeEventSource[] = [];

  readonly url: string;
  closed = false;
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onerror: (() => void) | null = null;

  constructor(url: string) {
    this.url = url;
    FakeEventSource.opened.push(this);
  }

  close() {
    this.closed = true;
  }

  /** The stream the app opened last. */
  static get last(): FakeEventSource {
    return FakeEventSource.opened[FakeEventSource.opened.length - 1];
  }
}

// サーバーのアドレス
describe('the address of the journal server', () => {
  // 指定がなければ、`http://localhost:8087`である。
  test('is http://localhost:8087 by default', () => {
    expect(serverUrlFrom('')).toBe('http://localhost:8087');
  });

  // ページのURLの`host`と`port`で変えられる。片方だけでもよい。
  test('takes the host and the port of the page URL, or one of them', () => {
    expect(serverUrlFrom('?host=example.com&port=9000')).toBe(
      'http://example.com:9000',
    );
    expect(serverUrlFrom('?port=9000')).toBe('http://localhost:9000');
    expect(serverUrlFrom('?host=example.com')).toBe('http://example.com:8087');
  });

  // アプリは、開いたページのURLからアドレスを決める。
  test('comes from the URL of the page the app was opened with', () => {
    expect(serverUrl).toBe('http://journal.test:9000');
  });
});

// サーバーとの接続
describe('the connection to the journal server', () => {
  let states: ConnectionState[];
  let pagesUpdated: number;
  let unsubscribe: () => void;

  beforeEach(() => {
    vi.useFakeTimers();
    vi.stubGlobal('EventSource', FakeEventSource);
    FakeEventSource.opened = [];
    states = [];
    pagesUpdated = 0;
    unsubscribe = subscribe({
      onStateChange: state => states.push(state),
      onPagesUpdated: () => pagesUpdated++,
    });
  });

  afterEach(() => {
    unsubscribe();
    vi.useRealTimers();
  });

  // 購読すると、接続中になり、サーバーのイベントストリームにつなぐ。
  test('starts connecting to the event stream of the server', () => {
    expect(states).toEqual(['connecting']);
    expect(FakeEventSource.opened.map(source => source.url)).toEqual([
      'http://journal.test:9000/api/events',
    ]);
  });

  // ストリームが開くと、接続済みになる。
  test('is connected once the stream is open', () => {
    FakeEventSource.last.onopen?.();

    expect(states).toEqual(['connecting', 'connected']);
  });

  // ページの更新のイベントが届くと、更新が知らされる。
  test('tells of an update when a pagesUpdated event arrives', () => {
    FakeEventSource.last.onopen?.();

    FakeEventSource.last.onmessage?.({
      data: '{"type":"pagesUpdated","pageId":"page-1"}',
    });

    expect(pagesUpdated).toBe(1);
  });

  // ほかの種類のイベントと、JSONでないデータは無視される。
  test('ignores other events and data that is not JSON', () => {
    FakeEventSource.last.onopen?.();

    FakeEventSource.last.onmessage?.({ data: '{"type":"confirm"}' });
    FakeEventSource.last.onmessage?.({ data: 'not json' });

    expect(pagesUpdated).toBe(0);
    expect(states).toEqual(['connecting', 'connected']);
  });

  // エラーが起きると、切断になり、ストリームは閉じられる。
  test('is disconnected and closes the stream on an error', () => {
    const stream = FakeEventSource.last;
    stream.onopen?.();

    stream.onerror?.();

    expect(states).toEqual(['connecting', 'connected', 'disconnected']);
    expect(stream.closed).toBe(true);
  });

  // 切断の3秒後に、新しいストリームでつなぎ直す。
  test('connects again with a new stream 3 seconds after an error', () => {
    FakeEventSource.last.onerror?.();

    vi.advanceTimersByTime(2999);
    expect(FakeEventSource.opened).toHaveLength(1);

    vi.advanceTimersByTime(1);
    expect(FakeEventSource.opened).toHaveLength(2);
    expect(states).toEqual(['connecting', 'disconnected', 'connecting']);
  });

  // つなぎ直しに失敗しても、3秒ごとに試し続ける。
  test('keeps trying every 3 seconds while the server is away', () => {
    FakeEventSource.last.onerror?.();
    vi.advanceTimersByTime(3000);
    FakeEventSource.last.onerror?.();
    vi.advanceTimersByTime(3000);

    expect(FakeEventSource.opened).toHaveLength(3);
  });

  // 購読を解除すると、ストリームは閉じられる。
  test('closes the stream when unsubscribed', () => {
    const stream = FakeEventSource.last;
    stream.onopen?.();

    unsubscribe();

    expect(stream.closed).toBe(true);
  });

  // つなぎ直しを待つ間に購読を解除すると、つなぎ直さない。
  test('does not connect again when unsubscribed while waiting', () => {
    FakeEventSource.last.onerror?.();

    unsubscribe();
    vi.advanceTimersByTime(10_000);

    expect(FakeEventSource.opened).toHaveLength(1);
  });
});

// サーバーへのリクエスト
describe('the requests to the journal server', () => {
  const fetchMock = vi.fn<typeof fetch>();
  const answer = (body: string, status = 200) =>
    fetchMock.mockResolvedValue(new Response(body, { status }));

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
  });

  // タスクの項目は、種類を指定してサーバーから取る。
  test('get the items of the given types', async () => {
    answer('{"items":[{"id":"item-1"}]}');

    const items = await getItemsByTypes(['task', 'note']);

    expect(items).toEqual([{ id: 'item-1' }]);
    expect(fetchMock).toHaveBeenCalledWith(
      'http://journal.test:9000/api/items?types=task,note',
      undefined,
    );
  });

  // 終わったタスクを見せないときは、終わっていないタスクだけをサーバーから取る。タスクの大半は終わっているため。
  test('get only the tasks that are not done', async () => {
    answer('{"items":[{"id":"item-1"}]}');

    const items = await getTasks(false);

    expect(items).toEqual([{ id: 'item-1' }]);
    expect(fetchMock).toHaveBeenCalledWith(
      'http://journal.test:9000/api/items?types=task&withoutProperty=end',
      undefined,
    );
  });

  // 終わったタスクも見せるときは、すべてのタスクを取る。
  test('get all tasks when the done tasks are shown too', async () => {
    answer('{"items":[]}');

    await getTasks(true);

    expect(fetchMock).toHaveBeenCalledWith(
      'http://journal.test:9000/api/items?types=task',
      undefined,
    );
  });

  // タグのページを置くフォルダは、サーバーの設定から取る。
  test('get the folder of the tag pages from the configuration', async () => {
    answer('{"tagsDir":"tags/","homeLink":""}');

    const tagsDir = await getTagsDir();

    expect(tagsDir).toBe('tags/');
    expect(fetchMock).toHaveBeenCalledWith(
      'http://journal.test:9000/api/config',
      undefined,
    );
  });

  // タスクの状態は、自分のIDを名乗ったPOSTで次へ進め、進めたあとの項目が返る。
  test('move a task to its next state with a POST', async () => {
    answer('{"item":{"id":"item-1","properties":{"begin":"10:00"}}}');

    const item = await cycleTask('item-1');

    expect(item).toEqual({ id: 'item-1', properties: { begin: '10:00' } });
    expect(fetchMock).toHaveBeenCalledWith(
      'http://journal.test:9000/api/items/item-1/cycle-task',
      { method: 'POST', headers: { 'X-Client-Id': clientId } },
    );
  });

  // サーバーが失敗を返すと、その本文をメッセージにしたエラーになる。
  test('fail with the text of the answer when the server refuses', async () => {
    answer('no such item', 404);

    await expect(cycleTask('item-9')).rejects.toThrow('no such item');
  });

  // 失敗の本文が空なら、エラーのメッセージはHTTPのステータスである。
  test('fail with the HTTP status when the answer has no text', async () => {
    answer('', 500);

    await expect(cycleTask('item-9')).rejects.toThrow('HTTP 500');
  });
});
