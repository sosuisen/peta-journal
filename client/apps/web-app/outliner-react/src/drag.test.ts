import { afterEach, describe, expect, test } from 'vitest';
import { startDrag } from './drag';
import { shell } from './test/screens';

afterEach(() => {
  Reflect.deleteProperty(document, 'elementFromPoint');
  Reflect.deleteProperty(document, 'caretPositionFromPoint');
  document.body.replaceChildren();
});

/** The item `id` on the page, as the outline draws it, with its editor holding `text`. */
function itemOf(id: string, text = ''): HTMLTextAreaElement {
  const host = document.createElement('app-outline-item');
  const box = document.createElement('div');
  box.id = id;
  box.className = 'outline-item';
  const editor = document.createElement('textarea');
  editor.id = `item-editor-${id}`;
  editor.value = text;
  box.append(editor);
  host.append(box);
  document.body.append(host);
  return editor;
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
