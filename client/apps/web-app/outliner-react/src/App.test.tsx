import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { StrictMode } from 'react';
import { afterEach, describe, expect, test, vi } from 'vitest';
import App from './App';
import {
  BacklinkGroup,
  BacklinkItem,
  CalendarDay,
  CalendarView,
  CalendarWeek,
  ConfigView,
  DirSegment,
  JournalView,
  Page,
  PageView,
  SearchView,
  type JournalType,
  type Panel,
} from './border/peta-types';
import { AskedOpsContext } from './askedContext';
import { CoreContext } from './context';
import { draw, outline, shell } from './test/screens';

afterEach(() => {
  vi.useRealTimers();
  localStorage.clear();
});

/** A document page named `title` in the folder `dirPath`. Each test changes what it needs. */
function docPage(
  title: string,
  dirPath = '',
  changes: Partial<PageView> = {},
): PageView {
  const page = new Page('p', '', dirPath, title, 'doc', 0n, 0n, new Map());
  const view = new PageView(
    page,
    title,
    [],
    false,
    false,
    false,
    false,
    '',
    null,
    [],
    [],
    false,
    [],
  );
  return Object.assign(view, changes);
}

function journal(
  kind: JournalType['kind'] | null,
  pageId: string | null = 'j',
) {
  return new JournalView(
    '2026-11-23',
    kind === null ? null : { kind },
    '2026-11-23 (Mon)',
    pageId,
  );
}

/** A month of one day, the 23rd, which is both today and the day of the journal. */
const NOVEMBER: Panel = {
  kind: 'Calendar',
  value: new CalendarView(
    2026,
    11,
    'November',
    '2026-11',
    '2026',
    [],
    [
      new CalendarWeek(48, '2026-W48', [
        new CalendarDay('2026-11-23', 23, false, true, true),
      ]),
    ],
  ),
};

/** Draws the app on a core whose start answers `panels`. */
function app(...panels: Panel[]) {
  const ui = shell();
  ui.wasm.answerNext([], panels);
  const drawn = draw(ui, <App />);
  const show = (...more: Panel[]) => act(() => ui.wasm.push([], more));
  return { ...ui, ...drawn, show };
}

/** The heading of the page on the screen, not of the sidebar. */
function pageHeading(): string | null {
  return within(document.querySelector('main') as HTMLElement).getByRole(
    'heading',
  ).textContent;
}

function button(name: string | RegExp): HTMLElement {
  return screen.getByRole('button', { name });
}

// アプリの枠
describe('the frame of the app', () => {
  // 枠を描くと、コアが始まり、開いたページを描く。
  test('the core starts when the frame is drawn, and the page it opened shows', () => {
    app({
      kind: 'Screen',
      value: { kind: 'Journal', value: journal('Daily') },
    });

    expect(pageHeading()).toBe('2026-11-23 (Mon)');
  });

  // 上の端のボタンは、それぞれの移動をコアに頼む。
  test('the buttons of the header ask the core to go', () => {
    const { wasm } = app();

    for (const name of ['Root', 'Tags', 'Today', 'Home (Ctrl+Alt+Enter)']) {
      fireEvent.click(button(name));
    }

    expect(wasm.events).toEqual([
      { kind: 'NavigateToRoot' },
      { kind: 'NavigateToTags' },
      { kind: 'NavigateToToday' },
      { kind: 'NavigateHome' },
    ]);
  });

  // 新しいページは、検索欄の文字を名前の候補にして作る。検索欄がなければ、空の名前である。
  test('a new page takes the text of the search as its name, or none without it', () => {
    const { wasm, show } = app();
    fireEvent.click(button('+'));
    show({
      kind: 'Search',
      value: new SearchView('idea', [], false, null, false),
    });
    fireEvent.click(button('+'));

    expect(wasm.events).toEqual([
      { kind: 'CreateNewPage', search_query: '' },
      { kind: 'CreateNewPage', search_query: 'idea' },
    ]);
  });

  // 上の端の中ほどに、新しいページのボタンと検索欄がある。
  test('the middle of the header holds the new page button and the search', () => {
    const { container } = app();

    expect(
      [...(container.querySelector('.header-center')?.children ?? [])].map(e =>
        e.tagName.toLowerCase(),
      ),
    ).toEqual(['button', 'app-search-bar']);
  });

  // 横の欄にはサイドバーがある。サイドバーのアイコンは、枠が持つ絵を使う。
  test('the side column holds the sidebar, whose icons the frame draws', () => {
    const { container } = app();

    expect(container.querySelector('aside app-sidebar')).not.toBeNull();
    expect([...container.querySelectorAll('symbol')].map(s => s.id)).toEqual([
      'icon-page',
      'icon-page-filled',
      'icon-folder',
      'icon-item',
    ]);
  });

  // メニューのボタンは、横の欄を閉じたり開いたりする。
  test('the menu button closes and opens the sidebar', () => {
    const { container } = app();
    const sidebar = () => container.querySelector('aside') as HTMLElement;

    const layout = () => container.querySelector('.layout')?.className;

    fireEvent.click(button('☰'));
    const closed = [sidebar().className, sidebar().style.width, layout()];
    fireEvent.click(button('☰'));

    expect(closed).toEqual([
      'sidebar collapsed',
      '0px',
      'layout sidebar-collapsed',
    ]);
    expect([sidebar().className, sidebar().style.width]).toEqual([
      'sidebar',
      '260px',
    ]);
    expect(container.querySelector('.layout')?.className).toBe('layout');
  });

  // 横の欄は、ブラウザに残した幅で始まる。残した幅がないか読めなければ、260pxで始まる。閉じた横の欄には、幅を変えるつまみがない。
  test('the sidebar starts at the width kept in the browser, or 260px, and a closed one has no handle', () => {
    localStorage.setItem(
      'peta-journal-config',
      JSON.stringify({ sidebarWidth: 320 }),
    );
    const kept = app();
    const width = () =>
      (kept.container.querySelector('aside') as HTMLElement).style.width;
    const keptWidth = width();
    fireEvent.click(button('☰'));
    const handle = kept.container.querySelector('.resize-handle');
    kept.unmount();
    localStorage.setItem('peta-journal-config', '{');
    const unreadable = app();

    expect([keptWidth, handle]).toEqual(['320px', null]);
    expect(
      (unreadable.container.querySelector('aside') as HTMLElement).style.width,
    ).toBe('260px');
  });

  // つまみを引くと、マウスが動いた分だけ横の欄の幅が変わる。幅は200pxから600pxの間である。変えた幅は、ほかの設定を残したままブラウザに残す。放したあとの動きでは変わらない。
  test('dragging the handle changes the width by the move, from 200px to 600px, and keeps it', () => {
    localStorage.setItem(
      'peta-journal-config',
      JSON.stringify({ other: true }),
    );
    const { container } = app();
    const width = () =>
      (container.querySelector('aside') as HTMLElement).style.width;
    const handle = container.querySelector('.resize-handle') as Element;

    const pressed = fireEvent.mouseDown(handle, { clientX: 100 });
    fireEvent.mouseMove(document, { clientX: 150 });
    const wider = width();
    fireEvent.mouseMove(document, { clientX: 1000 });
    const widest = width();
    fireEvent.mouseMove(document, { clientX: 0 });
    const narrowest = width();
    fireEvent.mouseMove(document, { clientX: 120 });
    fireEvent.mouseUp(document);
    fireEvent.mouseMove(document, { clientX: 300 });

    expect(pressed).toBe(false);
    expect([wider, widest, narrowest, width()]).toEqual([
      '310px',
      '600px',
      '200px',
      '280px',
    ]);
    expect(
      JSON.parse(localStorage.getItem('peta-journal-config') ?? ''),
    ).toEqual({ other: true, sidebarWidth: 280 });
  });

  // 下の端は、接続の状態と、フォーカスした項目を出す。状態がまだなければ、接続中である。
  test('the footer shows the connection and the focused item', () => {
    const { container, show } = app();
    const status = () => container.querySelector('.connection-status');
    const before = [status()?.className, status()?.textContent];
    const noItem = container.querySelector('.item-id');

    show(
      { kind: 'Connection', value: { kind: 'Connected' } },
      { kind: 'Outline', value: outline({ focused_item_id: 'a' }) },
    );

    expect(before).toEqual(['connection-status connecting', 'connecting']);
    expect(noItem).toBeNull();
    expect(status()?.textContent).toBe('connected');
    expect(container.querySelector('.item-id')?.textContent).toBe('Item: a');
  });
});

// ページのメニュー
describe('the page menu', () => {
  // メニューは押すと開き、項目を選ぶと閉じる。ページの削除をコアに頼む。
  test('the menu opens on a press, closes on a choice, and asks the core to delete the page', () => {
    const { wasm } = app();

    fireEvent.click(button('⋮'));
    fireEvent.click(button('Delete page'));

    expect(screen.queryByText('Delete page')).toBeNull();
    expect(wasm.events).toEqual([{ kind: 'DeleteCurrentPage' }]);
  });

  // もう一度押すと、閉じる。
  test('a second press closes it', () => {
    app();

    fireEvent.click(button('⋮'));
    fireEvent.click(button('⋮'));

    expect(screen.queryByText('Delete page')).toBeNull();
  });

  // 文書のページでは、プロパティの編集もある。日誌には、ない。
  test('a document page can also edit its properties, and a journal cannot', () => {
    const { wasm, show } = app({
      kind: 'Screen',
      value: { kind: 'Journal', value: journal('Daily') },
    });
    fireEvent.click(button('⋮'));
    const inJournal = screen.queryByText('Edit properties');
    show({ kind: 'Screen', value: { kind: 'Page', value: docPage('notes') } });

    fireEvent.click(button('Edit properties'));

    expect(inJournal).toBeNull();
    expect(wasm.events).toEqual([{ kind: 'EditPageProperties' }]);
  });

  // 開けなかったページにも、プロパティの編集はない。
  test('a page that did not open cannot edit its properties', () => {
    app({ kind: 'Screen', value: { kind: 'Page', value: null } });

    fireEvent.click(button('⋮'));

    expect(screen.queryByText('Edit properties')).toBeNull();
  });
});

// 設定の画面
describe('the settings screen', () => {
  // 画面が設定のとき、設定の画面を出す。見つからないページの画面には、設定が届いていても何も出さない。
  test('the settings show when the screen is the settings, and not for a missing page', () => {
    const { container, show } = app(
      { kind: 'Screen', value: { kind: 'Settings' } },
      {
        kind: 'Config',
        value: new ConfigView('', { kind: 'Us' }, false, '', '', [], null, 0),
      },
    );
    const settings = () =>
      container.querySelector('app-page-dispatcher > app-settings');
    const shown = settings();

    show({ kind: 'Screen', value: { kind: 'Page', value: null } });

    expect(shown).not.toBeNull();
    expect(
      container.querySelector('app-page-dispatcher')?.children,
    ).toHaveLength(0);
  });
});

// 日誌
describe('a journal', () => {
  // 日と週の日誌には、前と次への移動がある。
  test('a daily or weekly journal moves to the one before and after', () => {
    for (const kind of ['Daily', 'Weekly'] as const) {
      const { wasm, unmount } = app({
        kind: 'Screen',
        value: { kind: 'Journal', value: journal(kind) },
      });

      fireEvent.click(button('<'));
      fireEvent.click(button('>'));

      expect(wasm.events).toEqual([
        { kind: 'JournalNav', value: { kind: 'Prev' } },
        { kind: 'JournalNav', value: { kind: 'Next' } },
      ]);
      unmount();
    }
  });

  // 日付かカレンダーのボタンを押すと、見出しの下にカレンダーが開く。開くときは、今日の月を出すようコアに頼む。もう一度押すと閉じる。
  test('the date or the calendar button opens a calendar under the header at the month of today, and closes it', () => {
    const { container, wasm } = app(NOVEMBER, {
      kind: 'Screen',
      value: { kind: 'Journal', value: journal('Daily') },
    });
    const popup = () => container.querySelector('app-journal app-calendar');
    const before = popup();

    fireEvent.click(screen.getByText('2026-11-23 (Mon)'));
    const opened = popup()?.previousElementSibling?.className;
    fireEvent.click(container.querySelector('.calendar-btn') as Element);
    const closed = popup();
    fireEvent.click(container.querySelector('.calendar-btn') as Element);

    expect([before, closed]).toEqual([null, null]);
    expect(opened).toBe('journal-header');
    expect(popup()?.querySelector('.selected')?.textContent).toBe('23');
    expect(wasm.events).toEqual([
      { kind: 'CalendarNav', value: { kind: 'Today' } },
      { kind: 'CalendarNav', value: { kind: 'Today' } },
    ]);
  });

  // カレンダーでジャーナルを開くと閉じる。ほかのタブか窓で開いたときは、開いたままである。
  test('opening a journal from the calendar closes it, unless it opened in another tab or window', () => {
    const { container } = app(NOVEMBER, {
      kind: 'Screen',
      value: { kind: 'Journal', value: journal('Daily') },
    });
    const popup = () => container.querySelector('app-journal app-calendar');
    const day = () => within(popup() as HTMLElement).getByText('23');

    fireEvent.click(container.querySelector('.calendar-btn') as Element);
    fireEvent.click(day(), { ctrlKey: true });
    const kept = popup();
    fireEvent.click(day());

    expect(kept).not.toBeNull();
    expect(popup()).toBeNull();
  });

  // ページのある日誌は、その項目を出す。
  test('a journal with a page shows its items', () => {
    const { container, show } = app({
      kind: 'Screen',
      value: { kind: 'Journal', value: journal('Daily') },
    });

    act(() => {
      show({ kind: 'Outline', value: outline() });
    });

    expect(container.querySelector('app-outline')).not.toBeNull();
  });

  // 月と年の日誌と、種類のない日誌には、移動がない。
  test('a monthly journal and one without a kind do not move', () => {
    const { show } = app({
      kind: 'Screen',
      value: { kind: 'Journal', value: journal('Monthly') },
    });
    const monthly = screen.queryByText('<');
    show({ kind: 'Screen', value: { kind: 'Journal', value: journal(null) } });

    expect([monthly, screen.queryByText('<')]).toEqual([null, null]);
  });

  // ページのない日誌は、項目を出さない。下に動かした日誌は、見出しに印がつく。
  test('a journal without a page shows no items, and a scrolled one marks its header', () => {
    const { container, show } = app({
      kind: 'Screen',
      value: { kind: 'Journal', value: journal('Daily', null) },
    });
    const header = () => container.querySelector('app-journal > div');
    const plain = header()?.className;

    show({ kind: 'Outline', value: outline({ scrolled_from_top: true }) });

    expect(container.querySelector('app-outline')).toBeNull();
    expect([plain, header()?.className]).toEqual([
      'journal-header',
      'journal-header scrolled',
    ]);
  });
});

// 文書のページ
describe('a document page', () => {
  function page(view: PageView) {
    return app({ kind: 'Screen', value: { kind: 'Page', value: view } });
  }

  // 題名を出す。フォルダの中のページは、フォルダの道筋を出し、押すとそのフォルダの目次を開く。
  test('the title shows, and the folders of a page lead to their index', () => {
    const { wasm, container } = page(
      docPage('plan', 'work/', {
        dir_segments: [new DirSegment('work', 'work/', 'work/index')],
      }),
    );

    fireEvent.click(screen.getByText('work'));

    expect(pageHeading()).toBe('plan');
    expect(container.querySelector('.dir-path-breadcrumb')?.className).toBe(
      'dir-path-breadcrumb',
    );
    expect(wasm.events).toEqual([
      { kind: 'GoToPage', path: 'work/index', open_in: { kind: 'Here' } },
    ]);
  });

  // アトラスのページは、項目の代わりに、そのフォルダのキャンバスを出す。
  test('an atlas page shows the canvas of its folder in place of the items', () => {
    const { container } = page(
      docPage('index', 'maps/', { is_atlas: true, is_index: true }),
    );

    expect(container.querySelector('app-page .atlas')).not.toBeNull();
    expect(screen.getByTitle('Up')).not.toBeNull();
    expect(container.querySelector('app-outline')).toBeNull();
  });

  // プロパティは、見せるときだけ項目の上に出る。新しい行があれば、見せなくなっても残る。
  test('the properties show above the items when shown, and stay while a new row is there', () => {
    const { container, show } = page(docPage('notes'));
    const hidden = container.querySelector('.page-properties');
    const shown = (show_properties: boolean) =>
      show({
        kind: 'Screen',
        value: {
          kind: 'Page',
          value: docPage('notes', '', { show_properties }),
        },
      });

    shown(true);
    const above = container.querySelector('.outline-header .page-properties');
    fireEvent.click(screen.getByTitle('Add property'));
    shown(false);

    expect(hidden).toBeNull();
    expect(above).not.toBeNull();
    expect(container.querySelector('.property-row.draft')).not.toBeNull();
  });

  // 新しい行がなく、見せなくなれば、プロパティは消える。
  test('without a new row the properties go when they are no longer shown', () => {
    const { container, show } = page(
      docPage('notes', '', { show_properties: true }),
    );

    show({
      kind: 'Screen',
      value: { kind: 'Page', value: docPage('notes') },
    });

    expect(container.querySelector('.page-properties')).toBeNull();
  });

  // 一番上のフォルダのページは、道筋を出さない。下に動かしたページは、見出しに印がつく。
  test('a page at the top has no path, and a scrolled one marks its header', () => {
    const { container, show } = page(docPage('notes'));
    const header = () => container.querySelector('app-page > div');
    const plain = header()?.className;

    show({ kind: 'Outline', value: outline({ scrolled_from_top: true }) });

    expect(container.querySelector('.dir-path-breadcrumb')).toBeNull();
    expect([plain, header()?.className]).toEqual([
      'page-header',
      'page-header scrolled',
    ]);
  });

  // 目次のページは、最後のフォルダを題名として出し、目次の印をつける。名前を変えられるなら、そのボタンを出す。
  test('an index shows its folder as the title, and offers a new name when it can', () => {
    const { wasm, container } = page(
      docPage('index', 'work/', {
        is_index: true,
        can_demote: true,
        demote_name: 'work',
        dir_segments: [new DirSegment('work', 'work/', 'work/index')],
      }),
    );

    fireEvent.click(button('ページ名を work に変更する'));

    expect(container.querySelector('.dir-segment-title')?.textContent).toBe(
      'work',
    );
    expect(container.querySelector('.dir-segment')).toBeNull();
    expect(container.querySelector('.dir-path-breadcrumb')?.className).toBe(
      'dir-path-breadcrumb index-breadcrumb',
    );
    expect(wasm.events).toEqual([{ kind: 'DemoteIndex' }]);
  });

  // 深いフォルダの目次では、上のフォルダは目次へのリンクで、最後のフォルダだけが題名である。
  test('in a deep index the upper folders lead to their index, and only the last is the title', () => {
    const { container } = page(
      docPage('index', 'work/plan/', {
        is_index: true,
        dir_segments: [
          new DirSegment('work', 'work/', 'work/index'),
          new DirSegment('plan', 'work/plan/', 'work/plan/index'),
        ],
      }),
    );

    expect(
      [...container.querySelectorAll('.dir-segment')].map(a => a.textContent),
    ).toEqual(['work']);
    expect(container.querySelector('.dir-segment-title')?.textContent).toBe(
      'plan',
    );
  });

  // 名前を変えられない目次には、ボタンがない。
  test('an index that cannot take a new name has no button', () => {
    page(docPage('index', 'work/', { is_index: true }));

    expect(screen.queryByRole('button', { name: /ページ名/ })).toBeNull();
    expect(screen.getByText(/^index/)).not.toBeNull();
  });

  // 一番上の目次は、(root)/と目次の印を出し、道筋は出さない。
  test('the index at the top shows (root)/ and no path', () => {
    const { container } = page(
      docPage('index', 'x/', { is_index: true, is_root_index: true }),
    );

    expect(pageHeading()).toBe('(root)/');
    expect(container.querySelector('.index-label')?.textContent).toBe('index');
    expect(container.querySelector('.dir-path-breadcrumb')).toBeNull();
  });
});

// 被リンク
describe('the backlinks', () => {
  function linked() {
    return app({
      kind: 'Screen',
      value: {
        kind: 'Page',
        value: docPage('notes', '', {
          backlinks: [
            new BacklinkGroup('2026-11-22', '/journals/2026-11-22', [
              new BacklinkItem('i', 'see [[notes]]', '/journals/2026-11-22#i'),
            ]),
          ],
        }),
      },
    });
  }

  // リンクしているページと項目を、押すとアプリの中で開く。
  test('the linking pages and items open in the app', () => {
    const { wasm } = linked();

    const kept = [
      !fireEvent.click(screen.getByText('2026-11-22')),
      !fireEvent.click(screen.getByText('see [[notes]]')),
    ];

    expect(kept).toEqual([true, true]);
    expect(wasm.links).toEqual([
      '/journals/2026-11-22',
      '/journals/2026-11-22#i',
    ]);
  });

  // Shift、Ctrl、Commandを押したクリックは、ブラウザが開く。
  test('a click with Shift, Ctrl or Command is the browser’s', () => {
    const { wasm } = linked();
    const link = screen.getByText('2026-11-22');

    const kept = [
      !fireEvent.click(link, { shiftKey: true }),
      !fireEvent.click(link, { ctrlKey: true }),
      !fireEvent.click(link, { metaKey: true }),
    ];

    expect(kept).toEqual([false, false, false]);
    expect(wasm.links).toEqual([]);
  });

  // リンクがなければ、被リンクの欄はない。
  test('without links there is no section', () => {
    app({ kind: 'Screen', value: { kind: 'Page', value: docPage('notes') } });

    expect(screen.queryByText('Backlinks')).toBeNull();
  });
});

/** The page is shown or hidden, as the browser tells when a tab or the PC comes back. */
function turnPage(state: DocumentVisibilityState) {
  Object.defineProperty(document, 'visibilityState', {
    configurable: true,
    get: () => state,
  });
  fireEvent(document, new Event('visibilitychange'));
}

// 日付の変わり目
describe('a day that may have changed', () => {
  afterEach(() => {
    Reflect.deleteProperty(document, 'visibilityState');
  });

  // ページがまた見えるようになると、日付が変わったかもしれないとコアに知らせる。スリープから戻ったときなどである。
  test('the page shown again tells the core that the day may have changed', () => {
    const { wasm } = app();

    turnPage('visible');

    expect(wasm.events).toEqual([{ kind: 'DateMayHaveChanged' }]);
  });

  // ページが隠れたときは、知らせない。
  test('the page hidden tells the core nothing', () => {
    const { wasm } = app();

    turnPage('hidden');

    expect(wasm.events).toEqual([]);
  });
});

// ページ全体のキー
describe('the keys of the whole page', () => {
  // Ctrl+Alt+Enterは、ホームを開く。編集欄には届かない。
  test('Ctrl+Alt+Enter opens Home, and reaches no editor', () => {
    const { wasm } = app();
    const editor = document.createElement('textarea');
    document.body.append(editor);
    let reached = false;
    editor.addEventListener('keydown', () => (reached = true));

    const kept = !fireEvent.keyDown(editor, {
      key: 'Enter',
      ctrlKey: true,
      altKey: true,
    });

    expect([kept, reached]).toEqual([true, false]);
    expect(wasm.events).toEqual([{ kind: 'NavigateHome' }]);
    editor.remove();
  });

  // StrictModeで枠を描き直しても、キーは一度だけ届く。枠がなくなれば、ページのキーは聞かない。
  test('in StrictMode a key arrives once, and without the frame the page keys are off', () => {
    const { core, wasm, asked } = shell();
    const { unmount } = render(
      <StrictMode>
        <CoreContext value={core}>
          <AskedOpsContext value={asked}>
            <App />
          </AskedOpsContext>
        </CoreContext>
      </StrictMode>,
    );
    const home = () =>
      fireEvent.keyDown(document.body, {
        key: 'Enter',
        ctrlKey: true,
        altKey: true,
      });

    home();
    const once = wasm.events.length;
    unmount();
    home();

    expect([once, wasm.events.length]).toEqual([1, 1]);
    expect([
      fireEvent.dragOver(document.body),
      fireEvent.drop(document.body),
    ]).toEqual([true, true]);
  });

  // Shift、Command、IMEの変換が加わると、ホームは開かない。
  test('with Shift, Command or a composition it does not open Home', () => {
    const { wasm } = app();
    const press = (init: KeyboardEventInit) =>
      fireEvent.keyDown(document.body, {
        key: 'Enter',
        ctrlKey: true,
        altKey: true,
        ...init,
      });

    press({ shiftKey: true });
    press({ metaKey: true });
    press({ isComposing: true });
    press({ altKey: false });
    press({ ctrlKey: false });
    press({ key: 'x' });

    expect(wasm.events).toEqual([]);
  });

  // 項目の外に落としたファイルは、ブラウザに開かせない。
  test('a file dropped off the items is not opened by the browser', () => {
    app();

    const kept = [
      !fireEvent.dragOver(document.body),
      !fireEvent.drop(document.body),
    ];

    expect(kept).toEqual([true, true]);
  });

  /** The editor of the item a, which waits for its focus. */
  function waiting() {
    vi.useFakeTimers();
    const drawn = app();
    const editor = document.createElement('textarea');
    editor.id = 'item-editor-a';
    document.body.append(editor);
    act(() =>
      drawn.wasm.ask(1, { kind: 'FocusEditor', item_id: 'a', replay_keys: [] }),
    );
    const played = () => {
      act(() => {
        vi.runAllTimers();
      });
      editor.remove();
      return drawn.wasm.replays.map(r => r.key.key);
    };
    return { ...drawn, played };
  }

  // 編集欄がフォーカスを待つ間、ページに来たキーはその編集欄のために取っておき、あとで再生する。
  test('a key that reaches the page while an editor waits for its focus is played into it', () => {
    const { played } = waiting();

    const kept = !fireEvent.keyDown(document.body, { key: 'x' });

    expect(kept).toBe(true);
    expect(played()).toEqual(['x']);
  });

  // 入力欄のキーと、取り消されたキーは、取っておかない。
  test('a key of an input and a prevented key are not kept', () => {
    const { played } = waiting();
    const input = document.createElement('input');
    document.body.append(input);
    const prevent = (event: Event) => event.preventDefault();

    fireEvent.keyDown(input, { key: 'x' });
    document.body.addEventListener('keydown', prevent);
    fireEvent.keyDown(document.body, { key: 'y' });
    document.body.removeEventListener('keydown', prevent);

    expect(played()).toEqual([]);
    input.remove();
  });

  // 編集欄のキーは、編集欄が自分で取っておくので、ページでは取っておかない。
  test('a key of an editor is not kept by the page, as the editor keeps its own', () => {
    const { played } = waiting();
    const other = document.createElement('textarea');
    document.body.append(other);

    fireEvent.keyDown(other, { key: 'x' });

    expect(played()).toEqual([]);
    other.remove();
  });

  // 待つ編集欄がなければ、キーは取っておかない。
  test('no key is kept without an editor that waits', () => {
    app();

    expect(fireEvent.keyDown(document.body, { key: 'x' })).toBe(true);
  });
});

// 破壊的な確認のダイアログ
describe('the confirm dialog', () => {
  function asking() {
    const drawn = app();
    act(() =>
      drawn.wasm.ask(9, {
        kind: 'ConfirmDestructive',
        value: 'Delete 3 items?',
      }),
    );
    return drawn;
  }

  // 確認が来るとダイアログが出て、キャンセルにフォーカスがある。Enterで消してしまわないためである。
  test('the dialog shows the message, with the focus on Cancel', () => {
    asking();

    expect(screen.getByRole('alertdialog').textContent).toContain(
      'Delete 3 items?',
    );
    expect(document.activeElement).toBe(button('Cancel'));
  });

  // OKははいと答え、キャンセルとEscapeはいいえと答える。答えると、ダイアログは消える。
  test('OK answers yes, and Cancel and Escape answer no, and the dialog goes', () => {
    const answers = (act: () => void) => {
      const { wasm, unmount } = asking();
      act();
      const shown = screen.queryByRole('alertdialog');
      unmount();
      return [wasm.answers, shown];
    };

    expect(answers(() => fireEvent.click(button('OK')))).toEqual([
      [{ id: 9, output: { kind: 'Bool', value: true } }],
      null,
    ]);
    expect(answers(() => fireEvent.click(button('Cancel')))).toEqual([
      [{ id: 9, output: { kind: 'Bool', value: false } }],
      null,
    ]);
    expect(
      answers(() => fireEvent.keyDown(button('OK'), { key: 'Escape' })),
    ).toEqual([[{ id: 9, output: { kind: 'Bool', value: false } }], null]);
  });

  // ダイアログのEscapeは、ダイアログが使い、ブラウザには渡さない。
  test('Escape in the dialog is the dialog’s', () => {
    asking();

    expect(fireEvent.keyDown(button('OK'), { key: 'Escape' })).toBe(false);
  });

  // ほかのキーでは、答えない。
  test('another key gives no answer', () => {
    const { wasm } = asking();

    fireEvent.keyDown(button('OK'), { key: 'Enter' });

    expect(wasm.answers).toEqual([]);
    expect(screen.queryByRole('alertdialog')).not.toBeNull();
  });
});
