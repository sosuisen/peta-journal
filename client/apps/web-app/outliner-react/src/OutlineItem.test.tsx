import { act, fireEvent, render, screen } from '@testing-library/react';
import { StrictMode } from 'react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import type { FocusRequest, Row } from './border/peta-types';
import {
  eventLinkClicked,
  richLinkPage,
  richLinkWeb,
} from './border/peta-types';
import { AskedOpsContext } from './askedContext';
import { CoreContext } from './context';
import { OutlineItem } from './OutlineItem';
import { draw, focusAt, outline, row, shell } from './test/screens';

afterEach(() => {
  vi.restoreAllMocks();
  Reflect.deleteProperty(document, 'caretPositionFromPoint');
  Reflect.deleteProperty(navigator, 'clipboard');
  // A press on an item starts a drag, which a release ends
  fireEvent.mouseUp(document);
});

/** Draws the item of `shown`, and gives the shell and the drawn page. */
function item(
  shown: Row,
  {
    request = null,
    holdingKeys = false,
  }: { request?: FocusRequest | null; holdingKeys?: boolean } = {},
) {
  const ui = shell();
  // The core knows the row, as it does on the screen
  ui.wasm.push([{ kind: 'Reset', items: [shown] }]);
  const view = draw(
    ui,
    <OutlineItem row={shown} request={request} holdingKeys={holdingKeys} />,
  );
  const redraw = (next: Row, nextRequest: FocusRequest | null = request) =>
    view.rerender(
      <OutlineItem
        row={next}
        request={nextRequest}
        holdingKeys={holdingKeys}
      />,
    );
  return { ...ui, ...view, redraw };
}

/** Runs `act` and says whether its event of `type` reached the page around the app. */
function reachesThePage(type: string, act: () => void): boolean {
  let reached = false;
  const outside = () => (reached = true);
  document.addEventListener(type, outside);
  act();
  document.removeEventListener(type, outside);
  return reached;
}

/** The box of the item, which carries its state. */
function box(id: string): HTMLElement {
  const found = document.getElementById(id);
  if (found === null) {
    throw new Error(`no item ${id}`);
  }
  return found;
}

/** An ISO time today at `hours`:`minutes`:`seconds` in the time zone of the browser. */
function at(hours: number, minutes: number, seconds = 0): string {
  return new Date(2026, 10, 23, hours, minutes, seconds).toISOString();
}

// 項目の見た目
describe('how an item shows', () => {
  // 選択、見出し、印を出さない行は、クラスになる。見出しの段と字下げも、項目に出る。
  test('the selection, a heading and a row without a bullet are classes, with the level and the indent', () => {
    item(
      row('a', 'one', {
        is_selected: true,
        is_heading: true,
        heading_level: 2,
        show_bullet: false,
        indent_px: 24,
      }),
    );

    expect(box('a').className).toBe(
      'outline-item selected heading hide-bullet',
    );
    expect(box('a').dataset.headingLevel).toBe('2');
    expect(box('a').style.paddingLeft).toBe('24px');
  });

  // ふつうの項目には、クラスも段も倍率もない。
  test('a plain item has no class, no level and no zoom', () => {
    item(row('a', 'one'));

    expect(box('a').className).toBe('outline-item');
    expect(box('a').dataset.headingLevel).toBeUndefined();
    expect(
      box('a').querySelector<HTMLElement>(':scope > .item-content')?.style.zoom,
    ).toBe('');
  });

  // 拡大した項目は、その倍率で描かれる。
  test('a zoomed item is drawn at its zoom', () => {
    item(row('a', 'one', { effective_zoom: 1.5 }));

    const content = box('a').querySelector<HTMLElement>(
      ':scope > .item-content',
    );

    expect(content?.style.zoom).toBe('1.5');
  });

  // 元の見つからない埋め込みは、そのことを本文の代わりに出す。
  test('a transclusion whose source is gone says so instead of its body', () => {
    item(row('a', 'x1', { is_broken_transclusion: true }));

    expect(box('a').querySelector('.broken-transclusion')?.textContent).toBe(
      'source not found: x1',
    );
  });

  // 印は、閉じた項目では塗った輪に、埋め込みでは四角になる。
  test('the bullet is a filled ring when closed, and a square for a transclusion', () => {
    const marks = (shown: Row) => {
      const { unmount } = item(shown);
      const svg = box(shown.item.id).querySelector('.bullet svg');
      const drawn = [...(svg?.children ?? [])].map(
        e => `${e.tagName}:${e.getAttribute('fill')}`,
      );
      unmount();
      return drawn;
    };

    expect(marks(row('a'))).toEqual(['circle:#888']);
    expect(marks(row('a', '', { is_collapsed: true }))).toEqual([
      'circle:#ddd',
      'circle:#888',
    ]);
    expect(marks(row('a', '', { is_transclusion: true }))).toEqual([
      'rect:none',
      'circle:#888',
    ]);
    expect(
      marks(row('a', '', { is_transclusion: true, is_collapsed: true })),
    ).toEqual(['rect:#ddd', 'circle:#888']);
  });
});

// 子のある項目
describe('an item with children', () => {
  // 三角を押すと、開け閉めをコアに頼む。閉じた項目では、三角に印がつく。
  test('the triangle asks the core to open or close the item, and is marked when closed', () => {
    const { wasm, redraw } = item(
      row('a', 'one', { has_children: true, is_collapsed: true }),
    );
    const toggle = box('a').querySelector('.collapse-toggle');

    fireEvent.click(toggle as Element);
    const closed = toggle?.className;
    redraw(row('a', 'one', { has_children: true }));

    expect([closed, toggle?.className]).toEqual([
      'collapse-toggle collapsed',
      'collapse-toggle',
    ]);
    expect(wasm.events).toEqual([{ kind: 'CollapseToggle', item_id: 'a' }]);
  });

  // 子のある項目では、メニューのボタンは三角の外に出る。子がなければ、三角の場所にある。
  test('the menu button moves out of the place of the triangle when there are children', () => {
    const { unmount } = item(row('a', 'one', { has_children: true }));
    const withChildren = box('a').querySelector('.slot-menu .item-menu');
    unmount();
    item(row('a', 'one'));

    expect(withChildren).not.toBeNull();
    expect(box('a').querySelector('.slot-toggle .item-menu')).not.toBeNull();
    expect(box('a').querySelector('.collapse-toggle')).toBeNull();
  });

  // 子ができると、三角の場所のメニューのボタンは、新しい三角に替わる。同じボタンの色を変えると、CSSのトランジションが走り、そのあいだブラウザがスタイルを計算し直すためである。
  test('a new child puts a new triangle in place of the menu button, not the same button restyled', () => {
    const { redraw } = item(row('a', 'one'));
    const menu = box('a').querySelector('.slot-toggle button');

    redraw(row('a', 'one', { has_children: true }));
    const toggle = box('a').querySelector('.slot-toggle button');
    redraw(row('a', 'one'));

    expect(toggle?.className).toBe('collapse-toggle');
    expect(toggle).not.toBe(menu);
    expect(box('a').querySelector('.slot-toggle button')).not.toBe(toggle);
  });
});

// タスクの時刻
describe('the times of a task', () => {
  function times(begin: string | null, end: string | null) {
    const properties: Record<string, string> = {};
    if (begin !== null) properties.begin = begin;
    if (end !== null) properties.end = end;
    const { unmount } = item(
      row('a', 'task', { task: { kind: 'Done' } }, properties),
    );
    const shown = box('a').querySelector<HTMLElement>('.task-timestamps');
    const found =
      shown === null ? null : [shown.textContent, shown.title].join(' | ');
    unmount();
    return found;
  }

  // 走っているタスクは、始めた時刻を出す。
  test('a running task shows when it began', () => {
    expect(times(at(9, 5), null)).toBe('09:05- | 09:05-');
  });

  // 終えたタスクは、かかった時間を出し、時刻は吹き出しに出す。
  test('a done task shows how long it took, and the times in its tooltip', () => {
    expect(times(at(9, 0), at(10, 5))).toBe('1h5m | 09:00 - 10:05');
    expect(times(at(9, 0), at(11, 0, 30))).toBe('2h | 09:00 - 11:00');
    expect(times(at(9, 0), at(9, 5, 59))).toBe('5m | 09:00 - 09:05');
    expect(times(at(9, 0), at(9, 0, 30))).toBe('30s | 09:00 - 09:00');
  });

  // 終わりが始めより前の時間と、読めない時刻は、0秒とする。
  test('an end before the begin and a time that cannot be read take no time', () => {
    expect(times(at(9, 0), at(8, 0))).toBe('0s | 09:00 - 08:00');
    expect(times(at(9, 0), 'soon')).toBe('0s | 09:00 - soon');
  });

  // 始めた時刻のないタスクと、タスクでない項目には、時刻を出さない。
  test('no times without a begin, nor for an item that is not a task', () => {
    expect(times(null, at(9, 0))).toBeNull();
    const { unmount } = item(row('a', 'text', {}, { begin: at(9, 0) }));
    const shown = box('a').querySelector('.task-timestamps');
    unmount();
    expect(shown).toBeNull();
  });
});

// タスクのチェックボックス
describe('the checkbox of a task', () => {
  function checkbox(): HTMLInputElement {
    const found = box('a').querySelector<HTMLInputElement>('.task-checkbox');
    if (found === null) {
      throw new Error('no checkbox');
    }
    return found;
  }

  // 終えたタスクはチェックが入り、走っているタスクは中間の印になる。
  test('a done task is checked, and a running one is in between', () => {
    const { redraw } = item(row('a', 'task', { task: { kind: 'Done' } }));
    const done = [checkbox().checked, checkbox().indeterminate];
    redraw(row('a', 'task', { task: { kind: 'Running' } }));

    expect(done).toEqual([true, false]);
    expect([checkbox().checked, checkbox().indeterminate]).toEqual([
      false,
      true,
    ]);
  });

  // チェックボックスを押すと、タスクの状態を進めるようにコアに頼む。ブラウザはチェックを変えず、押したことは外に伝わらない。
  test('a click asks the core for the next state, the browser leaves the check, and the click goes no further', () => {
    const { wasm } = item(row('a', 'task', { task: { kind: 'NotStarted' } }));
    let reached = false;
    const outside = () => (reached = true);
    document.addEventListener('click', outside);

    const changed = !fireEvent.click(checkbox());
    document.removeEventListener('click', outside);

    expect(changed).toBe(true);
    expect(reached).toBe(false);
    expect(wasm.events).toEqual([{ kind: 'TaskCycle', item_id: 'a' }]);
  });

  // タスクでない項目には、チェックボックスがない。
  test('an item that is not a task has none', () => {
    item(row('a', 'text'));

    expect(box('a').querySelector('.task-checkbox')).toBeNull();
  });
});

// 項目のメニュー
describe('the menu of an item', () => {
  function open(shown: Row) {
    const drawn = item(shown);
    const writes: string[] = [];
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: {
        writeText: (text: string) => {
          writes.push(text);
          return Promise.resolve();
        },
      },
    });
    fireEvent.click(box(shown.item.id).querySelector('.item-menu') as Element);
    return { ...drawn, writes };
  }

  function menu(): HTMLElement | null {
    return document.querySelector('.item-popup-menu');
  }

  function choose(name: string): void {
    fireEvent.click(screen.getByText(name));
  }

  // メニューは、ボタンの下に開く。ボタンのクリックは、外に伝わらない。
  test('the menu opens under its button, and the click goes no further', () => {
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(
      function (this: HTMLElement) {
        return this.classList.contains('item-menu')
          ? new DOMRect(30, 40, 10, 10)
          : new DOMRect(10, 20, 500, 30);
      },
    );

    item(row('a', 'one'));

    const reached = reachesThePage('click', () =>
      fireEvent.click(box('a').querySelector('.item-menu') as Element),
    );

    expect(reached).toBe(false);
    expect([menu()?.style.left, menu()?.style.top]).toEqual(['20px', '30px']);
  });

  // コピーは本文を、参照のコピーは項目のIDを、クリップボードに書く。切り取りは、本文を書いてから消す。
  test('Copy writes the body, Copy Reference the id, and Cut empties the body after it', () => {
    const { wasm, writes } = open(row('a', 'one'));
    choose('Copy');
    fireEvent.click(box('a').querySelector('.item-menu') as Element);
    choose('Copy Reference');
    fireEvent.click(box('a').querySelector('.item-menu') as Element);
    choose('Cut');

    expect(writes).toEqual(['one', 'a', 'one']);
    expect(wasm.typings).toEqual([{ kind: 'Changed', item_id: 'a', body: '' }]);
  });

  // ほかの項目は、それぞれの操作をコアに頼み、メニューを閉じる。
  test('the other entries ask the core for their action, and close the menu', () => {
    const shown = row('a', 'one', {
      is_transclusion: true,
      has_own_zoom: true,
    });
    const names = [
      'Paste as Transclusion',
      'Remove Transclusion',
      'Send to Stash',
      'Reset Zoom',
    ];
    const { wasm } = open(shown);
    const closed = names.map(name => {
      if (menu() === null) {
        fireEvent.click(box('a').querySelector('.item-menu') as Element);
      }
      choose(name);
      return menu() === null;
    });

    expect(closed).toEqual([true, true, true, true]);
    expect(wasm.events.map(e => e.kind)).toEqual([
      'PasteAsTransclusion',
      'RemoveTransclusion',
      'SendToStash',
      'ResetZoom',
    ]);
  });

  // 埋め込みを外す項目と、拡大を戻す項目は、それができる項目にだけある。
  test('Remove Transclusion and Reset Zoom are there only when they can act', () => {
    open(row('a', 'one'));

    expect(screen.queryByText('Remove Transclusion')).toBeNull();
    expect(screen.queryByText('Reset Zoom')).toBeNull();
  });

  // 閉じたメニューは、ページに聞き手を残さない。
  test('a closed menu leaves no listener on the page', () => {
    const added = vi.spyOn(document, 'addEventListener');
    const removed = vi.spyOn(document, 'removeEventListener');
    open(row('a', 'one'));
    const listeners = added.mock.calls.filter(
      ([type]) => type === 'click' || type === 'keydown',
    );

    fireEvent.keyDown(document, { key: 'Escape' });

    for (const [type, listener] of listeners) {
      expect(removed).toHaveBeenCalledWith(type, listener);
    }
    expect(listeners).toHaveLength(2);
  });

  // メニューの外のクリックとEscapeは、メニューを閉じる。メニューの中のクリックは閉じない。
  test('a click outside and Escape close the menu, and a click inside does not', () => {
    open(row('a', 'one'));
    fireEvent.click(menu() as Element);
    const inside = menu() !== null;
    fireEvent.keyDown(document, { key: 'Enter' });
    const otherKey = menu() !== null;
    fireEvent.keyDown(document, { key: 'Escape' });
    const escaped = menu() === null;
    fireEvent.click(box('a').querySelector('.item-menu') as Element);
    fireEvent.click(document.body);

    expect([inside, otherKey, escaped]).toEqual([true, true, true]);
    expect(menu()).toBeNull();
  });
});

// 描いた本文
describe('the drawn body', () => {
  function content(id = 'a'): HTMLElement {
    return box(id).querySelector('app-item-view .item-content') as HTMLElement;
  }

  // 本文は、wasmが作ったHTMLで描かれる。画像は、URLの断片の大きさになる。本文が変わっても同じである。
  test('the body is the HTML of wasm, and an image gets the size of its fragment, also after a change', () => {
    const { redraw } = item(row('a', '', { body_model: '<p>none</p>' }));
    redraw(
      row('a', '', {
        body_model:
          '<p><img src="a.png#w=300&amp;h=240"><img src="b.png#h=120"><img src="c.png"></p>',
      }),
    );

    const sizes = [...content().querySelectorAll('img')].map(i => [
      i.getAttribute('width'),
      i.getAttribute('height'),
    ]);

    expect(sizes).toEqual([
      ['300', '240'],
      [null, '120'],
      [null, null],
    ]);
  });

  // 言語の名前のあるコードは色がつき、言語の名前が1つだけ出る。名前のないコードは、そのままである。
  test('code that names its language is coloured and labelled once, and other code is left alone', () => {
    const named = '<pre><code class="language-rust">let x = 1;</code></pre>';
    const plain = '<pre><code>plain words</code></pre>';
    const { core, asked } = shell();
    // The app runs in StrictMode, which runs each effect twice
    render(
      <StrictMode>
        <CoreContext value={core}>
          <AskedOpsContext value={asked}>
            <OutlineItem
              row={row('a', '', { body_model: named + plain })}
              request={null}
              holdingKeys={false}
            />
          </AskedOpsContext>
        </CoreContext>
      </StrictMode>,
    );

    const [rust, other] = [...content().querySelectorAll('pre')];

    expect(rust?.querySelector('code')?.classList.contains('hljs')).toBe(true);
    expect(
      [...(rust?.querySelectorAll('.code-lang-label') ?? [])].map(
        l => l.textContent,
      ),
    ).toEqual(['rust']);
    expect(rust?.style.position).toBe('relative');
    expect(other?.innerHTML).toBe('<code>plain words</code>');
  });

  // 文字を押すと、押した文字の前までの文字とともに、編集をコアに頼み、ドラッグを始める。
  test('a press on the letters asks the core to edit after them, and starts a drag', () => {
    const { wasm } = item(row('a', 'one two'));
    const text = content().querySelector('p')?.firstChild as Node;
    Object.defineProperty(document, 'caretPositionFromPoint', {
      configurable: true,
      value: () => ({ offsetNode: text, offset: 5 }),
    });

    const kept = !fireEvent.mouseDown(content().querySelector('p') as Element);

    expect(kept).toBe(true);
    expect(wasm.events).toEqual([
      {
        kind: 'EditRequested',
        item_id: 'a',
        place: { kind: 'AfterShownText', value: 'one t' },
      },
    ]);
    expect(wasm.drags.map(d => d.kind)).toEqual(['Pressed']);
  });

  // 描いた本文からのドラッグは、コアの置いたカーソルから、マウスの下の文字までを選ぶ。
  test('a drag from the drawn body selects from where the core put the cursor', () => {
    const { wasm, redraw } = item(row('a', 'one two'));
    Object.defineProperty(document, 'caretPositionFromPoint', {
      configurable: true,
      value: () => ({ offsetNode: document.body, offset: 0 }),
    });
    wasm.answerNext(
      [],
      [
        {
          kind: 'Outline',
          value: outline({ focus_request: focusAt(1, 'a', 4) }),
        },
      ],
    );
    fireEvent.mouseDown(content());
    redraw(row('a', 'one two', { is_editing: true }));
    const editor = document.getElementById(
      'item-editor-a',
    ) as HTMLTextAreaElement;
    Object.defineProperty(document, 'caretPositionFromPoint', {
      configurable: true,
      value: () => ({ offsetNode: editor, offset: 1 }),
    });
    Object.defineProperty(document, 'elementFromPoint', {
      configurable: true,
      value: () => null,
    });
    wasm.takesDrags = false;

    fireEvent.mouseMove(document);

    expect([editor.selectionStart, editor.selectionEnd]).toEqual([1, 4]);
    expect(editor.selectionDirection).toBe('backward');
    Reflect.deleteProperty(document, 'elementFromPoint');
  });

  // コアがカーソルを置かなかったときと、ほかの項目に置いたときは、ドラッグは文字を選ばない。
  test('a drag selects no text when the core put no cursor in the item', () => {
    for (const panel of [
      outline({ focus_request: focusAt(1, 'a', null) }),
      outline({ focus_request: focusAt(1, 'b', 2) }),
    ]) {
      const { wasm, redraw, unmount } = item(row('a', 'one two'));
      Object.defineProperty(document, 'caretPositionFromPoint', {
        configurable: true,
        value: () => ({ offsetNode: document.body, offset: 0 }),
      });
      wasm.answerNext([], [{ kind: 'Outline', value: panel }]);
      fireEvent.mouseDown(content());
      redraw(row('a', 'one two', { is_editing: true }));
      const editor = document.getElementById(
        'item-editor-a',
      ) as HTMLTextAreaElement;
      editor.setSelectionRange(2, 2);
      Object.defineProperty(document, 'elementFromPoint', {
        configurable: true,
        value: () => null,
      });
      wasm.takesDrags = false;

      fireEvent.mouseMove(document);
      fireEvent.mouseUp(document);

      expect([editor.selectionStart, editor.selectionEnd]).toEqual([2, 2]);
      unmount();
      Reflect.deleteProperty(document, 'elementFromPoint');
    }
  });

  // 文字の外を押すと、置き場所なしで編集を頼む。
  test('a press beside the letters asks to edit without a place', () => {
    const { wasm } = item(row('a', 'one'));
    Object.defineProperty(document, 'caretPositionFromPoint', {
      configurable: true,
      value: () => null,
    });

    fireEvent.mouseDown(content());

    expect(wasm.events).toEqual([
      { kind: 'EditRequested', item_id: 'a', place: { kind: 'Unplaced' } },
    ]);
  });

  // リンクを押しても、編集は始まらない。リンクを開くためである。
  test('a press on a link starts no edit', () => {
    const { wasm } = item(
      row('a', '', {
        body_model: '<p><a href="https://example.com">x</a></p>',
      }),
    );

    fireEvent.mouseDown(screen.getByText('x'));

    expect(wasm.events).toEqual([]);
    expect(wasm.drags).toEqual([]);
  });
});

// 描いた本文のリンク
describe('the links of the drawn body', () => {
  function links() {
    return item(
      row('a', '', {
        body_model:
          '<p><a data-wikilink href="/pages/notes">notes</a> <a class="hashtag" href="/tags/x">#x</a> <a href="https://example.com/">web</a></p>',
      }),
    );
  }

  const none = { shift: false, ctrl: false, cmd: false };

  // ページへのリンクとタグは、押したキーとともにコアへ渡す。ブラウザはリンク先へ移らない（ADR 083）。
  test('a link to a page and a tag go to the core with the keys held', () => {
    const { wasm } = links();

    let kept: boolean[] = [];
    const reached = reachesThePage('click', () => {
      kept = [
        !fireEvent.click(screen.getByText('notes')),
        !fireEvent.click(screen.getByText('#x'), { shiftKey: true }),
        !fireEvent.click(screen.getByText('notes'), { ctrlKey: true }),
        !fireEvent.click(screen.getByText('notes'), { metaKey: true }),
      ];
    });

    expect(kept).toEqual([true, true, true, true]);
    expect(reached).toBe(false);
    expect(wasm.events).toEqual([
      eventLinkClicked(richLinkPage('/pages/notes'), none),
      eventLinkClicked(richLinkPage('/tags/x'), { ...none, shift: true }),
      eventLinkClicked(richLinkPage('/pages/notes'), { ...none, ctrl: true }),
      eventLinkClicked(richLinkPage('/pages/notes'), { ...none, cmd: true }),
    ]);
  });

  // 外へのリンクは、ブラウザが解いたアドレスでコアへ渡す。
  test('a link out of the journal goes to the core with the address the browser reads', () => {
    const { wasm } = links();

    let kept = false;
    const reached = reachesThePage('click', () => {
      kept = !fireEvent.click(screen.getByText('web'), { shiftKey: true });
    });

    expect([kept, reached]).toEqual([true, false]);
    expect(wasm.events).toEqual([
      eventLinkClicked(richLinkWeb('https://example.com/'), {
        ...none,
        shift: true,
      }),
    ]);
  });

  // 中ボタンのクリックは、Ctrlを押したクリックとしてコアへ渡す。
  test('a middle click goes to the core as a click with Ctrl', () => {
    const { wasm } = links();

    const kept = !fireEvent(
      screen.getByText('notes'),
      new MouseEvent('auxclick', {
        button: 1,
        bubbles: true,
        cancelable: true,
      }),
    );

    expect(kept).toBe(true);
    expect(wasm.events).toEqual([
      eventLinkClicked(richLinkPage('/pages/notes'), { ...none, ctrl: true }),
    ]);
  });

  // 行き先のないリンクと、リンクでない所のクリックは、何もしない。
  test('a link without a target and a click off the links do nothing', () => {
    const opened: unknown[] = [];
    vi.spyOn(window, 'open').mockImplementation((...args) => {
      opened.push(args);
      return null;
    });
    const { wasm } = item(
      row('a', '', {
        body_model: '<p>text <a data-wikilink>page</a> <a>anchor</a></p>',
      }),
    );

    fireEvent.click(screen.getByText('page'));
    fireEvent.click(screen.getByText('anchor'));
    fireEvent.click(screen.getByText(/text/));

    expect(opened).toEqual([]);
    expect(wasm.events).toEqual([]);
  });

  // 脚注の参照を押すと、脚注を画面の中ほどに出す。アプリのリンクとしては開かない。
  test('a footnote reference brings its note to the middle of the screen', () => {
    const scrolled: [string, unknown][] = [];
    Element.prototype.scrollIntoView = function (
      this: Element,
      options?: boolean | ScrollIntoViewOptions,
    ) {
      scrolled.push([this.id, options]);
    };
    const { wasm } = item(
      row('a', '', {
        body_model:
          '<p><sup class="footnote-ref"><a href="#fn-1">1</a></sup></p><div id="fn-1">note</div>',
      }),
    );

    let kept = false;
    const reached = reachesThePage('click', () => {
      kept = !fireEvent.click(screen.getByText('1'));
    });

    expect([kept, reached]).toEqual([true, false]);
    expect(scrolled).toEqual([['fn-1', { block: 'center' }]]);
    expect(wasm.links).toEqual([]);
    Reflect.deleteProperty(Element.prototype, 'scrollIntoView');
  });
});

// 描いた本文に落とした画像
describe('an image dropped on the drawn body', () => {
  function dropped(type: string) {
    const file = new File(['x'], 'image.png', { type });
    return { dataTransfer: { items: [{ type, getAsFile: () => file }] }, file };
  }

  // 画像は、本文の末尾に入る。位置は文字で数える。
  test('goes in at the end of the body, counted in characters', () => {
    const { wasm } = item(row('a', 'a👍'));
    const { dataTransfer, file } = dropped('image/png');

    const kept = !fireEvent.drop(
      box('a').querySelector('app-item-view .item-content') as Element,
      { dataTransfer },
    );

    expect(kept).toBe(true);
    expect(wasm.pastes).toEqual([{ itemId: 'a', file, cursor: 2 }]);
  });

  // 画像でないものは、ブラウザに任せる。
  test('anything else is left to the browser', () => {
    const { wasm } = item(row('a', 'one'));

    const kept = !fireEvent.drop(
      box('a').querySelector('app-item-view .item-content') as Element,
      dropped('text/plain'),
    );

    expect(kept).toBe(false);
    expect(wasm.pastes).toEqual([]);
  });
});

// 編集中の項目
describe('the item in edit', () => {
  function editing(
    body: string,
    changes: Partial<Row> = {},
    options: { request?: FocusRequest | null; holdingKeys?: boolean } = {},
  ) {
    const drawn = item(
      row('a', body, { is_editing: true, ...changes }),
      options,
    );
    const editor = document.getElementById(
      'item-editor-a',
    ) as HTMLTextAreaElement;
    return { ...drawn, editor };
  }

  // 編集欄は本文を持ち、出たときにフォーカスを取る。
  test('the editor holds the body, and takes the focus when it comes', () => {
    const { editor } = editing('one');

    expect(editor.value).toBe('one');
    expect(document.activeElement).toBe(editor);
  });

  // コアの頼んだ位置にカーソルを置き、フォーカスする。同じ番号の頼みには、二度は従わない。
  test('the cursor goes where the core asks, and the same request is not followed twice', () => {
    const { editor, redraw } = editing(
      'a👍b',
      {},
      { request: focusAt(1, 'a', 2) },
    );
    const first = editor.selectionStart;
    editor.setSelectionRange(0, 0);
    redraw(row('a', 'a👍b', { is_editing: true }), focusAt(1, 'a', 2));
    const again = editor.selectionStart;
    redraw(row('a', 'a👍b', { is_editing: true }), focusAt(2, 'a', 1));

    expect([first, again, editor.selectionStart]).toEqual([3, 0, 1]);
  });

  // 位置のない頼みは、カーソルを動かさずにフォーカスだけする。ほかの項目への頼みには従わない。
  test('a request without a place only focuses, and a request for another item is not followed', () => {
    const { editor, redraw } = editing(
      'one',
      {},
      { request: focusAt(1, 'b', 0) },
    );
    editor.setSelectionRange(1, 1);
    editor.blur();
    redraw(row('a', 'one', { is_editing: true }), focusAt(2, 'b', 0));
    const other = document.activeElement === editor;
    redraw(row('a', 'one', { is_editing: true }), focusAt(3, 'a', null));

    expect(other).toBe(false);
    expect(document.activeElement).toBe(editor);
    expect(editor.selectionStart).toBe(1);
  });

  // コアの本文が変わると、編集欄の本文も変わる。貼り付けや取り消しのためである。
  test('a new body of the core goes into the editor, as after a paste or an undo', () => {
    const { editor, redraw } = editing('one');

    redraw(row('a', 'one two', { is_editing: true }));

    expect(editor.value).toBe('one two');
  });

  // 見出しの行は、段に合った大きさの太字になる。
  test('a heading line is bold, at the size of its level', () => {
    const { editor, unmount } = editing('one', {
      heading_line: true,
      heading_level: 2,
    });
    const heading = [editor.style.fontSize, editor.style.fontWeight];
    unmount();
    const { editor: deep } = editing('one', {
      heading_line: true,
      heading_level: 6,
    });

    expect(heading).toEqual(['1.4em', 'bold']);
    expect([deep.style.fontSize, deep.style.fontWeight]).toEqual(['', 'bold']);
  });

  // 見出しの段があっても、見出しの行でなければ、ふつうの文字である。
  test('a line that is not a heading line is plain, whatever its level', () => {
    const { editor } = editing('one', {
      heading_line: false,
      heading_level: 2,
    });

    expect([editor.style.fontSize, editor.style.fontWeight]).toEqual(['', '']);
  });

  // 編集欄を離れると、編集の終わりをコアに知らせる。
  test('leaving the editor tells the core the edit is over', () => {
    const { wasm, editor } = editing('one');

    fireEvent.blur(editor);

    expect(wasm.events).toEqual([{ kind: 'EditFinished', item_id: 'a' }]);
  });

  // 編集欄を押すと、ドラッグを始める。
  test('a press on the editor starts a drag', () => {
    const { wasm, editor } = editing('one');

    fireEvent.mouseDown(editor);

    expect(wasm.drags.map(d => d.kind)).toEqual(['Pressed']);
  });
});

// 編集欄への入力
describe('typing in the editor', () => {
  function editing(body: string) {
    const drawn = item(row('a', body, { is_editing: true }));
    const editor = document.getElementById(
      'item-editor-a',
    ) as HTMLTextAreaElement;
    return { ...drawn, editor };
  }

  /** Types `text` at the end, as the browser does before it tells the editor. */
  function type(editor: HTMLTextAreaElement, text: string, inputType: string) {
    editor.value += text;
    editor.setSelectionRange(editor.value.length, editor.value.length);
    fireEvent.input(editor, { inputType, data: text });
  }

  // 打った文字は、本文とUTF-16のカーソルとともにコアに渡る。カーソルがウィキリンクにあるかも知らせる。
  test('typed text goes to the core with the body and the UTF-16 cursor, and the wiki link is checked', () => {
    const { wasm, editor } = editing('👍');

    type(editor, 'a', 'insertText');

    expect(wasm.typings).toEqual([
      { kind: 'Typed', item_id: 'a', body: '👍a', cursor: 3n },
    ]);
    expect(wasm.events).toEqual([
      { kind: 'WikiQuery', item_id: 'a', context: null },
    ]);
  });

  // コアが本文を変えたら（打った括弧を閉じたら）、編集欄はコアの本文になり、カーソルは打った所に残る。
  test('when the core changes the typed body, the editor takes it, and the cursor stays', () => {
    const { wasm, editor } = editing('');
    // Another row comes first, and keeps its own body
    act(() => wasm.push([{ kind: 'Insert', at: 0, item: row('b', 'other') }]));
    wasm.answerNext([
      { kind: 'Update', at: 1, item: row('a', '[]', { is_editing: true }) },
    ]);

    type(editor, '[', 'insertText');

    expect(editor.value).toBe('[]');
    expect(editor.selectionStart).toBe(1);
  });

  // 打った文字でない変更（貼り付け、削除）は、本文だけをコアに渡す。
  test('a change that is not typed text hands only the body to the core', () => {
    const { wasm, editor } = editing('one');

    type(editor, '!', 'insertFromPaste');

    expect(wasm.typings).toEqual([
      { kind: 'Changed', item_id: 'a', body: 'one!' },
    ]);
  });
});

// 編集欄のキー
describe('the keys of the editor', () => {
  function editing(holdingKeys = false) {
    const drawn = item(row('a', 'one', { is_editing: true }), { holdingKeys });
    const editor = document.getElementById(
      'item-editor-a',
    ) as HTMLTextAreaElement;
    editor.setSelectionRange(3, 3);
    return { ...drawn, editor };
  }

  // アウトラインのキーは、編集欄の状態とともにコアに渡る。コアが受け取れば、編集欄はそのキーを使わない。
  test('a key goes to the core with the state of the editor, and the editor drops it when taken', () => {
    const { wasm, editor } = editing();
    let reached = false;
    const outside = () => (reached = true);
    document.addEventListener('keydown', outside);

    const dropped = !fireEvent.keyDown(editor, { key: 'Enter' });
    document.removeEventListener('keydown', outside);

    expect(dropped).toBe(true);
    expect(reached).toBe(false);
    expect(wasm.presses).toEqual([
      {
        kind: 'InEditor',
        item_id: 'a',
        chord: {
          key: { kind: 'Enter' },
          shift: false,
          ctrl: false,
          alt: false,
          cmd: false,
        },
        editor: {
          text: 'one',
          selection_start: 3n,
          selection_end: 3n,
          on_first_line: false,
          on_last_line: false,
        },
      },
    ]);
  });

  // コアが受け取らないキーと、コアの知らないキーは、編集欄が使う。
  test('a key the core does not take, and one it does not know, are the editor’s', () => {
    const { wasm, editor } = editing();
    wasm.takesKeys = false;

    const left = !fireEvent.keyDown(editor, { key: 'ArrowLeft' });
    const f1 = !fireEvent.keyDown(editor, { key: 'F1' });

    expect([left, f1]).toEqual([false, false]);
    expect(wasm.presses).toHaveLength(1);
  });

  // IMEの変換中のキーは、IMEのものである。
  test('a key of the IME is the IME’s', () => {
    const { wasm, editor } = editing();

    fireEvent.keyDown(editor, { key: 'Enter', isComposing: true });

    expect(wasm.presses).toEqual([]);
  });

  // Shift+Enterは、本文に改行を入れる。
  test('Shift+Enter puts a line break into the body', () => {
    const { wasm, editor } = editing();

    const dropped = !fireEvent.keyDown(editor, {
      key: 'Enter',
      shiftKey: true,
    });

    expect(dropped).toBe(true);
    expect(editor.value).toBe('one\n');
    expect(wasm.typings).toEqual([
      { kind: 'Changed', item_id: 'a', body: 'one\n' },
    ]);
    expect(wasm.presses).toEqual([]);
  });

  // Shiftとほかのキーは、改行でなく、キーとしてコアに渡る。
  test('Shift with another key goes to the core as a key, not as a line break', () => {
    const { wasm, editor } = editing();

    fireEvent.keyDown(editor, { key: 'ArrowDown', shiftKey: true });

    expect(editor.value).toBe('one');
    expect(wasm.presses).toHaveLength(1);
  });

  // Ctrl+Shift+Enterは、改行でなく、キーとしてコアに渡る。
  test('Ctrl+Shift+Enter goes to the core as a key, not as a line break', () => {
    const { wasm, editor } = editing();

    fireEvent.keyDown(editor, { key: 'Enter', shiftKey: true, ctrlKey: true });

    expect(editor.value).toBe('one');
    expect(wasm.presses).toHaveLength(1);
  });

  // コアがキーを預かっている間は、キーをコアに預ける。CtrlとCommandのキーは、預けない。
  test('while the core holds keys, a key is held by it, but not a Ctrl or a Command key', () => {
    const { wasm, editor } = editing(true);

    const dropped = !fireEvent.keyDown(editor, { key: 'x', code: 'KeyX' });
    fireEvent.keyDown(editor, { key: 'z', ctrlKey: true });
    fireEvent.keyDown(editor, { key: 'z', metaKey: true });

    expect(dropped).toBe(true);
    expect(wasm.events).toEqual([
      {
        kind: 'KeyHeld',
        value: {
          key: 'x',
          code: 'KeyX',
          shift_key: false,
          ctrl_key: false,
          alt_key: false,
          meta_key: false,
        },
      },
    ]);
    expect(wasm.presses).toHaveLength(2);
  });

  // コアがもう預かっていなければ、描き直す前でも、キーはコアに預けない。預かっていないコアは、キーを捨てるためである。
  test('a key is not held by the core that no longer holds keys, before the editor is drawn again', () => {
    const { wasm, editor } = editing(true);
    wasm.push(
      [],
      [{ kind: 'Outline', value: outline({ key_relay_holding: false }) }],
    );

    const dropped = !fireEvent.keyDown(editor, { key: 'Enter' });

    expect(dropped).toBe(true);
    expect(wasm.events).toEqual([]);
    expect(wasm.presses).toHaveLength(1);
  });

  // 次の編集欄がフォーカスを待つ間のキーは、その編集欄のために取っておく。
  test('a key pressed while the next editor waits for its focus is kept for it', () => {
    vi.useFakeTimers();
    const { wasm, editor } = editing();
    act(() =>
      wasm.ask(1, { kind: 'FocusEditor', item_id: 'b', replay_keys: [] }),
    );

    const dropped = !fireEvent.keyDown(editor, { key: 'x' });
    vi.useRealTimers();

    expect(dropped).toBe(true);
    expect(wasm.presses).toEqual([]);
  });

  // コアが預かっていても、次の編集欄がフォーカスを待つ間のキーは、その編集欄のために取っておく。コアに預けると、答えのあとの注文で、再生するキーを追い越すためである。
  test('a key pressed while the next editor waits for its focus is kept for it, though the core holds keys', () => {
    vi.useFakeTimers();
    const { wasm, editor } = editing(true);
    act(() =>
      wasm.ask(1, { kind: 'FocusEditor', item_id: 'b', replay_keys: [] }),
    );

    const dropped = !fireEvent.keyDown(editor, { key: 'x' });
    vi.useRealTimers();

    expect(dropped).toBe(true);
    expect(wasm.events).toEqual([]);
  });
});

// 編集欄への貼り付けと、落とした画像
describe('a paste into the editor and an image dropped on it', () => {
  function editing() {
    const drawn = item(row('a', 'a👍b', { is_editing: true }));
    const editor = document.getElementById(
      'item-editor-a',
    ) as HTMLTextAreaElement;
    editor.setSelectionRange(3, 3);
    return { ...drawn, editor };
  }

  function clipboard(text: string, html = '', image: File | null = null) {
    return {
      clipboardData: {
        items:
          image === null ? [] : [{ type: image.type, getAsFile: () => image }],
        getData: (type: string) =>
          ({ 'text/plain': text, 'text/html': html })[type] ?? '',
      },
    };
  }

  // 貼った文字は、HTMLと、文字で数えたカーソルとともにコアに渡る。Windowsの改行は、\nになる。
  test('pasted text goes to the core with its HTML and the cursor in characters, with the breaks of Windows as \\n', () => {
    const { wasm, editor } = editing();

    const dropped = !fireEvent.paste(
      editor,
      clipboard('x\r\ny\rz', '<b>x</b>'),
    );

    expect(dropped).toBe(true);
    expect(wasm.events).toEqual([
      {
        kind: 'PasteText',
        item_id: 'a',
        text: 'x\ny\nz',
        html: '<b>x</b>',
        cursor: 2n,
        raw: false,
      },
    ]);
  });

  // Ctrl+Shift+Vの貼り付けは、Markdownにせずにそのまま入る。Shiftを放すか、編集欄を離れれば、ふつうの貼り付けに戻る。
  test('a paste with Shift held goes in as it is, until Shift is let go or the editor is left', () => {
    const { wasm, editor } = editing();
    wasm.takesKeys = false;
    const raw = () => {
      fireEvent.paste(editor, clipboard('x'));
      const event = wasm.events.at(-1);
      return event?.kind === 'PasteText' ? event.raw : null;
    };

    fireEvent.keyDown(editor, { key: 'v', ctrlKey: true, shiftKey: true });
    const held = raw();
    fireEvent.keyUp(editor, { key: 'Shift', shiftKey: false });
    const letGo = raw();
    fireEvent.keyDown(editor, { key: 'Shift', shiftKey: true });
    fireEvent.blur(editor);
    const left = raw();

    expect([held, letGo, left]).toEqual([true, false, false]);
  });

  // 貼った画像は、カーソルの位置に入る。
  test('a pasted image goes in at the cursor', () => {
    const { wasm, editor } = editing();
    const image = new File(['x'], 'image.png', { type: 'image/png' });

    fireEvent.paste(editor, clipboard('', '', image));

    expect(wasm.pastes).toEqual([{ itemId: 'a', file: image, cursor: 2 }]);
    expect(wasm.events).toEqual([]);
  });

  // 落とした画像は、落とした位置に入る。画像でないものは、ブラウザに任せる。
  test('a dropped image goes in where it fell, and anything else is left to the browser', () => {
    const { wasm, editor } = editing();
    const image = new File(['x'], 'image.png', { type: 'image/png' });
    Object.defineProperty(document, 'caretPositionFromPoint', {
      configurable: true,
      value: () => ({ offsetNode: editor, offset: 1 }),
    });

    const dropped = !fireEvent.drop(editor, {
      dataTransfer: { items: [{ type: 'image/png', getAsFile: () => image }] },
    });
    const text = !fireEvent.drop(editor, {
      dataTransfer: { items: [{ type: 'text/plain', getAsFile: () => null }] },
    });

    expect([dropped, text]).toEqual([true, false]);
    expect(wasm.pastes).toEqual([{ itemId: 'a', file: image, cursor: 1 }]);
  });
});

// 編集欄の高さ
describe('the height of the editor', () => {
  // jsdom lays nothing out: the text of an editor is 10px high for each letter
  const scrollHeight = Object.getOwnPropertyDescriptor(
    HTMLElement.prototype,
    'scrollHeight',
  );
  beforeEach(() => {
    Object.defineProperty(HTMLTextAreaElement.prototype, 'scrollHeight', {
      configurable: true,
      get(this: HTMLTextAreaElement) {
        return this.value.length * 10;
      },
    });
  });
  afterEach(() => {
    Reflect.deleteProperty(HTMLTextAreaElement.prototype, 'scrollHeight');
    if (scrollHeight !== undefined) {
      Object.defineProperty(
        HTMLElement.prototype,
        'scrollHeight',
        scrollHeight,
      );
    }
  });

  function editing(body: string, request: FocusRequest | null = null) {
    const drawn = item(row('a', body, { is_editing: true }), { request });
    const editor = document.getElementById(
      'item-editor-a',
    ) as HTMLTextAreaElement;
    return { ...drawn, editor };
  }

  // 編集欄は、出たときに本文の高さになる。コアの頼みでフォーカスしたときも同じである。
  test('the editor has the height of its text when it comes, also at a request of the core', () => {
    const { editor, unmount } = editing('one');
    const plain = editor.style.height;
    unmount();
    const { editor: asked } = editing('three', focusAt(1, 'a', 0));

    expect([plain, asked.style.height]).toEqual(['30px', '50px']);
  });

  // コアの頼みが来るたびに、高さを合わせ直す。
  test('each request of the core fits the height again', () => {
    const { editor, redraw } = editing('one');
    editor.value = 'one two';

    redraw(row('a', 'one', { is_editing: true }), focusAt(1, 'a', 0));

    expect(editor.style.height).toBe('70px');
  });

  // コアの本文が変わったとき、文字を打ったとき、改行を入れたときに、高さを合わせる。
  test('the height follows a new body of the core, typed text and a line break', () => {
    const { wasm, editor, redraw } = editing('one');
    redraw(row('a', 'one two', { is_editing: true }));
    const changed = editor.style.height;
    // The core takes the typed text as it is
    wasm.answerNext([
      {
        kind: 'Update',
        at: 0,
        item: row('a', 'one two!', { is_editing: true }),
      },
    ]);
    editor.value += '!';
    fireEvent.input(editor, { inputType: 'insertText', data: '!' });
    const typed = editor.style.height;
    editor.setSelectionRange(8, 8);
    fireEvent.keyDown(editor, { key: 'Enter', shiftKey: true });

    expect([changed, typed, editor.style.height]).toEqual([
      '70px',
      '80px',
      '90px',
    ]);
  });
});
