import { act, fireEvent } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import type { Row } from './border/peta-types';
import Outline from './Outline';
import { draw, focusAt, outline, row, shell } from './test/screens';

// jsdom lays nothing out: the list is 600px high with 3000px of content. The header and each
// row are 28px, the height the outline gives a row it has not measured yet
const HEIGHTS = ['offsetHeight', 'clientHeight', 'scrollHeight'] as const;
const kept = HEIGHTS.map(name =>
  Object.getOwnPropertyDescriptor(HTMLElement.prototype, name),
);
const heightOf = (element: Element) =>
  element.classList.contains('outline-viewport') ? 600 : 28;
beforeEach(() => {
  for (const name of HEIGHTS) {
    Object.defineProperty(HTMLElement.prototype, name, {
      configurable: true,
      get(this: HTMLElement) {
        return name === 'scrollHeight' && heightOf(this) === 600
          ? 3000
          : heightOf(this);
      },
    });
  }
  vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(
    function (this: Element) {
      return new DOMRect(0, 0, 800, heightOf(this));
    },
  );
});
afterEach(() => {
  HEIGHTS.forEach((name, i) => {
    const descriptor = kept[i];
    if (descriptor !== undefined) {
      Object.defineProperty(HTMLElement.prototype, name, descriptor);
    }
  });
  vi.restoreAllMocks();
});

/** Draws the outline of `rows`, with `selecting` items selected. */
function drawn(rows: Row[], selecting = false) {
  const ui = shell();
  ui.wasm.push(
    [{ kind: 'Reset', items: rows }],
    [{ kind: 'Outline', value: outline({ selection_mode: selecting }) }],
  );
  const view = draw(
    ui,
    <Outline header={<h2>title</h2>} footer={<p>links</p>} />,
  );
  const list = view.container.querySelector(
    'cdk-virtual-scroll-viewport',
  ) as HTMLElement;
  return { ...ui, ...view, list };
}

function shown(container: HTMLElement): string[] {
  return [...container.querySelectorAll<HTMLElement>('.outline-row')].map(
    r => r.dataset.rowId ?? '',
  );
}

// アウトライン
describe('the outline', () => {
  // 行は、コアの順に、見出しと足元の間に並ぶ。日付の見出しのある行は、それを出す。
  test('the rows come in the order of the core, between the header and the footer, with their date headers', () => {
    const { container } = drawn([
      row('a', 'one', { date_header: '2026-11-23' }),
      row('b', 'two'),
    ]);

    expect(shown(container)).toEqual(['a', 'b']);
    expect(
      [...container.querySelectorAll('.date-header')].map(h => h.textContent),
    ).toEqual(['2026-11-23']);
    const parts = [
      ...(container.querySelector('cdk-virtual-scroll-viewport')?.children ??
        []),
    ].map(e => e.className);
    expect(parts).toEqual([
      'outline-header',
      'cdk-virtual-scroll-content-wrapper',
      'outline-footer',
    ]);
  });

  // 見出しと足元を渡さなければ、その箱もない。
  test('without a header and a footer there are no boxes for them', () => {
    const { container } = draw(shell(), <Outline />);

    expect(container.querySelector('.outline-header')).toBeNull();
    expect(container.querySelector('.outline-footer')).toBeNull();
  });

  // コアが行を変えると、出す行も変わる。
  test('the rows follow the changes of the core', () => {
    const { container, wasm } = drawn([row('a'), row('b')]);

    act(() =>
      wasm.push([
        { kind: 'Remove', at: 0 },
        { kind: 'Insert', at: 1, item: row('c') },
      ]),
    );

    expect(shown(container)).toEqual(['b', 'c']);
  });

  // 編集中の行だけが、フォーカスの頼みとキーの預かりを受け取る。
  test('only the row in edit gets the focus request and the holding of keys', () => {
    const { wasm } = drawn([row('a'), row('b', 'two', { is_editing: true })]);
    act(() =>
      wasm.push(
        [],
        [
          {
            kind: 'Outline',
            value: outline({
              focus_request: focusAt(1, 'b', 0),
              key_relay_holding: true,
            }),
          },
        ],
      ),
    );
    const editor = document.getElementById(
      'item-editor-b',
    ) as HTMLTextAreaElement;

    fireEvent.keyDown(editor, { key: 'x' });

    expect(document.activeElement).toBe(editor);
    expect(wasm.events.map(e => e.kind)).toEqual(['KeyHeld']);
  });

  // アウトラインの状態がまだなければ、編集中の行もキーを預からない。
  test('without the state of the outline the row in edit holds no keys', () => {
    const ui = shell();
    ui.wasm.push([
      { kind: 'Reset', items: [row('a', 'one', { is_editing: true })] },
    ]);
    draw(ui, <Outline />);

    fireEvent.keyDown(document.getElementById('item-editor-a') as Element, {
      key: 'x',
    });

    expect(ui.wasm.events).toEqual([]);
    expect(ui.wasm.presses).toHaveLength(1);
  });

  // Shiftを押したホイールは、行の拡大と縮小をコアに頼む。上に回すと拡大である。
  test('the wheel with Shift asks the core to zoom the row, in when turned up', () => {
    const { container, wasm } = drawn([row('a')]);
    const a = container.querySelector('.outline-row') as Element;

    fireEvent.wheel(a, { deltaY: -1, shiftKey: true });
    fireEvent.wheel(a, { deltaY: 1, shiftKey: true });
    fireEvent.wheel(a, { deltaY: 1 });

    expect(wasm.events).toEqual([
      { kind: 'ZoomStep', item_id: 'a', up: true },
      { kind: 'ZoomStep', item_id: 'a', up: false },
    ]);
  });

  // 動かすと、上から動いたかと、見える行と描いた行の範囲をコアに知らせる。
  test('a scroll tells the core whether it left the top, and which rows are seen and drawn', () => {
    const { list, wasm } = drawn([row('a'), row('b'), row('c')]);

    list.scrollTop = 0;
    fireEvent.scroll(list);
    // The header is 28px high, so the second row is at the top from 56px down
    list.scrollTop = 56;
    fireEvent.scroll(list);

    expect(wasm.events).toEqual([
      {
        kind: 'OutlineScrolled',
        scrolled_from_top: false,
        top_visible_index: 0n,
        rendered_start: 0n,
        rendered_end: 3n,
      },
      {
        kind: 'OutlineScrolled',
        scrolled_from_top: true,
        top_visible_index: 1n,
        rendered_start: 0n,
        rendered_end: 3n,
      },
    ]);
  });

  // 行がなければ、見える行も描いた行もない。
  test('without rows no row is seen or drawn', () => {
    const { list, wasm } = drawn([]);

    fireEvent.scroll(list);

    expect(wasm.events).toEqual([
      {
        kind: 'OutlineScrolled',
        scrolled_from_top: false,
        top_visible_index: 0n,
        rendered_start: 0n,
        rendered_end: 0n,
      },
    ]);
  });

  // 描くのは画面の近くの行だけで、上下の余白が残りの行の高さになる。下には、一覧の高さの半分の空きがある。
  test('only the rows near the screen are drawn, and the padding stands for the rest', () => {
    const rows = Array.from({ length: 100 }, (_, i) => row(`r${i}`));
    const { container, list } = drawn(rows);
    const wrapper = container.querySelector(
      '.cdk-virtual-scroll-content-wrapper',
    ) as HTMLElement;
    const heights = () => ({
      top: parseFloat(wrapper.style.paddingTop),
      drawn: shown(container).length * 28,
      bottom: parseFloat(wrapper.style.paddingBottom),
    });
    const atTop = heights();

    list.scrollTop = 1500;
    fireEvent.scroll(list);
    const below = heights();

    expect(atTop.top).toBe(0);
    expect(below.top).toBeGreaterThan(0);
    expect(shown(container)).not.toContain('r0');
    for (const { top, drawn, bottom } of [atTop, below]) {
      expect(drawn).toBeLessThan(100 * 28);
      expect(top + drawn + bottom).toBe(100 * 28 + 300);
    }
  });

  // 項目までのスクロールは、その項目を一覧の高さの1割だけ下に置く。滑らかにするかは、コアが決める。
  test('a scroll to an item puts it a tenth of the list down, smooth as the core says', () => {
    const { list, asked } = drawn([row('a'), row('b'), row('c'), row('d')]);
    const scrolled: ScrollToOptions[] = [];
    list.scrollTo = ((options: ScrollToOptions) =>
      scrolled.push(options)) as typeof list.scrollTo;

    asked.scroller?.('d', true);
    asked.scroller?.('a', false);
    asked.scroller?.('x', false);

    expect(scrolled).toEqual([
      { top: 52, behavior: 'smooth' },
      { top: 0, behavior: 'auto' },
    ]);
  });

  // 一覧がまだ1行も描いていない行へのスクロールは、一番上へのスクロールになる。コアが行を足したのと同じ変化で頼んだときである。
  test('a scroll to a row the list has not drawn yet goes to the top', () => {
    const { list, wasm, asked } = drawn([]);
    const scrolled: ScrollToOptions[] = [];
    list.scrollTo = ((options: ScrollToOptions) =>
      scrolled.push(options)) as typeof list.scrollTo;
    // The core has the row, and React has not drawn it yet
    wasm.push([{ kind: 'Insert', at: 0, item: row('a') }]);

    asked.scroller?.('a', false);

    expect(scrolled).toEqual([{ top: 0, behavior: 'auto' }]);
  });

  // アウトラインがなくなると、スクロールの役も外れる。
  test('without the outline nothing scrolls to an item', () => {
    const { asked, unmount } = drawn([row('a')]);

    unmount();

    expect(asked.scroller).toBeNull();
  });
});

// 項目を選んでいる間のキーとクリック
describe('the keys and clicks while items are selected', () => {
  const chord = (key: string, ctrl = false) => ({
    key:
      key === 'Delete'
        ? { kind: 'Delete' as const }
        : { kind: 'Letter' as const, value: key },
    shift: false,
    ctrl,
    alt: false,
    cmd: false,
  });

  // ページのキーは、選択のキーとしてコアに渡る。コアが受け取れば、ブラウザには渡さない。
  test('a key goes to the core as a key of the selection, and the browser drops it when taken', () => {
    const { wasm } = drawn([row('a')], true);

    const taken = !fireEvent.keyDown(document.body, { key: 'Delete' });
    wasm.takesKeys = false;
    const left = !fireEvent.keyDown(document.body, { key: 'Delete' });

    expect([taken, left]).toEqual([true, false]);
    expect(wasm.presses).toEqual([
      { kind: 'InSelection', chord: chord('Delete') },
      { kind: 'InSelection', chord: chord('Delete') },
    ]);
  });

  // Ctrl+CとCtrl+Xは、キーをコアに渡す前に、クリップボードへの書き込みを始める。
  test('Ctrl+C and Ctrl+X begin the write of the clipboard before the key goes to the core', () => {
    const { wasm } = drawn([row('a')], true);

    fireEvent.keyDown(document.body, { key: 'c', ctrlKey: true });
    fireEvent.keyDown(document.body, { key: 'X', metaKey: true });
    fireEvent.keyDown(document.body, { key: 'v', ctrlKey: true });
    fireEvent.keyDown(document.body, { key: 'c' });

    expect(wasm.copies).toBe(2);
    expect(wasm.presses).toHaveLength(4);
  });

  // コピーとカットの出来事は、Ctrl+CとCtrl+Xとしてコアに渡る。ブラウザは何もコピーしない。
  test('a copy and a cut go to the core as Ctrl+C and Ctrl+X, and the browser copies nothing', () => {
    const { wasm } = drawn([row('a')], true);

    const copied = !fireEvent.copy(document.body);
    const cut = !fireEvent.cut(document.body);

    expect([copied, cut]).toEqual([true, true]);
    expect(wasm.copies).toBe(2);
    expect(wasm.presses).toEqual([
      { kind: 'InSelection', chord: chord('c', true) },
      { kind: 'InSelection', chord: chord('x', true) },
    ]);
  });

  // 選択の外のクリックは、コアに知らせる。
  test('a click outside is told to the core', () => {
    const { wasm } = drawn([row('a')], true);

    fireEvent.click(document.body);

    expect(wasm.events).toEqual([{ kind: 'SelectionClickOutside' }]);
  });

  // 確認のダイアログのキーとクリックは、ダイアログのものである。コアの知らないキーも、渡さない。
  test('the keys and clicks of the confirm dialog are its own, and a key the core does not know is not sent', () => {
    const { wasm } = drawn([row('a')], true);
    const overlay = document.createElement('div');
    overlay.className = 'confirm-overlay';
    const button = document.createElement('button');
    overlay.append(button);
    document.body.append(overlay);

    fireEvent.keyDown(button, { key: 'Delete' });
    fireEvent.click(button);
    fireEvent.keyDown(document.body, { key: 'F1' });

    expect(wasm.presses).toEqual([]);
    expect(wasm.events).toEqual([]);
    overlay.remove();
  });

  // 選択をやめると、ページのクリックとコピーはコアに届かない。
  test('once the selection ends, the clicks and copies of the page do not reach the core', () => {
    const { wasm } = drawn([row('a')], true);
    act(() =>
      wasm.push(
        [],
        [{ kind: 'Outline', value: outline({ selection_mode: false }) }],
      ),
    );

    fireEvent.click(document.body);
    fireEvent.copy(document.body);
    fireEvent.keyDown(document.body, { key: 'Delete' });

    expect(wasm.events).toEqual([]);
    expect(wasm.presses).toEqual([]);
  });
});

// 何も選んでいない間のキー
describe('the keys while nothing is selected', () => {
  // ページに来た取り消しとやり直しは、コアに渡る。選択を消した直後のためである。
  test('Undo and Redo that reach the page go to the core, as right after a deletion', () => {
    const { wasm } = drawn([row('a')]);

    fireEvent.keyDown(document.body, { key: 'z', ctrlKey: true });
    fireEvent.keyDown(document.body, { key: 'Y', metaKey: true });

    expect(
      wasm.presses.map(p => (p.kind === 'InSelection' ? p.chord.key : null)),
    ).toEqual([
      { kind: 'Letter', value: 'z' },
      { kind: 'Letter', value: 'y' },
    ]);
  });

  // ほかのキーと、ページでなく部品に来たキーは、渡さない。
  test('other keys, and keys that reach a component, are not sent', () => {
    const { wasm } = drawn([row('a')]);
    const input = document.createElement('input');
    document.body.append(input);

    fireEvent.keyDown(document.body, { key: 'Delete' });
    fireEvent.keyDown(document.body, { key: 'z' });
    fireEvent.keyDown(input, { key: 'z', ctrlKey: true });

    expect(wasm.presses).toEqual([]);
    input.remove();
  });

  // アウトラインがなくなると、ページのキーは聞かない。
  test('without the outline the page keys are off', () => {
    const { wasm, unmount } = drawn([row('a')]);

    unmount();
    fireEvent.keyDown(document.body, { key: 'z', ctrlKey: true });

    expect(wasm.presses).toEqual([]);
  });
});
