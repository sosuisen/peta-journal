import { afterEach, describe, expect, test, vi } from 'vitest';
import {
  charsOfUtf16,
  focusWithoutScroll,
  imageFile,
  isImeComposing,
  localMinutes,
  localMinutesToUtc,
  localTime,
  nowLocalMinutes,
  setCursorChars,
  textBeforePoint,
  textareaOffsetAtPoint,
  utf16OfChars,
} from './dom';
import { autoResize, isOnFirstLine, isOnLastLine } from './layout';

/** jsdom lays nothing out: the tests say what the browser finds under a point. */
function caretAt(offsetNode: Node, offset: number): void {
  Object.defineProperty(document, 'caretPositionFromPoint', {
    configurable: true,
    value: () => ({ offsetNode, offset }),
  });
}

afterEach(() => {
  Reflect.deleteProperty(document, 'caretPositionFromPoint');
  document.body.replaceChildren();
});

function textarea(text: string): HTMLTextAreaElement {
  const el = document.createElement('textarea');
  el.value = text;
  document.body.append(el);
  return el;
}

// UTF-16の位置と文字の数
describe('UTF-16 units and characters', () => {
  // 絵文字は、UTF-16では2単位、文字では1つに数える。
  test('an emoji is two UTF-16 units and one character', () => {
    expect(charsOfUtf16('a👍b', 3)).toBe(2);
    expect(utf16OfChars('a👍b', 2)).toBe(3);
  });

  // カーソルは、文字の数で置ける。
  test('the cursor is put by a count of characters', () => {
    const el = textarea('a👍b');

    setCursorChars(el, 2);

    expect([el.selectionStart, el.selectionEnd]).toEqual([3, 3]);
  });
});

// 編集欄
describe('the editor', () => {
  // フォーカスは、ページを動かさずに編集欄に移る。行を置くのは、アウトラインの仕事である。
  test('the focus goes to the editor without a scroll of the page', () => {
    const el = textarea('');
    const focus = vi.spyOn(el, 'focus');

    focusWithoutScroll(el);

    expect(document.activeElement).toBe(el);
    expect(focus).toHaveBeenCalledWith({ preventScroll: true });
  });

  // 編集欄は、本文の高さに伸びる。
  test('the editor grows to the height of its text', () => {
    const el = textarea('one');
    // jsdom lays nothing out: the text says how high it is
    Object.defineProperty(el, 'scrollHeight', { value: 48 });
    el.style.height = '20px';

    autoResize(el);

    expect(el.style.height).toBe('48px');
  });

  // IMEの変換中のキーと、Safariが変換の確定のあとに送るキーコード229のキーは、IMEのキーである。
  test('a key of a composition and the key code 229 of Safari belong to the IME', () => {
    const keys = [
      new KeyboardEvent('keydown', { key: 'Enter', isComposing: true }),
      new KeyboardEvent('keydown', { key: 'Enter', keyCode: 229 }),
      new KeyboardEvent('keydown', { key: 'Enter' }),
    ];

    expect(keys.map(isImeComposing)).toEqual([true, true, false]);
  });

  // 空の本文と、先頭のカーソルは最初の行にある。空の本文と、末尾のカーソルは最後の行にある。
  test('an empty text and the ends are on the first and the last line', () => {
    const empty = textarea('');
    const text = textarea('one');
    text.setSelectionRange(0, 0);
    const atStart = isOnFirstLine(text);
    text.setSelectionRange(3, 3);
    const atEnd = isOnLastLine(text);

    expect([isOnFirstLine(empty), isOnLastLine(empty)]).toEqual([true, true]);
    expect([atStart, atEnd]).toEqual([true, true]);
  });
});

// 点の下の文字
describe('the text under a point', () => {
  // 描いた本文で、点より前の文字を、要素をまたいで答える。
  test('the letters before a point of a drawn body, across its elements', () => {
    const body = document.createElement('div');
    body.innerHTML = '<p>one <b>two</b></p>';
    document.body.append(body);
    const two = body.querySelector('b')?.firstChild as Node;
    caretAt(two, 1);

    expect(textBeforePoint(body, 0, 0)).toBe('one t');
  });

  // caretPositionFromPointのないSafariでは、caretRangeFromPointで同じ文字を答える。点が何の上にもなければ、何も答えない。
  test('Safari, which has only caretRangeFromPoint, gets the same letters', () => {
    const body = document.createElement('div');
    body.textContent = 'one';
    document.body.append(body);
    const range = document.createRange();
    range.setStart(body.firstChild as Node, 2);
    let under: Range | null = range;
    Object.defineProperty(document, 'caretRangeFromPoint', {
      configurable: true,
      value: () => under,
    });

    const letters = textBeforePoint(body, 0, 0);
    under = null;

    expect(letters).toBe('on');
    expect(textBeforePoint(body, 0, 0)).toBeNull();
    Reflect.deleteProperty(document, 'caretRangeFromPoint');
  });

  // 点が本文の文字の上にないときは、何も答えない。
  test('nothing when the point is on no letter of the body', () => {
    const body = document.createElement('div');
    body.textContent = 'one';
    const other = document.createTextNode('other');
    document.body.append(body, other);
    caretAt(other, 1);

    expect(textBeforePoint(body, 0, 0)).toBeNull();
  });

  // 点が文字でなく要素の上にあるときも、何も答えない。
  test('nothing when the point is on an element, not on a letter', () => {
    const body = document.createElement('div');
    body.innerHTML = '<p>one</p>';
    document.body.append(body);
    caretAt(body.firstChild as Node, 0);

    expect(textBeforePoint(body, 0, 0)).toBeNull();
  });

  // 編集欄の上の点は、その位置を文字の数で答える。
  test('a point on the editor is its place in characters', () => {
    const el = textarea('a👍b');
    caretAt(el, 3);

    expect(textareaOffsetAtPoint(el, 0, 0)).toBe(2);
  });

  // 編集欄の外の点は、編集欄のいちばん近い縁の点として数える。横にそれたドラッグも、文字を選び続けるためである。
  test('a point outside the editor counts as the nearest edge', () => {
    const el = textarea('one');
    el.getBoundingClientRect = () => new DOMRect(10, 20, 100, 50);
    const asked: number[][] = [];
    Object.defineProperty(document, 'caretPositionFromPoint', {
      configurable: true,
      value: (x: number, y: number) => {
        asked.push([x, y]);
        return { offsetNode: el, offset: 0 };
      },
    });

    textareaOffsetAtPoint(el, 0, 500);
    textareaOffsetAtPoint(el, 500, 0);

    expect(asked).toEqual([
      [11, 69],
      [109, 21],
    ]);
  });

  // 編集欄の中の文字を答えないブラウザでは、編集欄に重ねた写しの文字で答える。
  test('where the browser does not answer inside the editor, a copy laid over it answers', () => {
    const el = textarea('a👍b');
    Object.defineProperty(document, 'caretPositionFromPoint', {
      configurable: true,
      // Only the copy, the last child of the page, has a text to point at
      value: () => ({
        offsetNode: document.body.lastChild?.firstChild ?? document.body,
        offset: 3,
      }),
    });

    expect(textareaOffsetAtPoint(el, 0, 0)).toBe(2);
    expect(document.body.lastChild).toBe(el);
  });

  // 写しでも文字が見つからなければ、先頭の位置とする。
  test('the start when even the copy finds no letter', () => {
    const el = textarea('one');
    caretAt(document.body, 2);

    expect(textareaOffsetAtPoint(el, 0, 0)).toBe(0);
  });
});

// 貼り付けと落とした画像
describe('the image of a paste or a drop', () => {
  function data(...items: { type: string; file: File | null }[]) {
    return {
      items: items.map(({ type, file }) => ({ type, getAsFile: () => file })),
    } as unknown as DataTransfer;
  }

  // 最初の画像のファイルを答える。画像でないものと、ファイルのない画像は飛ばす。
  test('the first image file, past what is not an image or has no file', () => {
    const image = new File(['x'], 'b.png', { type: 'image/png' });

    const found = imageFile(
      data(
        { type: 'text/plain', file: new File(['t'], 'a.txt') },
        { type: 'image/png', file: null },
        { type: 'image/png', file: image },
      ),
    );

    expect(found).toBe(image);
  });

  // 画像がなければ、何も答えない。
  test('nothing without an image', () => {
    expect(imageFile(data({ type: 'text/plain', file: null }))).toBeNull();
  });
});

// 時刻
describe('the time of day', () => {
  // ISOの時刻を、ブラウザの時間帯のHH:MMで書く。読めない時刻は、そのまま返す。
  test('an ISO time is HH:MM in the time zone of the browser', () => {
    const time = new Date(2026, 10, 23, 9, 5).toISOString();

    expect(localTime(time)).toBe('09:05');
    expect(localTime('soon')).toBe('soon');
  });
});

// 日時の入力欄の値
describe('the value of a date-time input', () => {
  // UTCの日時は、ブラウザの時間帯の分までになる。読めない値は空である。
  test('a UTC time becomes the local time to the minute, and an unreadable one is empty', () => {
    const time = new Date(2026, 0, 2, 3, 4, 59).toISOString();

    expect(localMinutes(time)).toBe('2026-01-02T03:04');
    expect(localMinutes('soon')).toBe('');
  });

  // 入力欄の値は、サーバーが残す形のUTCの秒までになる。読めない値は空である。
  test('an input value becomes UTC to the second, as the server keeps it', () => {
    const utc = new Date(2026, 10, 23, 9, 5).toISOString().slice(0, 19) + 'Z';

    expect(localMinutesToUtc('2026-11-23T09:05')).toBe(utc);
    expect(localMinutesToUtc('')).toBe('');
  });

  // 今の時刻は、ブラウザの時間帯の分までである。
  test('now is the local time to the minute', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 10, 23, 21, 7, 30));

    expect(nowLocalMinutes()).toBe('2026-11-23T21:07');
    vi.useRealTimers();
  });
});
