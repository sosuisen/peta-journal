import { describe, expect, test } from 'vitest';
import { chordOf, editorState, openInOf } from './keys';

function key(init: KeyboardEventInit): KeyboardEvent {
  return new KeyboardEvent('keydown', init);
}

// キーの組み合わせ
describe('the chord of a key', () => {
  // 名前のあるキーは、コアの名前になる。修飾キーもそのまま渡る。
  test('a named key gets the name of the core, with its modifiers', () => {
    const chord = chordOf(
      key({ key: 'ArrowUp', shiftKey: true, altKey: true, metaKey: true }),
    );

    expect(chord).toEqual({
      key: { kind: 'Up' },
      shift: true,
      ctrl: false,
      alt: true,
      cmd: true,
    });
  });

  // 文字のキーは、Shiftで大文字になっても、小文字の文字になる。
  test('a letter is lower case, even when Shift made it upper case', () => {
    const chord = chordOf(key({ key: 'Z', shiftKey: true, ctrlKey: true }));

    expect(chord?.key).toEqual({ kind: 'Letter', value: 'z' });
    expect([chord?.shift, chord?.ctrl]).toEqual([true, true]);
  });

  // 名前がコアの知らないキー（F1など）は、組み合わせにならない。
  test('a key whose name the core does not know makes no chord', () => {
    expect(chordOf(key({ key: 'F1' }))).toBeNull();
  });
});

// キーを押したときの編集欄
describe('the editor as a key finds it', () => {
  function editor(text: string, start: number, end = start) {
    const el = document.createElement('textarea');
    el.value = text;
    el.setSelectionRange(start, end);
    return el;
  }

  // 本文と選択範囲を、UTF-16の位置で渡す。上下でないキーでは、行を測らない。
  test('the text and the selection go in UTF-16 units, and no line is measured for other keys', () => {
    const state = editorState(editor('a👍b', 1, 3), 'Enter');

    expect(state).toEqual({
      text: 'a👍b',
      selection_start: 1n,
      selection_end: 3n,
      on_first_line: false,
      on_last_line: false,
    });
  });

  // 上のキーは最初の行にいるかを、下のキーは最後の行にいるかを測る。先頭と末尾は、測らずにその行である。
  test('Up asks for the first line and Down for the last one', () => {
    const up = editorState(editor('one', 0), 'ArrowUp');
    const down = editorState(editor('one', 3), 'ArrowDown');

    expect([up.on_first_line, up.on_last_line]).toEqual([true, false]);
    expect([down.on_first_line, down.on_last_line]).toEqual([false, true]);
  });
});

// リンクを開く場所
describe('where a link opens', () => {
  // Shiftを押したクリックは新しいウィンドウ、CtrlかCommandは新しいタブ、何もなければその場で開く。両方なら、Shiftが勝つ。
  test('Shift opens a window, Ctrl or Command a tab, and nothing the place itself', () => {
    const click = (init: MouseEventInit) =>
      openInOf(new MouseEvent('click', init)).kind;

    expect([
      click({}),
      click({ ctrlKey: true }),
      click({ metaKey: true }),
      click({ shiftKey: true }),
      click({ shiftKey: true, ctrlKey: true }),
    ]).toEqual(['Here', 'NewView', 'NewView', 'Browser', 'Browser']);
  });
});
