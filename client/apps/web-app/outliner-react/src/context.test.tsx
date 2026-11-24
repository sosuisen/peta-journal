import { act, renderHook } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';
import { useAsked } from './askedContext';
import { useCore, usePanel, useRows } from './context';
import { draw, row, shell } from './test/screens';

// コアと画面の操作の文脈
describe('the contexts of the core and of the operations', () => {
  // コアの文脈の外でコアを使うと、エラーになる。
  test('the core outside its context is an error', () => {
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(() => renderHook(() => useCore())).toThrow(Error);
    error.mockRestore();
  });

  // 画面の操作の文脈の外で画面の操作を使うと、エラーになる。
  test('the operations of the screen outside their context are an error', () => {
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(() => renderHook(() => useAsked())).toThrow(Error);
    error.mockRestore();
  });
});

/** A component that counts how often it draws, and shows what `read` gives. */
function counted(read: () => string) {
  const drawn = { times: 0 };
  function Counted() {
    drawn.times += 1;
    return <span>{read()}</span>;
  }
  return { drawn, Counted };
}

// パネルと行の購読
describe('what a component reads of the screen', () => {
  // パネルを読むコンポーネントは、ほかのパネルが変わっても描き直されない。
  test('a component that reads a panel is not drawn again when another panel changes', () => {
    const ui = shell();
    const { drawn, Counted } = counted(
      () => usePanel('Connection')?.kind ?? 'none',
    );
    draw(ui, <Counted />);
    act(() =>
      ui.wasm.push([], [{ kind: 'Connection', value: { kind: 'Connected' } }]),
    );
    const before = drawn.times;

    act(() => ui.wasm.push([], [{ kind: 'Title', value: 'notes' }]));

    expect(drawn.times).toBe(before);
  });

  // パネルを読むコンポーネントは、行が変わっても描き直されない。
  test('a component that reads a panel is not drawn again when the rows change', () => {
    const ui = shell();
    const { drawn, Counted } = counted(
      () => usePanel('Connection')?.kind ?? 'none',
    );
    draw(ui, <Counted />);
    const before = drawn.times;

    act(() => ui.wasm.push([{ kind: 'Reset', items: [row('a', 'one')] }], []));

    expect(drawn.times).toBe(before);
  });

  // パネルを読むコンポーネントは、そのパネルが変わると、新しい値で描き直される。
  test('a component that reads a panel draws its new value when the panel changes', () => {
    const ui = shell();
    const { Counted } = counted(() => usePanel('Connection')?.kind ?? 'none');
    const { container } = draw(ui, <Counted />);

    act(() =>
      ui.wasm.push([], [{ kind: 'Connection', value: { kind: 'Connected' } }]),
    );

    expect(container.textContent).toBe('Connected');
  });

  // 行を読むコンポーネントは、パネルが変わっても描き直されない。
  test('a component that reads the rows is not drawn again when a panel changes', () => {
    const ui = shell();
    const { drawn, Counted } = counted(() => String(useRows().length));
    draw(ui, <Counted />);
    const before = drawn.times;

    act(() => ui.wasm.push([], [{ kind: 'Title', value: 'notes' }]));

    expect(drawn.times).toBe(before);
  });

  // 行を読むコンポーネントは、行が変わると、新しい行で描き直される。
  test('a component that reads the rows draws the new rows when they change', () => {
    const ui = shell();
    const { Counted } = counted(() => String(useRows().length));
    const { container } = draw(ui, <Counted />);

    act(() =>
      ui.wasm.push(
        [{ kind: 'Reset', items: [row('a', 'one'), row('b', 'two')] }],
        [],
      ),
    );

    expect(container.textContent).toBe('2');
  });
});
