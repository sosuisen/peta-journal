import { createEvent, fireEvent } from '@testing-library/react';
import { useState } from 'react';
import { afterEach, describe, expect, test, vi } from 'vitest';
import {
  ConfigView,
  FreeKey,
  PageProperty,
  PageView,
} from './border/peta-types';
import PageProperties, { type Draft } from './PageProperties';
import { draw, shell } from './test/screens';

afterEach(() => {
  vi.useRealTimers();
});

/** A property with a text value. Each test changes what it needs. */
function property(
  key: string,
  value: string,
  changes: Partial<PageProperty> = {},
): PageProperty {
  return Object.assign(
    new PageProperty(key, key, value, false, [], 'text', true, true),
    changes,
  );
}

function tags(key: string, values: string[]): PageProperty {
  return property(key, values.join(', '), {
    is_list: true,
    values,
    value_type: 'tags',
  });
}

function pageWith(
  properties: PageProperty[],
  freeKeys: FreeKey[] = [],
): PageView {
  const view = new PageView(
    { id: 'p' } as PageView['page'],
    'plan',
    [],
    false,
    false,
    false,
    false,
    '',
    null,
    properties,
    freeKeys,
    true,
    [],
  );
  return view;
}

function config(tagsDir: string): ConfigView {
  return new ConfigView('', { kind: 'Iso' }, false, tagsDir, '', [], null, 0);
}

/** The rows as the page keeps them, so a test can see a draft row come and go. */
function Holder({ page }: { readonly page: PageView }) {
  const [drafts, setDrafts] = useState<readonly Draft[]>([]);
  return <PageProperties page={page} drafts={drafts} setDrafts={setDrafts} />;
}

function properties(page: PageView) {
  const ui = shell();
  ui.wasm.push([], [{ kind: 'Config', value: config('tags/') }]);
  const drawn = draw(ui, <Holder page={page} />);
  const rows = () =>
    [...drawn.container.querySelectorAll('.property-row')].map(e => e);
  const show = (next: PageView) => drawn.rerender(<Holder page={next} />);
  return { ...ui, ...drawn, rows, show };
}

const at = (row: Element, selector: string) =>
  row.querySelector(selector) as HTMLInputElement;

// 保存されたプロパティ
describe('the saved properties', () => {
  // プロパティは1行ずつ、名前と値を出す。消せるものだけに×があり、押すと消す。
  test('each property is a row of its name and value, and only a removable one has ×', () => {
    const { rows, wasm } = properties(
      pageWith([
        property('status', 'draft', { name: 'Status' }),
        property('created', '2026-01-02T03:04:00Z', {
          value_type: 'datetime',
          removable: false,
        }),
      ]),
    );
    const [status, created] = rows();

    fireEvent.click(at(status, '.property-remove'));

    expect(
      rows().map(e => [
        e.querySelector('.property-key')?.textContent,
        e.querySelector('.property-remove') !== null,
      ]),
    ).toEqual([
      ['Status', true],
      ['created', false],
    ]);
    expect(at(status, 'input').value).toBe('draft');
    expect(at(created, 'input').type).toBe('datetime-local');
    expect(wasm.events).toEqual([
      { kind: 'DeletePageProperty', key: 'status' },
    ]);
  });

  // 文字の値は、変えたときに保存する。コアが値を変えれば、入力欄も変わる。
  test('a text value is saved on change, and follows the core', () => {
    const { rows, wasm, show } = properties(
      pageWith([property('status', 'draft')]),
    );
    const input = at(rows()[0], 'input');

    fireEvent.change(input, { target: { value: 'done' } });
    fireEvent.keyDown(input, { key: 'Enter' });
    fireEvent.blur(input);
    show(pageWith([property('status', 'later')]));

    expect(input.value).toBe('later');
    expect(wasm.events).toEqual([
      {
        kind: 'SetPageProperty',
        key: 'status',
        value_type: 'text',
        value: 'done',
      },
    ]);
  });

  // 入力欄は型に合わせる。数、日付、一覧の文字、範囲である。
  test('the input fits the type of the value', () => {
    const { rows } = properties(
      pageWith(
        ['number', 'date', 'list', 'bounds', 'text'].map(type =>
          property(type, '', { value_type: type }),
        ),
      ),
    );

    expect(
      rows().map(e => [at(e, 'input').type, at(e, 'input').placeholder]),
    ).toEqual([
      ['number', ''],
      ['date', ''],
      ['text', 'a, b'],
      ['text', 'x, y, width, height'],
      ['text', ''],
    ]);
  });

  // チェックは、押すたびに保存する。
  test('a checkbox is saved on every click', () => {
    const { rows, wasm } = properties(
      pageWith([
        property('done', 'true', { value_type: 'checkbox' }),
        property('open', 'false', { value_type: 'checkbox' }),
      ]),
    );
    const [done, open] = rows().map(e => at(e, 'input'));
    const checked = [done.checked, open.checked];

    fireEvent.click(done);
    fireEvent.click(open);

    expect(checked).toEqual([true, false]);
    expect(wasm.events).toEqual([
      {
        kind: 'SetPageProperty',
        key: 'done',
        value_type: 'checkbox',
        value: 'false',
      },
      {
        kind: 'SetPageProperty',
        key: 'open',
        value_type: 'checkbox',
        value: 'true',
      },
    ]);
  });

  // 日時は分までの時刻で出す。変えて離れるかEnterで、UTCにして保存する。変えなければ保存しない。
  test('a date-time shows to the minute, and is saved as UTC when it changed and the focus leaves or Enter is pressed', () => {
    const saved = new Date(2026, 0, 2, 3, 4).toISOString();
    const { rows, wasm } = properties(
      pageWith([property('due', saved, { value_type: 'datetime' })]),
    );
    const input = at(rows()[0], 'input');
    const shown = input.value;

    fireEvent.blur(input);
    fireEvent.keyDown(input, { key: 'Enter' });
    input.value = '2026-01-02T05:06';
    fireEvent.keyDown(input, { key: 'Enter', isComposing: true });
    fireEvent.blur(input);
    const left = fireEvent.keyDown(input, { key: 'Enter' });
    input.value = '';
    fireEvent.blur(input);

    const utc = `${new Date(2026, 0, 2, 5, 6).toISOString().slice(0, 19)}Z`;
    expect(shown).toBe('2026-01-02T03:04');
    expect(left).toBe(false);
    expect(wasm.events).toEqual(
      [utc, utc].map(value => ({
        kind: 'SetPageProperty',
        key: 'due',
        value_type: 'datetime',
        value,
      })),
    );
  });

  // 変えられない日時は、読むだけの入力欄で、分までの時刻を出す。
  test('a date-time that cannot be changed shows in a read-only input', () => {
    const saved = new Date(2026, 0, 2, 3, 4).toISOString();
    const { rows } = properties(
      pageWith([
        property('modified', saved, {
          value_type: 'datetime',
          editable: false,
          removable: false,
        }),
      ]),
    );
    const input = at(rows()[0], 'input');

    expect([input.readOnly, input.className, input.value]).toEqual([
      true,
      'property-value property-readonly',
      '2026-01-02T03:04',
    ]);
  });
});

// タグの一覧
describe('the tags of a list', () => {
  // 一覧の値はタグで出す。タグはそのページへのリンクで、押すとアプリの中で開く。
  test('each tag links to its page, and a click opens it in the app', () => {
    const { rows, wasm } = properties(pageWith([tags('tags', ['a', 'b'])]));
    const links = [...rows()[0].querySelectorAll('a')];

    const left = fireEvent.click(links[1]);

    expect(links.map(a => [a.textContent, a.getAttribute('href')])).toEqual([
      ['#a', '/pages/tags/a'],
      ['#b', '/pages/tags/b'],
    ]);
    expect(left).toBe(false);
    expect(wasm.links).toEqual(['/pages/tags/b']);
  });

  // キーを押しながらタグを押すと、新しいタブか窓で開くようブラウザに任せる。
  test('a click with a key held is left to the browser', () => {
    const { rows, wasm } = properties(pageWith([tags('tags', ['a'])]));
    const link = at(rows()[0], 'a');

    const left = [
      fireEvent.click(link, { ctrlKey: true }),
      fireEvent.click(link, { shiftKey: true }),
    ];

    expect(left).toEqual([true, true]);
    expect(wasm.links).toEqual([]);
  });

  // タグの×で、そのタグを消す。
  test('the × of a tag removes it', () => {
    const { rows, wasm } = properties(pageWith([tags('tags', ['a', 'b'])]));

    fireEvent.click(rows()[0].querySelectorAll('.property-tag-remove')[1]);

    expect(wasm.events).toEqual([
      { kind: 'RemovePageTag', key: 'tags', tag: 'b' },
    ]);
  });

  // ＋で入力欄を開いてフォーカスする。Enterで打ったタグを足し、入力欄を閉じる。
  test('+ opens a focused input, and Enter adds the typed tags', () => {
    const { rows, wasm } = properties(pageWith([tags('tags', ['a'])]));
    fireEvent.click(at(rows()[0], '.property-tag-add'));
    const input = at(rows()[0], '.property-tag-input');
    const focused = document.activeElement === input;
    input.value = 'b, c';

    fireEvent.keyDown(input, { key: 'Enter', isComposing: true });
    const left = fireEvent.keyDown(input, { key: 'Enter' });

    expect([focused, left]).toEqual([true, false]);
    expect(rows()[0].querySelector('.property-tag-input')).toBeNull();
    expect(wasm.events).toEqual([
      { kind: 'AddPageTags', key: 'tags', typed: 'b, c' },
    ]);
  });

  // 入力欄から離れても、打ったタグを足す。空白だけなら何も足さない。
  test('leaving the input adds the typed tags, and blank text adds nothing', () => {
    const { rows, wasm } = properties(pageWith([tags('tags', [])]));
    const open = () => {
      fireEvent.click(at(rows()[0], '.property-tag-add'));
      return at(rows()[0], '.property-tag-input');
    };

    const first = open();
    first.value = 'b';
    fireEvent.blur(first);
    const second = open();
    second.value = '  ';
    fireEvent.blur(second);

    expect(wasm.events).toEqual([
      { kind: 'AddPageTags', key: 'tags', typed: 'b' },
    ]);
  });

  // Escapeで、何も足さずに入力欄を閉じる。ほかのキーでは閉じない。
  test('Escape closes the input without adding, and other keys keep it', () => {
    const { rows, wasm } = properties(pageWith([tags('tags', [])]));
    fireEvent.click(at(rows()[0], '.property-tag-add'));
    const input = at(rows()[0], '.property-tag-input');
    input.value = 'b';

    fireEvent.keyDown(input, { key: 'a' });
    const kept = rows()[0].querySelector('.property-tag-input');
    fireEvent.keyDown(input, { key: 'Escape' });
    fireEvent.blur(input);

    expect(kept).toBe(input);
    expect(rows()[0].querySelector('.property-tag-input')).toBeNull();
    expect(wasm.events).toEqual([]);
  });
});

// タグの並べ替え
describe('the order of the tags', () => {
  /** The chips are 100 wide from 0: the right half starts at 50. */
  function chips() {
    const drawn = properties(pageWith([tags('tags', ['a', 'b', 'c'])]));
    const chip = (n: number) =>
      drawn.rows()[0].querySelectorAll('.property-tag')[n];
    for (const e of drawn.rows()[0].querySelectorAll('.property-tag')) {
      e.getBoundingClientRect = () => ({ left: 0, width: 100 }) as DOMRect;
    }
    return { ...drawn, chip };
  }

  function dataTransfer() {
    const data = new Map<string, string>();
    return {
      data,
      setData: (type: string, value: string) => data.set(type, value),
      effectAllowed: '',
      dropEffect: '',
    };
  }

  /** jsdom has no DragEvent, so the place of the pointer is put on the event by hand. */
  function dragAt(
    kind: 'dragOver' | 'drop',
    chip: Element,
    clientX: number,
    moved?: ReturnType<typeof dataTransfer>,
  ): boolean {
    const event = createEvent[kind](chip, { dataTransfer: moved });
    Object.defineProperty(event, 'clientX', { value: clientX });
    return fireEvent(chip, event);
  }

  // タグを別のタグの右半分に落とすと、その後ろに動く。左半分なら前に動く。
  test('a tag dropped on the right half of another goes after it, and on the left half before it', () => {
    const { chip, wasm } = chips();
    const moved = dataTransfer();

    fireEvent.dragStart(chip(0), { dataTransfer: moved });
    dragAt('dragOver', chip(2), 60, moved);
    dragAt('drop', chip(2), 60, moved);
    fireEvent.dragStart(chip(2), { dataTransfer: moved });
    dragAt('drop', chip(1), 40, moved);

    expect([...moved.data]).toEqual([['text/plain', '#c']]);
    expect([moved.effectAllowed, moved.dropEffect]).toEqual(['move', 'move']);
    expect(wasm.events).toEqual([
      { kind: 'MovePageTag', key: 'tags', tag: 'a', target: 'c', after: true },
      { kind: 'MovePageTag', key: 'tags', tag: 'c', target: 'b', after: false },
    ]);
  });

  // 引いている間、引いたタグと、落とす側に印がつく。離れるか終わると印は消える。
  test('while dragging, the dragged tag and the side to drop on are marked', () => {
    const { chip } = chips();
    const moved = dataTransfer();

    fireEvent.dragStart(chip(0), { dataTransfer: moved });
    dragAt('dragOver', chip(1), 40, moved);
    const before = [chip(0).className, chip(1).className];
    dragAt('dragOver', chip(1), 60, moved);
    const after = chip(1).className;
    fireEvent.dragLeave(chip(1));
    const left = chip(1).className;
    dragAt('dragOver', chip(2), 60, moved);
    fireEvent.dragEnd(chip(0));

    expect(before).toEqual([
      'property-tag dragging',
      'property-tag drop-before',
    ]);
    expect([after, left]).toEqual(['property-tag drop-after', 'property-tag']);
    expect([0, 1, 2].map(n => chip(n).className)).toEqual([
      'property-tag',
      'property-tag',
      'property-tag',
    ]);
  });

  // ほかから引いてきた文字などは、タグの上に落とせない。
  test('something dragged from elsewhere cannot be dropped on a tag', () => {
    const { chip, wasm } = chips();

    const over = dragAt('dragOver', chip(1), 60);
    const dropped = dragAt('drop', chip(1), 60);

    expect([over, dropped]).toEqual([true, true]);
    expect(chip(1).className).toBe('property-tag');
    expect(wasm.events).toEqual([]);
  });
});

// 新しい行
describe('a new row', () => {
  const freeKeys = [
    new FreeKey('status', 'Status', 'text'),
    new FreeKey('due', 'Due', 'datetime'),
    new FreeKey('tags', 'Tags', 'tags'),
  ];

  function newRow() {
    const drawn = properties(pageWith([], freeKeys));
    fireEvent.click(drawn.getByTitle('Add property'));
    const row = () => drawn.container.querySelector('.property-row.draft');
    const pick = (key: string) =>
      fireEvent.change(at(row() as Element, 'select'), {
        target: { value: key },
      });
    return { ...drawn, row, pick };
  }

  // ＋で、キーを選ぶ行を足す。選べるのは、まだないキーである。値の入力欄はまだない。
  test('+ adds a row that picks a key from the free keys', () => {
    const { row } = newRow();
    const select = at(row() as Element, 'select');

    expect(
      [...select.querySelectorAll('option')].map(o => [o.value, o.textContent]),
    ).toEqual([
      ['', 'key'],
      ['status', 'Status'],
      ['due', 'Due'],
      ['tags', 'Tags'],
    ]);
    expect(row()?.querySelector('input')).toBeNull();
  });

  // キーを選ぶと、型に合う入力欄にフォーカスする。値を入れてEnterで、プロパティになり、行は消える。
  test('a picked key focuses an input of its type, and a value with Enter makes the property', () => {
    const { row, pick, wasm } = newRow();
    pick('status');
    const input = at(row() as Element, 'input');
    const focused = document.activeElement === input;

    fireEvent.keyDown(input, { key: 'Enter' });
    input.value = 'draft';
    fireEvent.keyDown(input, { key: 'Enter', isComposing: true });
    const left = fireEvent.keyDown(input, { key: 'Enter' });
    fireEvent.blur(input);

    expect([focused, left]).toEqual([true, false]);
    expect(row()).toBeNull();
    expect(wasm.events).toEqual([
      {
        kind: 'SetPageProperty',
        key: 'status',
        value_type: 'text',
        value: 'draft',
      },
    ]);
  });

  // 入力欄から離れても、値があればプロパティになる。
  test('leaving the input with a value makes the property', () => {
    const { row, pick, wasm } = newRow();
    pick('status');
    const input = at(row() as Element, 'input');
    fireEvent.blur(input);
    const kept = row();
    input.value = 'draft';

    fireEvent.blur(input);

    expect(kept).not.toBeNull();
    expect(row()).toBeNull();
    expect(wasm.events).toEqual([
      {
        kind: 'SetPageProperty',
        key: 'status',
        value_type: 'text',
        value: 'draft',
      },
    ]);
  });

  // 新しい日時の行は今の時刻で始まり、Enterだけで、その時刻がプロパティになる。
  test('a new date-time starts at now, and Enter alone accepts it', () => {
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date(2026, 10, 23, 21, 7, 30));
    const { row, pick, wasm } = newRow();
    pick('due');
    const input = at(row() as Element, 'input');
    const shown = input.value;

    fireEvent.keyDown(input, { key: 'Enter' });

    const utc = `${new Date(2026, 10, 23, 21, 7).toISOString().slice(0, 19)}Z`;
    expect(shown).toBe('2026-11-23T21:07');
    expect(wasm.events).toEqual([
      {
        kind: 'SetPageProperty',
        key: 'due',
        value_type: 'datetime',
        value: utc,
      },
    ]);
  });

  // 一覧のキーを選ぶと、タグの入力欄が開く。足したタグがプロパティになり、行は消える。
  test('a list key opens the input of the tags, and the added tags make the property', () => {
    const { row, pick, wasm } = newRow();
    pick('tags');
    const input = at(row() as Element, '.property-tag-input');
    const focused = document.activeElement === input;
    input.value = 'a';

    fireEvent.keyDown(input, { key: 'Enter' });

    expect(focused).toBe(true);
    expect(row()).toBeNull();
    expect(wasm.events).toEqual([
      { kind: 'AddPageTags', key: 'tags', typed: 'a' },
    ]);
  });

  // 新しい行の×で、保存せずに行を消す。×を押しても、入力欄はフォーカスを手放さない。
  test('the × of a new row removes it without saving', () => {
    const { row, pick, wasm } = newRow();
    pick('status');
    const remove = at(row() as Element, '.property-remove');

    const kept = !fireEvent.mouseDown(remove);
    fireEvent.click(remove);

    expect(kept).toBe(true);
    expect(row()).toBeNull();
    expect(wasm.events).toEqual([]);
  });

  // キーを選ぶ前の行は、どうしても保存しない。キーを選び直すと、空に戻せる。
  test('a row without a key saves nothing', () => {
    const { row, pick, wasm } = newRow();
    pick('status');
    pick('');

    expect(row()?.querySelector('input')).toBeNull();
    expect(wasm.events).toEqual([]);
  });

  // 新しい行は、いくつでも足せる。
  test('more new rows can be added', () => {
    const { container, getByTitle } = newRow();

    fireEvent.click(getByTitle('Add property'));

    expect(container.querySelectorAll('.property-row.draft')).toHaveLength(2);
  });
});

// 設定がまだ届かないとき
describe('before the config arrives', () => {
  // タグのフォルダは、空として扱う。
  test('the folder of the tags is empty', () => {
    const ui = shell();
    const { container } = draw(
      ui,
      <Holder page={pageWith([tags('t', ['a'])])} />,
    );

    expect(container.querySelector('a')?.getAttribute('href')).toBe('/pages/a');
  });
});
