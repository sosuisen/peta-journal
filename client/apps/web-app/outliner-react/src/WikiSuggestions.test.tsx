import { act, fireEvent, screen } from '@testing-library/react';
import { describe, expect, test } from 'vitest';
import { WikiView } from './border/peta-types';
import { draw, outline, shell } from './test/screens';
import WikiSuggestions, { checkWikiLink } from './WikiSuggestions';

// ウィキリンクの中の文字
describe('the text inside a wiki link', () => {
  function editor(text: string): HTMLTextAreaElement {
    const el = document.createElement('textarea');
    el.value = text;
    el.setSelectionRange(text.length, text.length);
    // jsdom lays nothing out: the editor says where it is
    el.getBoundingClientRect = () => new DOMRect(100, 200, 300, 40);
    document.body.append(el);
    return el;
  }

  // カーソルがリンクの中にあれば、その文字と、ポップアップを出すカーソルの下の位置をコアに渡す。
  test('inside a link the text and the place under the cursor go to the core', () => {
    const { core, wasm } = shell();
    wasm.wikiQuery = (text, cursor) => text.slice(2, cursor);

    checkWikiLink(core, 'a', editor('[[no'));

    expect(wasm.events).toEqual([
      {
        kind: 'WikiQuery',
        item_id: 'a',
        context: { query: 'no', x: 100, y: 220 },
      },
    ]);
    document.body.replaceChildren();
  });

  // リンクの外では、リンクがないことをコアに渡す。コアは候補を閉じる。
  test('outside a link the core is told there is none', () => {
    const { core, wasm } = shell();

    checkWikiLink(core, 'a', editor('no'));

    expect(wasm.events).toEqual([
      { kind: 'WikiQuery', item_id: 'a', context: null },
    ]);
    document.body.replaceChildren();
  });
});

// ページ名の候補
describe('the page names offered', () => {
  function offer(suggestions: string[], selected = 0) {
    const ui = shell();
    draw(ui, <WikiSuggestions />);
    const wiki = new WikiView('a', suggestions, BigInt(selected), 12, 34);
    act(() =>
      ui.wasm.push([], [{ kind: 'Outline', value: outline({ wiki }) }]),
    );
    return ui;
  }

  // 候補がなければ、何も出さない。アウトラインのパネルがまだなくても同じである。
  test('nothing shows without names, nor before the outline panel came', () => {
    const ui = shell();
    const { container } = draw(ui, <WikiSuggestions />);
    const before = container.innerHTML;
    act(() => ui.wasm.push([], [{ kind: 'Outline', value: outline() }]));

    expect([before, container.innerHTML]).toEqual(['', '']);
  });

  // 候補は、コアの言う位置に、選んだ候補に印をつけて出る。
  test('the names show where the core says, with a mark on the one picked', () => {
    offer(['notes', 'novel'], 1);

    const names = screen.getAllByText(/^no/);

    expect(names.map(n => n.className)).toEqual([
      'wiki-suggestion-item',
      'wiki-suggestion-item selected',
    ]);
    const box = names[0]?.parentElement;
    expect([box?.style.left, box?.style.top]).toEqual(['12px', '34px']);
  });

  // 候補を押すと、その名前をコアに渡す。編集欄はフォーカスを失わず、押したことは外に伝わらない。
  test('a press on a name hands it to the core, the editor keeps the focus, and the press goes no further', () => {
    const { wasm } = offer(['notes', 'novel']);

    let reached = false;
    const outside = () => (reached = true);
    document.addEventListener('mousedown', outside);

    const kept = !fireEvent.mouseDown(screen.getByText('novel'));
    document.removeEventListener('mousedown', outside);

    expect(kept).toBe(true);
    expect(reached).toBe(false);
    expect(wasm.events).toEqual([{ kind: 'WikiPick', title: 'novel' }]);
  });
});
