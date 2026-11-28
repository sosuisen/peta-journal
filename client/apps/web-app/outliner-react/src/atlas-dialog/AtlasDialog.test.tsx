import {
  createEvent,
  fireEvent,
  render,
  screen,
  cleanup,
} from '@testing-library/react';
import { afterEach, describe, expect, test, vi } from 'vitest';
import { AtlasDialog, type AtlasDialogProps, type LinkSuggestions } from '.';

afterEach(cleanup);

// The component alone, as a page outside the app draws it: no core, no app state, no provider
function dialog(props: Partial<AtlasDialogProps> = {}) {
  const ok = vi.fn<(side: string | null, text: string) => void>();
  const cancel = vi.fn<() => void>();
  const drawnWith = (changes: Partial<AtlasDialogProps>) => (
    <AtlasDialog
      name="Edit"
      mode="edit"
      text="x"
      previewHtml={text => `<p>${text}</p>`}
      onOk={ok}
      onCancel={cancel}
      {...props}
      {...changes}
    />
  );
  const view = render(drawnWith({}));
  const input = () => screen.getByRole<HTMLTextAreaElement>('textbox');
  /** The same dialog given other props, as a shell draws it again */
  const redraw = (changes: Partial<AtlasDialogProps>) =>
    view.rerender(drawnWith(changes));
  return { ok, cancel, input, redraw };
}

/** The names offered for a link, and what the dialog asked of them */
function links(shown: LinkSuggestions['shown']) {
  const asked: unknown[][] = [];
  const given: LinkSuggestions = {
    shown,
    typed: box => asked.push(['typed', box.value, box.selectionStart]),
    key: key => asked.push(['key', key]),
    picked: (text, cursor, title) => {
      asked.push(['picked', text, cursor, title]);
      return {
        text: `${text.slice(0, 2)}${title}]]`,
        cursor: 2 + title.length,
      };
    },
    close: () => asked.push(['close']),
    closed: (before, text, cursor) => {
      asked.push(['closed', before, text, cursor]);
      return text.endsWith('[[') ? `${text}]]` : undefined;
    },
  };
  return { given, asked };
}

// テキストのダイアログ（単体）
describe('the dialog of a text, alone', () => {
  // 名前のダイアログを、編集のタブで開く。本文は複数行の入力に出て、カーソルは終わりにある。
  test('it opens by its name on the edit tab, with the text in a box of several lines and the cursor at its end', () => {
    const { input } = dialog({ text: 'one\ntwo' });

    expect(screen.getByRole('dialog', { name: 'Edit' })).toBeTruthy();
    expect(input().tagName).toBe('TEXTAREA');
    expect(input().value).toBe('one\ntwo');
    expect(input().readOnly).toBe(false);
    expect(input().placeholder).toBe('Text');
    expect(document.activeElement).toBe(input());
    expect(input().selectionStart).toBe(7);
    expect(
      screen.getByRole('tab', { name: 'Edit' }).getAttribute('aria-selected'),
    ).toBe('true');
    expect(screen.queryByRole('radio')).toBeNull();
  });

  // OKとCtrl+Enterは書いたものを渡す。Enterは改行を入れるだけである。EscapeとCancelとダイアログの外の押下は、Cancelである。
  test('OK and Ctrl with Enter give what is written; Enter only breaks the line; Escape, Cancel and a press beside are Cancel', () => {
    const first = dialog();
    fireEvent.change(first.input(), { target: { value: 'a\nb' } });
    const enter = createEvent.keyDown(first.input(), { key: 'Enter' });
    fireEvent(first.input(), enter);
    fireEvent.keyDown(first.input(), { key: 'a', ctrlKey: true });
    const ctrlEnter = createEvent.keyDown(first.input(), {
      key: 'Enter',
      ctrlKey: true,
    });
    fireEvent(first.input(), ctrlEnter);
    fireEvent.change(first.input(), { target: { value: 'c' } });
    fireEvent.keyDown(first.input(), {
      key: 'Enter',
      metaKey: true,
      isComposing: true,
    });
    fireEvent.click(screen.getByRole('button', { name: 'OK' }));
    cleanup();

    const second = dialog();
    const escape = createEvent.keyDown(second.input(), { key: 'Escape' });
    fireEvent(second.input(), escape);
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    fireEvent.mouseDown(second.input());
    const inside = second.cancel.mock.calls.length;
    fireEvent.mouseDown(
      document.querySelector('.atlas-dialog-overlay') as Element,
    );

    expect(enter.defaultPrevented).toBe(false);
    expect(ctrlEnter.defaultPrevented).toBe(true);
    expect(escape.defaultPrevented).toBe(true);
    expect(first.ok.mock.calls).toEqual([
      [null, 'a\nb'],
      [null, 'c'],
    ]);
    expect(first.cancel).not.toHaveBeenCalled();
    expect(inside).toBe(2);
    expect(second.cancel).toHaveBeenCalledTimes(3);
    expect(second.ok).not.toHaveBeenCalled();
  });

  // 辺のあるもの（グループとコールアウト）は、今の辺から選び、OKが選んだ辺を渡す。
  test('a side starts at its own and OK gives the one picked', () => {
    const { ok } = dialog({ side: 'left' });
    const picked = (name: string) =>
      screen.getByRole<HTMLInputElement>('radio', { name }).checked;
    const before = ['Top', 'Bottom', 'Left', 'Right'].map(picked);

    fireEvent.click(screen.getByRole('radio', { name: 'Right' }));
    fireEvent.click(screen.getByRole('button', { name: 'OK' }));

    expect(before).toEqual([false, false, true, false]);
    expect(ok.mock.calls).toEqual([['right', 'x']]);
  });

  // Previewは、渡されたHTMLを描き、描いたあとの手直しを呼ぶ。中のリンクは開かない。編集に戻っても、書いたものはそのままである。
  test('the Preview draws the HTML given for what is written, lets it be finished, and opens no link', () => {
    const drawn = vi.fn<(content: HTMLElement) => void>();
    const { input, redraw } = dialog({
      previewHtml: text => `<p><a href="/x">${text}</a></p>`,
      drawn,
    });
    fireEvent.change(input(), { target: { value: 'a **b**' } });

    fireEvent.click(screen.getByRole('tab', { name: 'Preview' }));
    const content = document.querySelector(
      '.atlas-dialog-preview .item-content',
    ) as HTMLElement;
    const html = content.innerHTML;
    const click = createEvent.click(content.querySelector('a') as Element);
    fireEvent(content.querySelector('a') as Element, click);
    const noBox = screen.queryByRole('textbox') === null;
    const drawnOnce = drawn.mock.calls.length;
    // A shell that draws the HTML anew, as for another folder of the tags
    redraw({ previewHtml: text => `<p>${text}!</p>`, drawn });
    const again = content.innerHTML;
    fireEvent.click(screen.getByRole('tab', { name: 'Edit' }));

    expect(html).toBe('<p><a href="/x">a **b**</a></p>');
    expect(drawnOnce).toBe(1);
    expect(again).toBe('<p>a **b**!</p>');
    expect(drawn.mock.calls).toEqual([[content], [content]]);
    expect(click.defaultPrevented).toBe(true);
    expect(noBox).toBe(true);
    expect(input().value).toBe('a **b**');
  });

  // 見るだけのダイアログは、本文を変えられず、ボタンはCancelだけで、辺もない。Previewはある。
  test('a dialog to view only cannot change the text, has Cancel alone and no side, and has the Preview', () => {
    const { input, cancel, ok } = dialog({
      name: 'View Text',
      mode: 'view',
      text: 'read **me**',
      side: 'left',
    });
    fireEvent.keyDown(input(), { key: 'Enter', ctrlKey: true });
    const buttons = screen
      .getAllByRole('button')
      .map(button => button.textContent);
    const tabs = screen.getAllByRole('tab').map(tab => tab.textContent);

    fireEvent.click(screen.getByRole('tab', { name: 'Preview' }));
    const html = document.querySelector(
      '.atlas-dialog-preview .item-content',
    )?.innerHTML;
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(screen.getByRole('dialog', { name: 'View Text' })).toBeTruthy();
    expect(buttons).toEqual(['Cancel']);
    expect(tabs).toEqual(['Text', 'Preview']);
    expect(ok).not.toHaveBeenCalled();
    expect(screen.queryByRole('radio')).toBeNull();
    expect(html).toBe('<p>read **me**</p>');
    expect(cancel).toHaveBeenCalledTimes(1);
  });

  // 見るだけのダイアログの本文は、読むだけの入力にある
  test('the text of a dialog to view only is in a box that is read only', () => {
    const { input } = dialog({ mode: 'view', text: 'read' });

    expect(input().readOnly).toBe(true);
    expect(input().value).toBe('read');
    expect(input().hasAttribute('placeholder')).toBe(false);
  });
});

// `[[`のページ名の候補（単体）
describe('the page names offered for a link, alone', () => {
  // 入力が変わるたびに、箱を渡す
  test('each change of the text gives the box', () => {
    const { given, asked } = links(null);
    const { input } = dialog({ links: given });

    fireEvent.change(input(), { target: { value: '[[To]]' } });

    expect(asked).toEqual([['typed', '[[To]]', 6]]);
  });

  // 打った`[[`は、渡された閉じ方で閉じ、カーソルは括弧の間にある。消したときは閉じ方を聞かない。
  test('a typed opening is closed as it is given, with the cursor inside, and a delete asks nothing', () => {
    const { given, asked } = links(null);
    const { input } = dialog({ text: 'a', links: given });

    fireEvent.input(input(), {
      target: { value: 'a[[' },
      inputType: 'insertText',
    });
    const closed = [input().value, input().selectionStart];
    fireEvent.input(input(), {
      target: { value: 'a[[]' },
      inputType: 'deleteContentBackward',
    });

    expect(closed).toEqual(['a[[]]', 3]);
    expect(asked.filter(a => a[0] === 'closed')).toEqual([
      ['closed', 'a', 'a[[', 3],
    ]);
    expect(input().value).toBe('a[[]');
  });

  // 候補は、渡された位置に、選ばれたものに印を付けて出る。押した名前は、カーソルのリンクに入り、候補は閉じる。
  test('the names show where they are given, the selected one marked, and a click puts one in the link', () => {
    const { given, asked } = links({
      titles: ['Todo', 'Tools'],
      selected: 1,
      x: 10,
      y: 20,
    });
    const { input } = dialog({ text: '[[To]]', links: given });
    const list = document.querySelector(
      '.atlas-dialog-suggestions',
    ) as HTMLElement;
    const names = [...list.children].map(n => [n.textContent, n.className]);

    const beyond = vi.fn();
    document.addEventListener('mousedown', beyond);
    const press = createEvent.mouseDown(screen.getByText('Todo'));
    fireEvent(screen.getByText('Todo'), press);
    document.removeEventListener('mousedown', beyond);

    expect([list.style.left, list.style.top]).toEqual(['10px', '20px']);
    expect(names).toEqual([
      ['Todo', 'atlas-dialog-suggestion'],
      ['Tools', 'atlas-dialog-suggestion selected'],
    ]);
    expect(asked).toEqual([['picked', '[[To]]', 6, 'Todo'], ['close']]);
    expect(input().value).toBe('[[Todo]]');
    expect(input().selectionStart).toBe(6);
    expect(screen.queryByRole('dialog')).not.toBeNull();
    // The box keeps the focus, and the press goes no further than the name
    expect(press.defaultPrevented).toBe(true);
    expect(beyond).not.toHaveBeenCalled();
  });

  // 名前が1つもないときは、候補の欄を出さない
  test('no names show no list', () => {
    const { given } = links({ titles: [], selected: 0, x: 0, y: 0 });
    dialog({ links: given });

    expect(document.querySelector('.atlas-dialog-suggestions')).toBeNull();
  });

  // 候補がある間、上下の矢印とEscapeは候補のものである。Enterは選ばれた名前を入れ、OKにならない。
  test('while names show, the arrows and Escape are theirs, and Enter puts the selected one in', () => {
    const { given, asked } = links({
      titles: ['Todo', 'Tools'],
      selected: 1,
      x: 0,
      y: 0,
    });
    const { input, ok, cancel } = dialog({ text: '[[To]]', links: given });

    fireEvent.keyDown(input(), { key: 'ArrowDown' });
    fireEvent.keyDown(input(), { key: 'ArrowUp' });
    fireEvent.keyDown(input(), { key: 'Escape' });
    fireEvent.keyDown(input(), { key: 'Enter' });

    expect(asked).toEqual([
      ['key', 'ArrowDown'],
      ['key', 'ArrowUp'],
      ['key', 'Escape'],
      ['picked', '[[To]]', 6, 'Tools'],
      ['close'],
    ]);
    expect(cancel).not.toHaveBeenCalled();
    expect(ok).not.toHaveBeenCalled();
  });

  // ダイアログが閉じるときは、候補も閉じる
  test('the names close with the dialog, by the last links it was given', () => {
    const first = links(null);
    const last = links(null);
    const { redraw } = dialog({ links: first.given });
    redraw({ links: last.given });

    cleanup();

    expect(first.asked).toEqual([]);
    expect(last.asked).toEqual([['close']]);
  });
});

// モジュールは、Reactと自分の中のものだけを読む。アプリの外のページが、そのまま使える。
test('the module imports React and its own files alone', () => {
  // The text of each file of the module but the tests, as Vite reads it
  const sources = import.meta.glob<string>(
    ['./*.ts', './*.tsx', '!./*.test.*'],
    {
      query: '?raw',
      import: 'default',
      eager: true,
    },
  );
  const imports = Object.values(sources).flatMap(source =>
    [...source.matchAll(/from\s+'([^']+)'|import\s+'([^']+)'/g)].map(
      m => m[1] ?? m[2],
    ),
  );

  expect(Object.keys(sources).sort()).toEqual([
    './AtlasDialog.tsx',
    './index.ts',
  ]);
  expect(imports.filter(i => i !== 'react' && !i.startsWith('./'))).toEqual([]);
});
