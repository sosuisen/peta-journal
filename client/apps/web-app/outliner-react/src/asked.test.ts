import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { AskedOps, deferredKey } from './asked';
import type { DeferredKey } from './border/peta-types';
import { outline, shell } from './test/screens';

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  document.body.replaceChildren();
});

/** The editor of the item `itemId` on the page, with `text` and the cursor at its end. */
function editorOf(itemId: string, text = ''): HTMLTextAreaElement {
  const host = document.createElement('app-item-editor');
  const el = document.createElement('textarea');
  // jsdom lays nothing out: the text of the editor is 10px high for each letter
  Object.defineProperty(el, 'scrollHeight', {
    get: () => el.value.length * 10,
  });
  el.id = `item-editor-${itemId}`;
  el.value = text;
  el.setSelectionRange(text.length, text.length);
  host.append(el);
  document.body.append(host);
  return el;
}

function held(key: string, changes: Partial<DeferredKey> = {}): DeferredKey {
  return {
    key,
    code: key,
    shift_key: false,
    ctrl_key: false,
    alt_key: false,
    meta_key: false,
    ...changes,
  };
}

// 編集欄へのフォーカス
describe('the focus of an editor', () => {
  // 描いたあとに項目の編集欄へフォーカスし、番号でDoneと答える。
  test('the editor of the item gets the focus after the render, and Done answers its number', () => {
    const { wasm } = shell();
    const el = editorOf('a', 'one');

    wasm.ask(4, { kind: 'FocusEditor', item_id: 'a', replay_keys: [] });
    const before = document.activeElement;
    vi.runAllTimers();

    expect(before).not.toBe(el);
    expect(document.activeElement).toBe(el);
    expect(el.style.height).toBe('30px');
    expect(wasm.answers).toEqual([{ id: 4, output: { kind: 'Done' } }]);
  });

  // 項目の名前がなければ、画面の編集欄へフォーカスする。
  test('without an item the editor on the screen gets the focus', () => {
    const { wasm } = shell();
    const el = editorOf('b');

    wasm.ask(1, { kind: 'FocusEditor', item_id: null, replay_keys: [] });
    vi.runAllTimers();

    expect(document.activeElement).toBe(el);
  });

  // 編集欄が画面になくても、Doneと答える。コアが待ち続けないためである。
  test('Done answers even when the editor is not on the screen', () => {
    const { wasm } = shell();

    wasm.ask(2, { kind: 'FocusEditor', item_id: 'a', replay_keys: [] });
    vi.runAllTimers();

    expect(wasm.answers).toEqual([{ id: 2, output: { kind: 'Done' } }]);
  });
});

// 預けたキーの再生
describe('the replay of the held keys', () => {
  // 文字のキーは選択範囲に打ち、消すキーは範囲を消し、動かすキーはカーソルを動かす。変わった本文は、コアに渡る。
  test('a letter types, a delete deletes and a move moves, and the changed text goes to the core', () => {
    const { wasm } = shell();
    const el = editorOf('a', 'xy');
    const answers = [
      { kind: 'MoveTo' as const, value: 1n },
      { kind: 'Type' as const, value: 'ab' },
      { kind: 'Delete' as const, from: 0n, to: 1n },
    ];
    wasm.replay = () => answers.shift() ?? { kind: 'Press' };

    wasm.ask(1, {
      kind: 'FocusEditor',
      item_id: 'a',
      replay_keys: [held('Home'), held('a'), held('Backspace')],
    });
    vi.runAllTimers();

    expect(el.value).toBe('aby');
    expect(el.style.height).toBe('30px');
    expect(wasm.replays.map(r => [r.text, r.start, r.end])).toEqual([
      ['xy', 2, 2],
      ['xy', 1, 1],
      ['xaby', 3, 3],
    ]);
    expect(wasm.typings).toEqual([
      { kind: 'Changed', item_id: 'a', body: 'xaby' },
      { kind: 'Changed', item_id: 'a', body: 'aby' },
    ]);
    expect(el.selectionStart).toBe(0);
  });

  // 文字を打ったあとは、カーソルがウィキリンクの中にあるかをコアに知らせる。
  test('after a replayed letter the core is told about the wiki link at the cursor', () => {
    const { wasm } = shell();
    editorOf('a');
    wasm.replay = () => ({ kind: 'Type', value: 'x' });

    wasm.ask(1, {
      kind: 'FocusEditor',
      item_id: 'a',
      replay_keys: [held('x')],
    });
    vi.runAllTimers();

    expect(wasm.events).toEqual([
      { kind: 'WikiQuery', item_id: 'a', context: null },
    ]);
  });

  // ほかのキーは、修飾キーとともに、編集欄に押される。
  test('any other key is pressed in the editor, with its modifiers', () => {
    const { wasm } = shell();
    const el = editorOf('a');
    const pressed: KeyboardEvent[] = [];
    el.addEventListener('keydown', event => pressed.push(event));

    wasm.ask(1, {
      kind: 'FocusEditor',
      item_id: 'a',
      replay_keys: [held('Tab', { shift_key: true, alt_key: true })],
    });
    vi.runAllTimers();

    expect(pressed.map(deferredKey)).toEqual([
      held('Tab', { shift_key: true, alt_key: true }),
    ]);
    expect([pressed[0]?.bubbles, pressed[0]?.cancelable]).toEqual([true, true]);
  });

  // コアは、注文に答えるまでキーを預かり続ける（ADR 087）。答えてから再生するので、キーは編集欄に入る。
  test('the keys play into the editor though the core holds keys until the answer', () => {
    const { wasm } = shell();
    const el = editorOf('a');
    wasm.push(
      [],
      [{ kind: 'Outline', value: outline({ key_relay_holding: true }) }],
    );
    wasm.answerNext(
      [],
      [{ kind: 'Outline', value: outline({ key_relay_holding: false }) }],
    );
    wasm.replay = () => ({ kind: 'Type', value: 'b' });

    wasm.ask(1, {
      kind: 'FocusEditor',
      item_id: 'a',
      replay_keys: [held('b')],
    });
    vi.runAllTimers();

    expect(el.value).toBe('b');
    expect(wasm.events.filter(e => e.kind === 'KeyHeld')).toEqual([]);
    expect(wasm.answers).toEqual([{ id: 1, output: { kind: 'Done' } }]);
  });

  // 再生したEnterでコアがまたキーを預かると、残りのキーはコアに預ける。新しい項目のキーだからである。
  test('the keys after a replayed Enter go to the core while it holds the keys again', () => {
    const { wasm } = shell();
    editorOf('a');
    wasm.push(
      [],
      [{ kind: 'Outline', value: outline({ key_relay_holding: true }) }],
    );

    wasm.ask(1, {
      kind: 'FocusEditor',
      item_id: 'a',
      replay_keys: [held('b')],
    });
    vi.runAllTimers();

    expect(wasm.replays).toEqual([]);
    expect(wasm.events).toEqual([{ kind: 'KeyHeld', value: held('b') }]);
  });

  // フォーカスを待つ間に押したキーは、コアが預けたキーのあとに再生する。
  test('a key pressed while the focus waits is played after the keys the core held', () => {
    const { wasm, asked } = shell();
    editorOf('a');

    wasm.ask(1, {
      kind: 'FocusEditor',
      item_id: 'a',
      replay_keys: [held('x')],
    });
    const kept = asked.keepForReplay(
      new KeyboardEvent('keydown', { key: 'y' }),
    );
    vi.runAllTimers();

    expect(kept).toBe(true);
    expect(wasm.replays.map(r => r.key.key)).toEqual(['x', 'y']);
  });

  // フォーカスを待っていなければ、キーは預からない。Ctrl、Command、IMEのキーも預からない。
  test('no key is kept without a wait, nor a Ctrl, a Command or an IME key', () => {
    const { wasm, asked } = shell();
    const keep = (init: KeyboardEventInit) =>
      asked.keepForReplay(new KeyboardEvent('keydown', { key: 'y', ...init }));

    const before = keep({});
    wasm.ask(1, { kind: 'FocusEditor', item_id: 'a', replay_keys: [] });
    const during = [
      keep({ ctrlKey: true }),
      keep({ metaKey: true }),
      keep({ isComposing: true }),
    ];
    vi.runAllTimers();
    const after = keep({});

    expect([before, ...during, after]).toEqual([
      false,
      false,
      false,
      false,
      false,
    ]);
  });

  // 項目へのフォーカスを待つ間は、コアが預けたキーがなくても、キーを預かる。並べ替えた行の編集欄は、フォーカスを失っているためである。
  test('a focus of an item keeps keys even without held keys', () => {
    const { wasm, asked } = shell();

    wasm.ask(1, { kind: 'FocusEditor', item_id: 'a', replay_keys: [] });

    expect(
      asked.keepForReplay(new KeyboardEvent('keydown', { key: 'y' })),
    ).toBe(true);
  });

  // 項目のないフォーカスでも、コアが預けたキーがあれば、キーを預かる。
  test('a focus without an item keeps keys while the core held some', () => {
    const { wasm, asked } = shell();

    wasm.ask(1, {
      kind: 'FocusEditor',
      item_id: null,
      replay_keys: [held('x')],
    });

    expect(
      asked.keepForReplay(new KeyboardEvent('keydown', { key: 'y' })),
    ).toBe(true);
  });

  // 項目のないフォーカスで、コアが預けたキーもなければ、キーは預からない。
  test('a focus without an item and without held keys keeps no key', () => {
    const { wasm, asked } = shell();

    wasm.ask(1, { kind: 'FocusEditor', item_id: null, replay_keys: [] });

    expect(
      asked.keepForReplay(new KeyboardEvent('keydown', { key: 'y' })),
    ).toBe(false);
  });
});

// 項目までのスクロール
describe('the scroll to an item', () => {
  // スクロールは知らせなので、アウトラインに渡すだけで、答えない。
  test('the outline scrolls, and no answer goes back', () => {
    const { wasm, asked } = shell();
    const scrolled: [string, boolean][] = [];
    asked.scroller = (itemId, smooth) => scrolled.push([itemId, smooth]);

    wasm.ask(1, { kind: 'ScrollToItem', item_id: 'a', smooth: true });

    expect(scrolled).toEqual([['a', true]]);
    expect(wasm.answers).toEqual([]);
  });
});

// 破壊的な確認
describe('the confirm of a deletion', () => {
  // 確認が来るとメッセージが出て、答えるとその番号にはいかいいえが返り、メッセージは消える。
  test('the message shows, and the answer goes back by its number', () => {
    const { wasm, asked } = shell();
    let told = 0;
    asked.subscribe(() => (told += 1));

    wasm.ask(5, { kind: 'ConfirmDestructive', value: 'Delete 2 items?' });
    const shown = asked.confirmMessage();
    asked.answerConfirm(false);

    expect(shown).toBe('Delete 2 items?');
    expect(asked.confirmMessage()).toBeNull();
    expect(wasm.answers).toEqual([
      { id: 5, output: { kind: 'Bool', value: false } },
    ]);
    expect(told).toBe(2);
  });

  // 待っている確認がなければ、答えは何もしない。
  test('an answer without a confirm does nothing', () => {
    const { wasm, asked } = shell();
    let told = 0;
    asked.subscribe(() => (told += 1));

    asked.answerConfirm(true);

    expect(wasm.answers).toEqual([]);
    expect(told).toBe(0);
  });

  // 読むのをやめた部品には、もう知らせない。
  test('a reader that stopped reading is not told', () => {
    const { wasm, asked } = shell();
    let told = 0;
    const stop = asked.subscribe(() => (told += 1));

    stop();
    wasm.ask(5, { kind: 'ConfirmDestructive', value: 'Delete?' });

    expect(told).toBe(0);
  });
});

// 答えの行き先
describe('where the answers go', () => {
  // コアを渡す前に答えると、エラーになる。
  test('an answer before the core is given is an error', () => {
    const asked = new AskedOps();
    asked.take({ id: 1, op: { kind: 'ConfirmDestructive', value: 'Delete?' } });

    expect(() => asked.answerConfirm(true)).toThrow(Error);
  });
});

// シェルがしない操作
describe('an operation the shell does not do', () => {
  // Webのシェルがしない操作は、エラーとして書き残す。
  test('is written down as an error', () => {
    const { wasm } = shell();
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});

    wasm.ask(1, { kind: 'SetTitle', value: 'notes' });

    expect(error).toHaveBeenCalledOnce();
    error.mockRestore();
  });
});
