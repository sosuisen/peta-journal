import { afterEach, describe, expect, test, vi } from 'vitest';
import { startDrag } from './drag';
import { shell } from './test/screens';

afterEach(() => {
  vi.useRealTimers();
  Reflect.deleteProperty(document, 'elementFromPoint');
  Reflect.deleteProperty(document, 'caretPositionFromPoint');
  document.body.replaceChildren();
});

/**
 * The item `id` in `list`, as the outline draws it, with its editor holding `text`. The page
 * itself stands for a list when there is no outline.
 */
function itemOf(
  id: string,
  text = '',
  list: Element = document.body,
): HTMLTextAreaElement {
  const host = document.createElement('app-outline-item');
  const box = document.createElement('div');
  box.id = id;
  box.className = 'outline-item';
  const editor = document.createElement('textarea');
  editor.id = `item-editor-${id}`;
  editor.value = text;
  box.append(editor);
  host.append(box);
  list.append(host);
  return editor;
}

/**
 * The outline seen from 100 to 300 on the screen, scrolled by `scrolled`. jsdom scrolls
 * nothing, so the outline keeps the place it is given.
 */
function outlineOf(scrolled: number): HTMLElement {
  const viewport = document.createElement('cdk-virtual-scroll-viewport');
  viewport.className = 'outline-viewport';
  viewport.getBoundingClientRect = () => new DOMRect(0, 100, 100, 200);
  let top = scrolled;
  Object.defineProperty(viewport, 'scrollTop', {
    configurable: true,
    get: () => top,
    set: (value: number) => {
      top = value;
    },
  });
  document.body.append(viewport);
  return viewport;
}

/** The mouse moves to `y` with its button held. */
function moveTo(y: number) {
  document.dispatchEvent(new MouseEvent('mousemove', { clientY: y }));
}

/** jsdom lays nothing out: the tests say what is under the mouse. */
function under(element: Element | null, offsetNode: Node, offset: number) {
  Object.defineProperty(document, 'elementFromPoint', {
    configurable: true,
    value: () => element,
  });
  Object.defineProperty(document, 'caretPositionFromPoint', {
    configurable: true,
    value: () => ({ offsetNode, offset }),
  });
}

// 項目の文字からのドラッグ
describe('a drag from the text of an item', () => {
  // 押した所、動いた所、放した所を、順にコアに渡す。動いた所では、マウスの下の項目も渡す。
  test('the press, each move and the release go to the core, with the item under the mouse', () => {
    const { core, wasm } = shell();
    const a = itemOf('a');
    itemOf('b');
    a.getBoundingClientRect = () => new DOMRect(0, 10, 100, 20);

    startDrag(core, 'a', a, null);
    under(document.getElementById('b'), a, 0);
    document.dispatchEvent(new MouseEvent('mousemove', { clientY: 50 }));
    under(document.body, a, 0);
    document.dispatchEvent(new MouseEvent('mousemove', { clientY: 60 }));
    document.dispatchEvent(new MouseEvent('mouseup'));
    document.dispatchEvent(new MouseEvent('mousemove', { clientY: 70 }));

    expect(wasm.drags).toEqual([
      { kind: 'Pressed', item_id: 'a', top: 10, bottom: 30 },
      { kind: 'Moved', y: 50, item_under: 'b' },
      { kind: 'Moved', y: 60, item_under: null },
      { kind: 'Released' },
    ]);
  });

  // 新しいドラッグは、前のドラッグを終える。前のドラッグの動きは、もうコアに渡らない。
  test('a new drag ends the one before', () => {
    const { core, wasm } = shell();
    const a = itemOf('a');
    under(null, a, 0);

    startDrag(core, 'a', a, null);
    startDrag(core, 'a', a, null);
    document.dispatchEvent(new MouseEvent('mousemove', { clientY: 5 }));
    document.dispatchEvent(new MouseEvent('mouseup'));

    expect(wasm.drags.map(d => d.kind)).toEqual([
      'Pressed',
      'Pressed',
      'Moved',
      'Released',
    ]);
  });

  // 描いた項目からのドラッグは、コアが受け取らない間、押した文字からマウスの下の文字までを、向きとともに選ぶ。
  test('from the drawn item the drag selects from the pressed letter, with its direction', () => {
    const { core, wasm } = shell();
    const editor = itemOf('a', 'a👍bc');
    wasm.takesDrags = false;

    startDrag(core, 'a', editor, 2);
    under(null, editor, 5);
    document.dispatchEvent(new MouseEvent('mousemove'));
    const forward = [
      editor.selectionStart,
      editor.selectionEnd,
      editor.selectionDirection,
    ];
    under(null, editor, 0);
    document.dispatchEvent(new MouseEvent('mousemove'));
    document.dispatchEvent(new MouseEvent('mouseup'));

    expect(forward).toEqual([3, 5, 'forward']);
    expect([
      editor.selectionStart,
      editor.selectionEnd,
      editor.selectionDirection,
    ]).toEqual([0, 3, 'backward']);
  });

  // コアがドラッグを受け取って項目を選ぶ間は、文字を選ばない。編集欄からのドラッグも、文字の選択をブラウザに任せる。
  test('no text is selected while the core takes the drag, nor from the editor', () => {
    const { core, wasm } = shell();
    const editor = itemOf('a', 'abc');
    editor.setSelectionRange(1, 1);
    under(null, editor, 3);

    startDrag(core, 'a', editor, 0);
    document.dispatchEvent(new MouseEvent('mousemove'));
    wasm.takesDrags = false;
    startDrag(core, 'a', editor, null);
    document.dispatchEvent(new MouseEvent('mousemove'));
    document.dispatchEvent(new MouseEvent('mouseup'));

    expect([editor.selectionStart, editor.selectionEnd]).toEqual([1, 1]);
  });

  // 編集欄がまだ画面にないときは、何も選ばない。
  test('nothing is selected while the editor is not on the screen', () => {
    const { core, wasm } = shell();
    const view = document.createElement('div');
    document.body.append(view);
    under(null, view, 0);
    wasm.takesDrags = false;

    startDrag(core, 'a', view, 0);
    document.dispatchEvent(new MouseEvent('mousemove'));
    document.dispatchEvent(new MouseEvent('mouseup'));

    expect(wasm.drags.map(d => d.kind)).toEqual([
      'Pressed',
      'Moved',
      'Released',
    ]);
  });
});

// 項目を選ぶドラッグの、アウトラインの端でのスクロール
describe('the scroll at an edge of the outline while a drag selects items', () => {
  // コアが項目を選ぶ間、マウスを下の端に置くと、アウトラインが1歩ずつ下にスクロールする。1歩ごとに、止まったマウスの下の項目をコアに渡す。ボタンを放すと止まる。
  test('the mouse held at the bottom edge scrolls the outline step by step, telling the core the item under it', () => {
    vi.useFakeTimers();
    const { core, wasm } = shell();
    const outline = outlineOf(0);
    const a = itemOf('a', '', outline);
    itemOf('b', '', outline);
    itemOf('c', '', outline);
    under(document.getElementById('b'), a, 0);

    startDrag(core, 'a', a, null);
    moveTo(295);
    // The core has nothing more to do while the selection reaches the same item
    wasm.takesDrags = false;
    vi.advanceTimersByTime(16);
    const first = outline.scrollTop;
    under(document.getElementById('c'), a, 0);
    vi.advanceTimersByTime(16);
    const second = outline.scrollTop;
    document.dispatchEvent(new MouseEvent('mouseup'));
    vi.advanceTimersByTime(48);

    expect([first, second, outline.scrollTop]).toEqual([5, 10, 10]);
    expect(wasm.drags.slice(1)).toEqual([
      { kind: 'Moved', y: 295, item_under: 'b' },
      { kind: 'Moved', y: 295, item_under: 'b' },
      { kind: 'Moved', y: 295, item_under: 'c' },
      { kind: 'Released' },
    ]);
  });

  // 上の端では上にスクロールし、一番上より上には行かない。
  test('the top edge scrolls the outline up, no further than its top', () => {
    vi.useFakeTimers();
    const { core } = shell();
    const outline = outlineOf(7);
    const a = itemOf('a', '', outline);
    under(null, a, 0);

    startDrag(core, 'a', a, null);
    moveTo(105);
    vi.advanceTimersByTime(16);
    const first = outline.scrollTop;
    vi.advanceTimersByTime(16);

    expect([first, outline.scrollTop]).toEqual([2, 0]);
  });

  // 端でマウスを動かし続けても、1歩の幅は変わらない。マウスが端から離れると、スクロールは止まる。端に戻ると、また始まる。
  test('moves at the edge keep one step at a time, and the scroll stops off the edge and starts again at it', () => {
    vi.useFakeTimers();
    const { core } = shell();
    const outline = outlineOf(0);
    const a = itemOf('a', '', outline);
    under(null, a, 0);

    startDrag(core, 'a', a, null);
    moveTo(295);
    moveTo(296);
    vi.advanceTimersByTime(16);
    moveTo(200);
    vi.advanceTimersByTime(48);
    const left = outline.scrollTop;
    moveTo(295);
    vi.advanceTimersByTime(16);

    expect([left, outline.scrollTop]).toEqual([5, 10]);
  });

  // 文字を選ぶ間は、端でもスクロールしない。新しいドラッグは、前のドラッグのスクロールを止める。
  test('no scroll while the drag selects letters, and a new drag stops the scroll before', () => {
    vi.useFakeTimers();
    const { core, wasm } = shell();
    const outline = outlineOf(0);
    const a = itemOf('a', '', outline);
    under(null, a, 0);
    wasm.takesDrags = false;

    startDrag(core, 'a', a, null);
    moveTo(295);
    vi.advanceTimersByTime(48);
    const letters = outline.scrollTop;
    wasm.takesDrags = true;
    startDrag(core, 'a', a, null);
    moveTo(295);
    startDrag(core, 'a', a, null);
    vi.advanceTimersByTime(48);

    expect([letters, outline.scrollTop]).toEqual([0, 0]);
  });
});
