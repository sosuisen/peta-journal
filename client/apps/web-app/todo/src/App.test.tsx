// @vitest-environment jsdom
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import type { ConnectionListener, Item, LinkConfig } from './api';

const server = vi.hoisted(() => ({
  listener: undefined as ConnectionListener | undefined,
  getTasks: vi.fn<(includeDone: boolean) => Promise<Item[]>>(),
  getLinkConfig: vi.fn<() => Promise<LinkConfig>>(),
  cycleTask: vi.fn<(itemId: string) => Promise<Item>>(),
}));

// The app talks to the journal server only through this module
vi.mock('./api', () => ({
  serverUrl: 'http://journal.test',
  subscribe: (listener: ConnectionListener) => {
    server.listener = listener;
    return () => {};
  },
  getTasks: server.getTasks,
  getLinkConfig: server.getLinkConfig,
  cycleTask: server.cycleTask,
}));

import App from './App';

const LINKS: LinkConfig = {
  wikilinkHref: '/pages/{}',
  hashtagHref: '/pages/tags/{}',
  pagesBase: '/pages/',
  wikilinkFragmentPattern: '[0-9]{14}_[0-9A-Z]{7}',
};

/** The AST of a task whose text is `text`, with the nodes after it. */
function ast(text: string, ...more: object[]): string {
  return JSON.stringify({
    type: 'document',
    children: [
      { type: 'paragraph', children: [{ type: 'text', value: text }, ...more] },
    ],
  });
}

/** A task on the journal of 2026-01-05, changed by `overrides`. */
function task(overrides: Partial<Item> = {}): Item {
  return {
    id: 'item-1',
    type: 'journal',
    dirPath: '',
    name: '2026-01-05',
    url: '/pages/2026-01-05#item-1',
    ast: ast('Write the report'),
    properties: {},
    ...overrides,
  };
}

/**
 * Does what the server does, then waits until the app has drawn what follows. The answers of
 * the requests it starts are settled before this returns.
 */
async function fromServer(event: () => void) {
  await act(() => {
    event();
    return Promise.resolve();
  });
}

/** The server opens the event stream. */
async function connect() {
  await fromServer(() => server.listener?.onStateChange('connected'));
}

/** A promise and the function that settles it, for an answer the test hands over late. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(r => (resolve = r));
  return { promise, resolve };
}

beforeEach(() => {
  server.listener = undefined;
  server.getTasks.mockReset().mockResolvedValue([]);
  server.getLinkConfig.mockReset().mockResolvedValue(LINKS);
  server.cycleTask.mockReset();
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

// タスクを読む
describe('loading the tasks', () => {
  // 接続すると、終わっていないタスクだけを読む。
  test('asks for the tasks that are not done when it connects', async () => {
    render(<App />);
    expect(server.getTasks).not.toHaveBeenCalled();

    await connect();

    expect(server.getTasks).toHaveBeenCalledWith(false);
  });

  // タスクは開始日の順に、開始日の見出しの下に並ぶ。開始日のないタスクは最後に並ぶ。
  test('lists the tasks under the headings of their plan dates', async () => {
    server.getTasks.mockResolvedValue([
      task({ id: 'b', ast: ast('Later'), properties: { plan: '2026-01-07' } }),
      task({ id: 'c', ast: ast('Someday') }),
      task({ id: 'a', ast: ast('Sooner'), properties: { plan: '2026-01-05' } }),
    ]);
    render(<App />);

    await connect();

    await screen.findByText('Sooner');
    const list = document.querySelector('.item-list')!;
    expect(list.textContent).toMatch(
      /2026-01-05.*Sooner.*2026-01-07.*Later.*開始日なし.*Someday/,
    );
  });

  // タスクがなければ、そのことを出す。
  test('says so when there are no tasks', async () => {
    render(<App />);

    await connect();

    expect(await screen.findByText('No items found')).toBeTruthy();
  });

  // サーバーでページが変わると、タスクを読み直す。
  test('loads the tasks again when a page changes on the server', async () => {
    render(<App />);
    await connect();
    server.getTasks.mockResolvedValue([task()]);

    await fromServer(() => server.listener?.onPagesUpdated());

    expect(await screen.findByText('Write the report')).toBeTruthy();
    expect(server.getTasks).toHaveBeenCalledTimes(2);
  });

  // 読むのに失敗すると、エラーを出す。次に接続を試みると、エラーは消える。
  test('shows a failed load until the next try to connect', async () => {
    server.getTasks.mockRejectedValue(new Error('the server is down'));
    render(<App />);

    await connect();

    expect(await screen.findByText('Error: the server is down')).toBeTruthy();
    await fromServer(() => server.listener?.onStateChange('connecting'));
    expect(screen.queryByText('Error: the server is down')).toBeNull();
  });

  // 接続が切れたときは、エラーを消さない。
  test('keeps the error when the connection is lost', async () => {
    server.getTasks.mockRejectedValue(new Error('the server is down'));
    render(<App />);
    await connect();
    await screen.findByText('Error: the server is down');

    await fromServer(() => server.listener?.onStateChange('disconnected'));

    expect(screen.getByText('Error: the server is down')).toBeTruthy();
  });

  // リンクの設定を読むのに失敗すると、エラーを出す。
  test('shows a failed load of the link settings', async () => {
    server.getLinkConfig.mockRejectedValue(new Error('no config'));
    render(<App />);

    await connect();

    expect(await screen.findByText('Error: no config')).toBeTruthy();
  });

  // 絞り込みを続けて変えたとき、前の読み込みの答えが後から来ても、表には出ない。
  test('drops the answer of a load that a later filter made outdated', async () => {
    const first = deferred<Item[]>();
    server.getTasks
      .mockReturnValueOnce(first.promise)
      .mockResolvedValueOnce([task({ ast: ast('Fresh') })]);
    render(<App />);
    await connect();

    await userEvent.click(screen.getByLabelText('Done'));
    await screen.findByText('Fresh');
    await fromServer(() => first.resolve([task({ ast: ast('Stale') })]));

    expect(screen.queryByText('Stale')).toBeNull();
    expect(screen.getByText('Fresh')).toBeTruthy();
  });

  // 前の読み込みが失敗しても、後から来たその失敗は出ない。
  test('drops the failure of a load that a later filter made outdated', async () => {
    let fail!: (reason: Error) => void;
    server.getTasks
      .mockReturnValueOnce(new Promise<Item[]>((_, reject) => (fail = reject)))
      .mockResolvedValueOnce([task({ ast: ast('Fresh') })]);
    render(<App />);
    await connect();

    await userEvent.click(screen.getByLabelText('Done'));
    await screen.findByText('Fresh');
    await fromServer(() => fail(new Error('too late')));

    expect(screen.queryByText('Error: too late')).toBeNull();
  });
});

// 接続の状態
describe('the state of the connection', () => {
  // 下の欄に、接続の状態を出す。
  test('is shown at the bottom', async () => {
    render(<App />);
    expect(screen.getByText('disconnected')).toBeTruthy();

    await fromServer(() => server.listener?.onStateChange('connecting'));
    expect(screen.getByText('connecting')).toBeTruthy();

    await connect();
    expect(screen.getByText('connected')).toBeTruthy();
  });
});

// 絞り込み
describe('the filters', () => {
  // Doneにチェックすると、終わったタスクも読んで出す。
  test('load and show the done tasks with Done checked', async () => {
    const done = task({
      ast: ast('Finished'),
      properties: { end: '2026-01-05T10:00' },
    });
    server.getTasks.mockImplementation(includeDone =>
      Promise.resolve(includeDone ? [done] : []),
    );
    render(<App />);
    await connect();

    await userEvent.click(screen.getByLabelText('Done'));

    expect(await screen.findByText('Finished')).toBeTruthy();
    expect(server.getTasks).toHaveBeenLastCalledWith(true);
  });

  // 開始日を変えると、その日より後に始まるタスクは隠れる。
  test('hide the tasks that begin after the date', async () => {
    server.getTasks.mockResolvedValue([
      task({ id: 'a', ast: ast('Monday'), properties: { plan: '2026-01-05' } }),
      task({ id: 'b', ast: ast('Friday'), properties: { plan: '2026-01-09' } }),
    ]);
    render(<App />);
    await connect();
    await screen.findByText('Friday');

    fireEvent.change(screen.getByLabelText('開始日:'), {
      target: { value: '2026-01-06' },
    });

    expect(screen.getByText('Monday')).toBeTruthy();
    expect(screen.queryByText('Friday')).toBeNull();
  });

  // 日誌のタスクは最初から出ていて、Journalのチェックを外すと隠れる。
  test('show the tasks of the journals until Journal is unchecked', async () => {
    server.getTasks.mockResolvedValue([task({ ast: ast('From a journal') })]);
    render(<App />);
    await connect();
    await screen.findByText('From a journal');

    await userEvent.click(screen.getByLabelText('Journal'));

    expect(screen.queryByText('From a journal')).toBeNull();
  });

  // 文書のページのタスクは、Pageにチェックすると出る。
  test('show the tasks of the doc pages with Page checked', async () => {
    server.getTasks.mockResolvedValue([
      task({ type: 'doc', name: 'Notes', ast: ast('From a doc') }),
    ]);
    render(<App />);
    await connect();
    await screen.findByText('No items found');

    await userEvent.click(screen.getByLabelText('Page'));

    expect(screen.getByText('From a doc')).toBeTruthy();
  });
});

// タスクの行
describe('the row of a task', () => {
  // ページの名前と、開始日と期限を出す。
  test('shows the name of its page, its plan date and its due date', async () => {
    server.getTasks.mockResolvedValue([
      task({ properties: { plan: '2026-01-05', due: '2026-01-08' } }),
    ]);
    render(<App />);

    await connect();

    expect(
      await screen.findByText('2026-01-05', { selector: '.item-page' }),
    ).toBeTruthy();
    expect(screen.getByText('開始: 2026-01-05')).toBeTruthy();
    expect(screen.getByText('期限: 2026-01-08')).toBeTruthy();
  });

  // 行をクリックすると、PetaJournalで、そのタスクのページを開く。
  test('opens the page of the task in PetaJournal on a click', async () => {
    const open = vi.fn();
    vi.stubGlobal('open', open);
    server.getTasks.mockResolvedValue([task()]);
    render(<App />);
    await connect();

    await userEvent.click(await screen.findByText('Write the report'));

    expect(open).toHaveBeenCalledWith(
      'http://journal.test/pages/2026-01-05#item-1',
      'peta-journal',
    );
  });

  // 本文のタグは、タグのページへのリンクになる。クリックしても、タスクのページは開かない。
  test('links a tag in its text to the tag page, which a click follows alone', async () => {
    const open = vi.fn();
    vi.stubGlobal('open', open);
    server.getTasks.mockResolvedValue([
      task({ ast: ast('Read ', { type: 'hashtag', value: 'books' }) }),
    ]);
    render(<App />);
    await connect();

    const tag = await screen.findByRole('link', { name: /books/ });
    expect(tag.getAttribute('href')).toBe(
      'http://journal.test/pages/tags/books',
    );
    // jsdom cannot follow the link, so the default is stopped
    tag.addEventListener('click', event => event.preventDefault());
    await userEvent.click(tag);

    expect(open).not.toHaveBeenCalled();
  });

  // バッジをクリックすると、タスクの状態を進め、サーバーが返したタスクに替える。
  test('moves the task to its next state on a click on the badge', async () => {
    const open = vi.fn();
    vi.stubGlobal('open', open);
    server.getTasks.mockResolvedValue([task()]);
    server.cycleTask.mockResolvedValue(
      task({ properties: { begin: '2026-01-05T09:00' } }),
    );
    render(<App />);
    await connect();

    await userEvent.click(await screen.findByRole('button', { name: 'TODO' }));

    expect(server.cycleTask).toHaveBeenCalledWith('item-1');
    expect(await screen.findByRole('button', { name: 'RUNNING' })).toBeTruthy();
    expect(open).not.toHaveBeenCalled();
  });

  // 状態を進めるのに失敗すると、エラーを出し、タスクはそのまま残る。
  test('shows a failed change of the state and keeps the task', async () => {
    server.getTasks.mockResolvedValue([task()]);
    server.cycleTask.mockRejectedValue(new Error('not saved'));
    render(<App />);
    await connect();

    await userEvent.click(await screen.findByRole('button', { name: 'TODO' }));

    expect(await screen.findByText('Error: not saved')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'TODO' })).toBeTruthy();
  });
});
