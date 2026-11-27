import { act, fireEvent, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, test, vi } from 'vitest';
import {
  Heading,
  HeadingsPanel,
  IndexPagesView,
  IndexSort,
  Page,
  RecentPagesView,
  SidebarPage,
  type LoadState,
} from './border/peta-types';
import Sidebar from './Sidebar';
import { draw, shell } from './test/screens';

/** The panel whose reading fails, as a bug in the part that reads it would. */
const broken = vi.hoisted(() => ({ panel: null as string | null }));

vi.mock('./context', async original => {
  const context = await original<typeof import('./context')>();
  return {
    ...context,
    usePanel: (kind: Parameters<typeof context.usePanel>[0]) => {
      if (kind === broken.panel) {
        throw new Error(`${kind} is broken`);
      }
      return context.usePanel(kind);
    },
  };
});

afterEach(() => {
  localStorage.clear();
  broken.panel = null;
  vi.restoreAllMocks();
});

/** A page of the sidebar named `label`, at `path`. */
function page(
  id: string,
  label: string,
  changes: Partial<SidebarPage> = {},
): SidebarPage {
  const shown = new SidebarPage(
    new Page(id, '', '', label, 'doc', 0n, 0n, new Map()),
    label,
    false,
    false,
    null,
    `/pages/${label}`,
  );
  return Object.assign(shown, changes);
}

const loaded: LoadState = { kind: 'Loaded' };

/** What each list of the sidebar changes from the folder `work/`, with the page b open. */
interface Changes {
  readonly index?: Partial<IndexPagesView>;
  readonly headings?: Partial<HeadingsPanel & { headings: Heading[] }>;
  readonly recent?: Partial<RecentPagesView>;
}

/** The sidebar of the folder `work/`, with the page b open. Each test changes what it needs. */
function sidebar(changes: Changes = {}) {
  const index = new IndexPagesView(
    'work/',
    '/pages/work/index',
    new IndexSort({ kind: 'Name' }, { kind: 'Asc' }),
    '/pages/index',
    [page('a', 'plan'), page('b', 'notes')],
    'b',
    loaded,
  );
  const {
    headings = [new Heading('h1', 'Goals', 1), new Heading('h2', 'Steps', 2)],
    ...contents
  } = changes.headings ?? {};
  const recent = new RecentPagesView(
    [page('b', 'notes'), page('c', 'diary')],
    loaded,
  );
  const ui = shell();
  ui.wasm.push(
    [],
    [
      { kind: 'IndexPages', value: Object.assign(index, changes.index) },
      {
        kind: 'Headings',
        value: Object.assign(new HeadingsPanel(loaded), contents),
      },
      { kind: 'RecentPages', value: Object.assign(recent, changes.recent) },
    ],
    [{ kind: 'Reset', items: headings }],
  );
  return { ...ui, ...draw(ui, <Sidebar />) };
}

const loading = { state: { kind: 'Loading' } } as const;
const failed = { state: { kind: 'Failed' } } as const;

/** Whether a list of the sidebar is drawn as loading. */
function busy(container: HTMLElement): number {
  return container.querySelectorAll('[aria-busy="true"]').length;
}

const here = { kind: 'Here' } as const;

function section(title: string): HTMLElement {
  return screen.getByText(title).closest('.sidebar-section') as HTMLElement;
}

function names(within: HTMLElement): (string | null)[] {
  return [...within.querySelectorAll('.page-name')].map(e => e.textContent);
}

// 目次の節
describe('the index section', () => {
  // 目次の見出しはフォルダの名前で、押すとそのフォルダの目次を開く。
  test('its title is the folder, and opens the index of the folder', () => {
    const { wasm } = sidebar();

    fireEvent.click(screen.getByText('work/'), { ctrlKey: true });

    expect(wasm.events).toEqual([
      {
        kind: 'GoToPage',
        path: '/pages/work/index',
        open_in: { kind: 'NewView' },
      },
    ]);
  });

  // 一覧は、上のフォルダ（../）とページを出す。今のページに印がつく。押すと、そのページを開く。
  test('the list shows the folder above and the pages, marks the page open, and opens a page', () => {
    const { wasm } = sidebar();
    const list = section('work/');

    fireEvent.click(within(list).getByText('../'));
    fireEvent.click(within(list).getByText('plan'), { shiftKey: true });

    expect(names(list)).toEqual(['../', 'plan', 'notes']);
    expect(
      [...list.querySelectorAll('.sidebar-item.active')].map(
        e => e.textContent,
      ),
    ).toEqual(['notes']);
    expect(wasm.events).toEqual([
      { kind: 'GoToPage', path: '/pages/index', open_in: here },
      { kind: 'GoToPage', path: '/pages/plan', open_in: { kind: 'Browser' } },
    ]);
  });

  // 上のフォルダがなければ、../は出ない。
  test('without a folder above there is no ../', () => {
    sidebar({ index: { parent_path: null } });
    const withoutParent = names(section('work/'));

    expect(withoutParent).toEqual(['plan', 'notes']);
  });

  // 目次のページも上のフォルダもなければ、目次の節はない。
  test('without pages and a folder above there is no index section', () => {
    const { container } = sidebar({
      index: { pages: [], parent_path: null },
    });

    expect(container.querySelector('.index-header-title')).toBeNull();
  });

  // 上のフォルダだけでも、目次の節は出る。
  test('a folder above alone shows the index section', () => {
    sidebar({ index: { pages: [] } });

    expect(names(section('work/'))).toEqual(['../']);
  });
});

// 目次の並べ方
describe('the sort of the index', () => {
  function openMenu() {
    fireEvent.click(screen.getByTitle('Sort'));
    return document.querySelector('.index-sort-menu') as HTMLElement;
  }

  // 並べ方のメニューは、6つの並べ方を出し、今のものに印がつく。
  test('the menu shows the six sorts, and marks the one in use', () => {
    sidebar({
      index: { sort: new IndexSort({ kind: 'Modified' }, { kind: 'Desc' }) },
    });

    const menu = openMenu();

    expect(
      [...menu.querySelectorAll('.index-sort-item')].map(e => e.textContent),
    ).toEqual([
      'Name ↑',
      'Name ↓',
      'Modified ↑',
      '✓Modified ↓',
      'Published ↑',
      'Published ↓',
    ]);
    expect(
      [...menu.querySelectorAll('.index-sort-item.current')].map(
        e => e.textContent,
      ),
    ).toEqual(['✓Modified ↓']);
  });

  // 並べ方を選ぶと、コアに知らせ、ブリッジが読む形でブラウザに残す。メニューは閉じる。
  test('a chosen sort goes to the core and stays in the browser as the bridge reads it', () => {
    localStorage.setItem(
      'sidebar-index-sort',
      '{"":{"key":"name","order":"desc"}}',
    );
    const { wasm } = sidebar();

    fireEvent.click(within(openMenu()).getByText('Published ↓'));

    expect(wasm.events).toEqual([
      {
        kind: 'SetIndexSort',
        dir: 'work/',
        sort: { key: { kind: 'Published' }, order: { kind: 'Desc' } },
      },
    ]);
    expect(
      JSON.parse(localStorage.getItem('sidebar-index-sort') ?? ''),
    ).toEqual({
      '': { key: 'name', order: 'desc' },
      'work/': { key: 'published', order: 'desc' },
    });
    expect(document.querySelector('.index-sort-menu')).toBeNull();
  });

  // ブラウザに残した値が読めなくても、選んだ並べ方は残る。
  test('a stored value that cannot be read gives way to the chosen sort', () => {
    localStorage.setItem('sidebar-index-sort', 'not json');
    sidebar();

    fireEvent.click(within(openMenu()).getByText('Name ↓'));

    expect(
      JSON.parse(localStorage.getItem('sidebar-index-sort') ?? ''),
    ).toEqual({ 'work/': { key: 'name', order: 'desc' } });
  });

  // メニューは、もう一度押すか、外のクリックか、Escapeで閉じる。メニューの中のクリックでは閉じない。
  test('the menu closes on a second press, a click outside and Escape, not on a click inside', () => {
    sidebar();
    const menu = () => document.querySelector('.index-sort-menu');

    openMenu();
    fireEvent.click(menu() as Element);
    const stays = menu();
    fireEvent.click(screen.getByTitle('Sort'));
    const pressedAgain = menu();
    openMenu();
    fireEvent.click(document.body);
    const clickedOutside = menu();
    openMenu();
    fireEvent.keyDown(document, { key: 'Escape' });

    expect(stays).not.toBeNull();
    expect([pressedAgain, clickedOutside, menu()]).toEqual([null, null, null]);
  });
});

// 見出しの節
describe('the contents section', () => {
  // 見出しは段の印をつけて並び、押すとその項目へのスクロールをコアに頼む。
  test('the headings show their level, and ask the core to scroll to their item', () => {
    const { wasm, container } = sidebar();

    fireEvent.click(screen.getByText('Steps'));

    expect(
      [...container.querySelectorAll('.contents-item')].map(e => [
        e.className,
        e.textContent,
      ]),
    ).toEqual([
      ['contents-item contents-h1', 'Goals'],
      ['contents-item contents-h2', 'Steps'],
    ]);
    expect(wasm.events).toEqual([
      { kind: 'ScrollToItemRequested', item_id: 'h2' },
    ]);
  });

  // 文字の変わった見出しは、差分で届き、新しい文字で並ぶ。
  test('a heading whose text changed comes as a change, and shows its new text', () => {
    const { wasm, container } = sidebar();

    act(() =>
      wasm.push(
        [],
        [],
        [{ kind: 'Update', at: 1, item: new Heading('h2', 'Plans', 2) }],
      ),
    );

    expect(
      [...container.querySelectorAll('.contents-item')].map(e => e.textContent),
    ).toEqual(['Goals', 'Plans']);
  });

  // 見出しがなければ、節はない。
  test('without headings there is no section', () => {
    sidebar({ headings: { headings: [] } });

    expect(screen.queryByText('Contents')).toBeNull();
  });
});

// 最近のページの節
describe('the recent pages section', () => {
  // 押すとそのページを開く。今のページでも、印はつかない。
  test('a page opens on a click, and the page open is not marked', () => {
    const { wasm } = sidebar();
    const list = section('Recent Pages');

    fireEvent.click(within(list).getByText('diary'));

    expect(names(list)).toEqual(['notes', 'diary']);
    expect(list.querySelector('.active')).toBeNull();
    expect(wasm.events).toEqual([
      { kind: 'GoToPage', path: '/pages/diary', open_in: here },
    ]);
  });

  // フォルダ、公開したページ、ふつうのページで、アイコンが違う。フォルダの名前があれば、名前の前に出る。
  test('a folder, a published page and a plain page have their own icons, and a folder name comes first', () => {
    const { container } = sidebar({
      recent: {
        pages: [
          page('d', 'work', { is_dir: true }),
          page('e', 'post', { published: true, dir_label: 'blog/' }),
          page('f', 'memo'),
        ],
      },
    });
    const items = section('Recent Pages').querySelectorAll('.sidebar-item');

    expect(
      [...items].map(e => [
        e.className,
        e.querySelector('use')?.getAttribute('href'),
        e.querySelector('.page-label')?.textContent,
      ]),
    ).toEqual([
      ['sidebar-item recent-page-item folder-item', '#icon-folder', 'work'],
      ['sidebar-item recent-page-item', '#icon-page-filled', 'blog/post'],
      ['sidebar-item recent-page-item', '#icon-page', 'memo'],
    ]);
    expect(container.querySelector('.page-dir')?.textContent).toBe('blog/');
  });
});

// 節の開け閉め
describe('closing and opening a section', () => {
  // 節を閉じると中が消え、開くと戻る。閉じた節はブラウザに残り、次に描いても閉じている。
  test('a closed section hides its list until it opens, and stays closed the next time', () => {
    const { unmount } = sidebar();
    const toggle = () =>
      within(section('Recent Pages')).getByRole('button', {
        name: /Collapse|Expand/,
      });

    fireEvent.click(toggle());
    const closed = names(section('Recent Pages'));
    const label = toggle().getAttribute('aria-label');
    unmount();
    sidebar();
    const after = names(section('Recent Pages'));
    fireEvent.click(toggle());

    expect([closed, label, after]).toEqual([[], 'Expand', []]);
    expect(names(section('Recent Pages'))).toEqual(['notes', 'diary']);
    expect(JSON.parse(localStorage.getItem('sidebar-collapsed') ?? '')).toEqual(
      [],
    );
  });

  // 目次と見出しの節も、それぞれ閉じられる。ほかの節には響かない。
  test('the index and the contents close on their own', () => {
    sidebar();
    const close = (title: string) =>
      fireEvent.click(
        within(section(title)).getByRole('button', { name: 'Collapse' }),
      );

    close('work/');
    close('Contents');
    const labels = ['work/', 'Contents', 'Recent Pages'].map(title =>
      within(section(title))
        .getAllByRole('button', { name: /Collapse|Expand/ })
        .map(b => b.getAttribute('aria-label')),
    );

    expect(names(section('work/'))).toEqual([]);
    expect(section('Contents').querySelector('.contents-item')).toBeNull();
    expect(names(section('Recent Pages'))).toEqual(['notes', 'diary']);
    expect(labels).toEqual([['Expand'], ['Expand'], ['Collapse']]);
    expect(
      (
        JSON.parse(localStorage.getItem('sidebar-collapsed') ?? '') as string[]
      ).sort(),
    ).toEqual(['contents', 'index']);
  });

  // ブラウザに残した値が読めなければ、すべての節が開いている。
  test('all sections are open when the stored value cannot be read', () => {
    localStorage.setItem('sidebar-collapsed', 'not json');
    sidebar();

    expect(names(section('Recent Pages'))).toEqual(['notes', 'diary']);
  });
});

// ページの右クリック
describe('a right click on a page', () => {
  // 右クリックすると、その場所にエクスポートのフォルダを開くメニューが出る。押すとコアに頼み、メニューは閉じる。
  test('opens a menu that asks the core to open the export folder of the page', () => {
    const { wasm } = sidebar();

    const kept = !fireEvent.contextMenu(screen.getAllByText('diary')[0], {
      clientX: 30,
      clientY: 40,
    });
    const menu = document.querySelector('.sidebar-context-menu') as HTMLElement;
    const at = [menu.style.left, menu.style.top];
    fireEvent.click(within(menu).getByText('Open export folder'));

    expect([kept, at]).toEqual([true, ['30px', '40px']]);
    expect(wasm.events).toEqual([{ kind: 'OpenExportFolder', page_id: 'c' }]);
    expect(document.querySelector('.sidebar-context-menu')).toBeNull();
  });

  // メニューは、外のクリックとEscapeで閉じる。
  test('the menu closes on a click outside and on Escape', () => {
    sidebar();
    const open = () => fireEvent.contextMenu(screen.getAllByText('diary')[0]);
    const menu = () => document.querySelector('.sidebar-context-menu');

    open();
    fireEvent.click(document.body);
    const clicked = menu();
    open();
    fireEvent.keyDown(document, { key: 'Escape' });

    expect([clicked, menu()]).toEqual([null, null]);
  });
});

// 読み込み中と失敗の一覧
describe('the lists that are loading or failed', () => {
  // 読み込み中のフォルダのページは、見出しなしで読み込み中と描かれる。失敗すると、そう書かれる。
  test('the pages of a folder show as loading without a title, and tell when they failed', () => {
    const { container } = sidebar({ index: { ...loading, header: '' } });
    const whileLoading = [
      busy(container),
      container.querySelector('.index-header-title'),
    ];
    sidebar({ index: { ...failed, header: '' } });

    expect(whileLoading).toEqual([1, null]);
    expect(
      screen.queryByText('The folder could not be loaded.'),
    ).not.toBeNull();
  });

  // 読み込み中の目次は、見出しとともに読み込み中と描かれる。失敗すると、そう書かれる。
  test('the contents show as loading under their title, and tell when they failed', () => {
    const { container } = sidebar({ headings: { ...loading, headings: [] } });
    const whileLoading = busy(section('Contents'));
    container.remove();
    sidebar({ headings: { ...failed, headings: [] } });

    expect(whileLoading).toBe(1);
    expect(
      within(section('Contents')).queryByText(
        'The contents could not be loaded.',
      ),
    ).not.toBeNull();
  });

  // 読み込み中の最近のページは、見出しとともに読み込み中と描かれる。失敗すると、そう書かれる。
  test('the recent pages show as loading under their title, and tell when they failed', () => {
    const { container } = sidebar({ recent: { ...loading, pages: [] } });
    const whileLoading = busy(section('Recent Pages'));
    container.remove();
    sidebar({ recent: { ...failed, pages: [] } });

    expect(whileLoading).toBe(1);
    expect(
      within(section('Recent Pages')).queryByText(
        'The recent pages could not be loaded.',
      ),
    ).not.toBeNull();
  });

  // 読み込み済みの一覧は、読み込み中とは描かれない。
  test('loaded lists are not drawn as loading', () => {
    const { container } = sidebar();

    expect(busy(container)).toBe(0);
  });

  // 1つの一覧が描けなくても、ほかの一覧は描かれる。
  test('a list that cannot be drawn leaves the others', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    broken.panel = 'Headings';

    sidebar();

    expect(screen.getAllByRole('alert')).toHaveLength(1);
    expect(names(section('work/'))).toEqual(['../', 'plan', 'notes']);
    expect(names(section('Recent Pages'))).toEqual(['notes', 'diary']);
  });
});

// サイドバーの足元
describe('the footer of the sidebar', () => {
  // 設定のボタンは、設定の画面を開く。
  test('the settings button opens the settings', () => {
    const { wasm } = sidebar();

    fireEvent.click(screen.getByTitle('Settings'));

    expect(wasm.links).toEqual(['/settings']);
  });

  // コアがまだサイドバーを出していなくても、最近のページの節と足元はある。
  test('before the core gives a sidebar there are the recent pages and the footer', () => {
    draw(shell(), <Sidebar />);

    expect(screen.getByText('Recent Pages')).not.toBeNull();
    expect(screen.getByTitle('Settings')).not.toBeNull();
  });
});
