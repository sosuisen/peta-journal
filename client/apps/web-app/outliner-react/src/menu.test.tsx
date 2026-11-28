import { useState } from 'react';
import { flushSync } from 'react-dom';
import { createRoot, Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { useClosesOnOutside } from './menu';

function Box({ close }: { readonly close: () => void }) {
  useClosesOnOutside('.box', close);
  return <div className="box" />;
}

let host: HTMLDivElement;
let root: Root;
beforeEach(() => {
  host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host);
});
afterEach(() => {
  flushSync(() => root.unmount());
  host.remove();
});

const press = (target: EventTarget, type: string) =>
  target.dispatchEvent(new MouseEvent(type, { bubbles: true }));

// The events are sent without act. A browser runs the microtasks after each listener of a
// press of the user, so React draws the box and runs its effects while the event that opened
// it is still on its way to the document. A test dispatches from a script, where the
// microtasks wait for the whole dispatch: flushSync draws at the same point as the browser.

// メニューの外にある別のボタンのクリックで開いた箱は、そのクリックでは閉じない（メニューの項目が開く箱）。
test('a box opened by a click of a button outside it is not closed by that click', () => {
  const close = vi.fn();
  function Opener() {
    const [open, setOpen] = useState(false);
    return (
      <>
        <button onClick={() => flushSync(() => setOpen(true))}>open</button>
        {open && <Box close={close} />}
      </>
    );
  }
  flushSync(() => root.render(<Opener />));
  const button = host.querySelector('button') as HTMLButtonElement;
  press(button, 'pointerdown');
  press(button, 'click');
  expect(host.querySelector('.box')).not.toBeNull();
  expect(close).not.toHaveBeenCalled();
});

// 押した瞬間（pointerdown）に開いた箱は、同じ押下のクリックでは閉じない。外を押すと閉じる（PixiJSのボタンのように、押した瞬間に動くもの）。
test('a box opened at the press of a pointer stays after the click of that press, and a press outside closes it', () => {
  const close = vi.fn();
  const canvas = document.createElement('canvas');
  document.body.append(canvas);
  canvas.addEventListener('pointerdown', () =>
    flushSync(() => root.render(<Box close={close} />)),
  );
  press(canvas, 'pointerdown');
  press(canvas, 'pointerup');
  press(canvas, 'click');
  expect(close).not.toHaveBeenCalled();
  press(canvas, 'pointerdown');
  expect(close).toHaveBeenCalledOnce();
  canvas.remove();
});

// 箱の中を押しても閉じない。Escape で閉じる。
test('a press inside the box keeps it, and Escape closes it', () => {
  const close = vi.fn();
  flushSync(() => root.render(<Box close={close} />));
  const box = host.querySelector('.box') as HTMLElement;
  press(box, 'pointerdown');
  press(box, 'click');
  expect(close).not.toHaveBeenCalled();
  document.dispatchEvent(
    new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }),
  );
  expect(close).toHaveBeenCalledOnce();
});
