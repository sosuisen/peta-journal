import { describe, expect, test } from 'vitest';
import { panelsAfter, listAfter } from './screen';

// 行の差分を当てる
describe('rows after their changes', () => {
  // 挿入、削除、移動、更新を順に当てると、行はコアの行と同じになる。位置は、その前の操作を当てたあとの行で数える。
  test('the changes in order make the rows of the core, each counted after the one before', () => {
    const rows = ['a', 'b', 'c'];

    const after = listAfter(rows, [
      { kind: 'Remove', at: 0 },
      { kind: 'Insert', at: 2, item: 'd' },
      { kind: 'Move', from: 0, to: 1 },
      { kind: 'Update', at: 0, item: 'C' },
    ]);

    expect(after).toEqual(['C', 'b', 'd']);
  });

  // 入れ替えは、前の行をすべて捨てる。
  test('a reset throws away every row before it', () => {
    expect(listAfter(['a', 'b'], [{ kind: 'Reset', items: ['x'] }])).toEqual([
      'x',
    ]);
  });

  // 操作の来ない行は、同じ値のまま残る。前の行の列は変わらない。
  test('a row with no change stays the same value, and the rows before stay as they were', () => {
    const a = { id: 'a' };
    const b = { id: 'b' };
    const rows = [a, b];

    const after = listAfter(rows, [
      { kind: 'Update', at: 1, item: { id: 'B' } },
    ]);

    expect(after[0]).toBe(a);
    expect(rows).toEqual([a, b]);
  });

  // 操作がなければ、行の列そのものが同じである。
  test('without a change the rows are the very same list', () => {
    const rows = ['a'];

    expect(listAfter(rows, [])).toBe(rows);
  });
});

// パネルの差分を当てる
describe('panels after their changes', () => {
  type Panel =
    | { kind: 'Title'; value: string }
    | { kind: 'Today'; value: string }
    | { kind: 'Sidebar'; value: { pages: string[] } };

  // 届いたパネルだけが入れ替わり、届かないパネルは前のままである。
  test('only the panels that came are replaced, and the others stay as they were', () => {
    const sidebar = { pages: ['a'] };
    const panels = { Title: 'old', Sidebar: sidebar };

    const after = panelsAfter<Panel>(panels, [
      { kind: 'Title', value: 'new' },
      { kind: 'Today', value: '2026-11-23' },
    ]);

    expect(after).toEqual({
      Title: 'new',
      Sidebar: sidebar,
      Today: '2026-11-23',
    });
    expect(after.Sidebar).toBe(sidebar);
    expect(panels.Title).toBe('old');
  });

  // パネルが1つも届かなければ、パネルは同じオブジェクトのままである。
  test('without a panel the panels are the very same object', () => {
    const panels = { Title: 'old' };

    expect(panelsAfter<Panel>(panels, [])).toBe(panels);
  });
});
