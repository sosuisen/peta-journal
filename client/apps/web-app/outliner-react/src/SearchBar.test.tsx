import { act, fireEvent } from '@testing-library/react';
import { afterEach, describe, expect, test, vi } from 'vitest';
import { SearchEntry, SearchPanel, type SearchKind } from './border/peta-types';
import SearchBar from './SearchBar';
import { draw, outline, shell } from './test/screens';

afterEach(() => {
  vi.useRealTimers();
});

function entry(
  kind: SearchKind['kind'],
  title: string,
  label = '',
  snippet = '',
): SearchEntry {
  return new SearchEntry(`${kind}:${title}`, { kind }, title, label, snippet);
}

/** The search as a test shows it: the panel, and the results that come as a list beside it. */
interface Shown {
  readonly panel: SearchPanel;
  readonly results: SearchEntry[];
}

/** The search of `query`, with no result shown. Each test changes what it needs. */
function search(
  changes: Partial<SearchPanel & { results: SearchEntry[] }> = {},
): Shown {
  const { results = [], ...panel } = changes;
  return {
    panel: Object.assign(new SearchPanel('', false, null, false), panel),
    results,
  };
}

function searchBar(shown: Shown = search()) {
  const ui = shell();
  const push = ({ panel, results }: Shown) =>
    ui.wasm.push([], [{ kind: 'Search', value: panel }], {
      search_results: [{ kind: 'Reset', items: results }],
    });
  push(shown);
  const drawn = draw(ui, <SearchBar />);
  const input = drawn.container.querySelector('input') as HTMLInputElement;
  const show = (next: Shown) => act(() => push(next));
  return { ...ui, ...drawn, input, show };
}

// 検索の入力欄
describe('the input of the search', () => {
  // 入力欄はコアの問い合わせを出す。コアが消せば、入力欄も消える。
  test('the input shows the query of the core, and is cleared when the core clears it', () => {
    const { input, show } = searchBar(search({ query: 'plan' }));
    const shown = input.value;

    show(search({ query: '' }));

    expect([shown, input.value]).toEqual(['plan', '']);
  });

  // 打つと、問い合わせをコアに渡す。
  test('typing gives the query to the core', () => {
    const { input, wasm } = searchBar();

    fireEvent.input(input, { target: { value: 'pl' } });

    expect(wasm.events).toEqual([{ kind: 'SearchQueryChanged', value: 'pl' }]);
  });

  // フォーカスすると候補を開き、離れると200ms後に閉じるようコアに頼む。候補を押す間は閉じないためである。
  test('a focus opens the results, and a blur closes them 200ms later', () => {
    vi.useFakeTimers();
    const { input, wasm } = searchBar();

    fireEvent.focus(input);
    fireEvent.blur(input);
    act(() => {
      vi.advanceTimersByTime(199);
    });
    const before = wasm.events.length;
    act(() => {
      vi.advanceTimersByTime(1);
    });

    expect(before).toBe(1);
    expect(wasm.events).toEqual([
      { kind: 'SearchDropdown', value: true },
      { kind: 'SearchDropdown', value: false },
    ]);
  });
});

// 検索の結果
describe('the results of the search', () => {
  // 結果は差分で届く。変わった結果だけが、新しい文言になる。
  test('the results come as changes, and a changed one shows its new label', () => {
    const { container, wasm } = searchBar(search({ show_dropdown: true }));
    const plan = entry('Create', 'plan', 'Create page "plan"');
    const diary = entry('Create', 'work/plan', 'Create page "work/plan"');

    act(() =>
      wasm.push([], [], {
        search_results: [{ kind: 'Reset', items: [plan, diary] }],
      }),
    );
    act(() =>
      wasm.push([], [], {
        search_results: [
          { kind: 'Update', at: 1, item: { ...diary, label: 'changed' } },
        ],
      }),
    );

    expect(
      [...container.querySelectorAll('.search-create')].map(e => e.textContent),
    ).toEqual(['Create page "plan"', 'changed']);
  });
});

// 検索のキー
describe('the keys of the search', () => {
  // 上下とEscapeはコアに渡り、ブラウザには渡らない。
  test('Down, Up and Escape go to the core, not to the browser', () => {
    const { input, wasm } = searchBar();

    const kept = ['ArrowDown', 'ArrowUp', 'Escape'].map(
      key => !fireEvent.keyDown(input, { key }),
    );

    expect(kept).toEqual([true, true, true]);
    expect(wasm.events).toEqual([
      { kind: 'SearchKey', value: { kind: 'Down' } },
      { kind: 'SearchKey', value: { kind: 'Up' } },
      { kind: 'SearchKey', value: { kind: 'Escape' } },
    ]);
  });

  // ほかのキーと、IMEの変換中のキーは、コアに渡さない。
  test('other keys and keys of an IME composition are not sent', () => {
    const { input, wasm } = searchBar(search({ selected_index: 0n }));

    const left = [
      fireEvent.keyDown(input, { key: 'a' }),
      fireEvent.keyDown(input, { key: 'Enter', isComposing: true }),
      fireEvent.keyDown(input, { key: 'ArrowDown', keyCode: 229 }),
    ];

    expect(left).toEqual([true, true, true]);
    expect(wasm.events).toEqual([]);
  });

  // Enterは、選んだ候補があるときと、答えを待つときだけコアに渡る。待つときは、コアが答えまでEnterを預かる。
  test('Enter goes to the core only with a chosen result or while the answer is on its way', () => {
    const { input, wasm, show } = searchBar();
    const nothing = fireEvent.keyDown(input, { key: 'Enter' });
    show(search({ selected_index: 0n }));
    fireEvent.keyDown(input, { key: 'Enter' });
    show(search({ searching: true }));
    fireEvent.keyDown(input, { key: 'Enter' });

    expect(nothing).toBe(true);
    expect(wasm.events).toEqual([
      { kind: 'SearchKey', value: { kind: 'Enter' } },
      { kind: 'SearchKey', value: { kind: 'Enter' } },
    ]);
  });
});

// 検索の候補
describe('the results of the search', () => {
  const results = [
    entry('Today', '', 'Today: 2026-11-23'),
    entry('Page', 'notes'),
    entry('Item', 'notes', '', 'the <mark>plan</mark>'),
    entry('Create', '', 'Create "plan"'),
  ];

  // 候補は種類ごとの形で出る。項目は、ページの名前と、印のついた抜き書きを出す。選んだ候補に印がつく。
  test('each kind of result has its own shape, and the chosen one is marked', () => {
    const { container } = searchBar(
      search({ results, show_dropdown: true, selected_index: 2n }),
    );
    const dropdown = container.querySelector('.search-dropdown') as Element;

    expect(
      [...dropdown.children].map(e => [
        e.className,
        e.querySelector('use')?.getAttribute('href') ?? null,
        e.textContent,
      ]),
    ).toEqual([
      ['search-today', null, 'Today: 2026-11-23'],
      ['search-result', '#icon-page', 'notes'],
      ['search-result selected', '#icon-item', 'notesthe plan'],
      ['search-create', null, 'Create "plan"'],
    ]);
    expect(dropdown.querySelector('.item-snippet')?.innerHTML).toBe(
      'the <mark>plan</mark>',
    );
  });

  // 今日と作るの候補も、選ぶと印がつく。
  test('Today and Create are marked when chosen', () => {
    const { container, show } = searchBar(
      search({ results, show_dropdown: true, selected_index: 0n }),
    );
    const today = container.querySelector('.search-today')?.className;
    show(search({ results, show_dropdown: true, selected_index: 3n }));

    expect([
      today,
      container.querySelector('.search-create')?.className,
    ]).toEqual(['search-today selected', 'search-create selected']);
  });

  // 候補を押すと、その位置をコアに渡す。押したときに渡すのは、入力欄が離れて候補が閉じる前だからである。
  test('a press on a result gives its place to the core', () => {
    const { container, wasm } = searchBar(
      search({ results, show_dropdown: true }),
    );

    for (const shown of container.querySelectorAll('.search-dropdown > div')) {
      fireEvent.mouseDown(shown);
    }

    expect(wasm.events).toEqual(
      [0n, 1n, 2n, 3n].map(position => ({
        kind: 'SearchResultClicked',
        position,
      })),
    );
  });

  // 結果がないときと、候補を閉じたときは、何も出さない。
  test('nothing shows without results, or while the results are closed', () => {
    const { container, show } = searchBar(search({ show_dropdown: true }));
    const empty = container.querySelector('.search-dropdown');
    show(search({ results, show_dropdown: false }));

    expect([empty, container.querySelector('.search-dropdown')]).toEqual([
      null,
      null,
    ]);
  });

  // コアがまだ検索を出していなくても、入力欄はある。
  test('the input is there before the core gives a search', () => {
    const { container } = draw(shell(), <SearchBar />);

    expect(container.querySelector('input')?.value).toBe('');
  });
});

// 検索欄へのフォーカス
describe('the focus of the search', () => {
  // Ctrl+/は、どこにいても検索欄にフォーカスする。ブラウザには渡らない。
  test('Ctrl+/ focuses the search from anywhere', () => {
    const { input, wasm } = searchBar();
    act(() =>
      wasm.push(
        [],
        [{ kind: 'Outline', value: outline({ focused_item_id: 'a' }) }],
      ),
    );
    const editor = document.createElement('textarea');
    document.body.append(editor);

    const kept = !fireEvent.keyDown(editor, { key: '/', ctrlKey: true });

    expect(kept).toBe(true);
    expect(document.activeElement).toBe(input);
    editor.remove();
  });

  // /だけでも、編集中でなく、入力欄の外で、AltとCommandがなければ、検索欄にフォーカスする。
  test('a bare / focuses the search outside the editors and the inputs', () => {
    const { input } = searchBar();

    const kept = !fireEvent.keyDown(document.body, { key: '/' });

    expect(kept).toBe(true);
    expect(document.activeElement).toBe(input);
  });

  // 編集中、入力欄の中、AltかCommandを押したとき、ほかのキーでは、/は文字のままである。
  test('a bare / stays a letter while editing, in an input, with Alt or Command, and other keys do nothing', () => {
    const { input, wasm } = searchBar();
    const other = document.createElement('input');
    const editor = document.createElement('textarea');
    document.body.append(other, editor);

    const left = [
      fireEvent.keyDown(other, { key: '/' }),
      fireEvent.keyDown(editor, { key: '/' }),
      fireEvent.keyDown(document.body, { key: '/', altKey: true }),
      fireEvent.keyDown(document.body, { key: '/', metaKey: true }),
      fireEvent.keyDown(document.body, { key: '?', ctrlKey: true }),
    ];
    act(() =>
      wasm.push(
        [],
        [{ kind: 'Outline', value: outline({ focused_item_id: 'a' }) }],
      ),
    );
    left.push(fireEvent.keyDown(document.body, { key: '/' }));

    expect(left).toEqual([true, true, true, true, true, true]);
    expect(document.activeElement).not.toBe(input);
    other.remove();
    editor.remove();
  });

  // 検索欄がなくなると、/は聞かない。
  test('without the search a / is not heard', () => {
    const { unmount } = searchBar();

    unmount();

    expect(fireEvent.keyDown(document.body, { key: '/' })).toBe(true);
  });
});
