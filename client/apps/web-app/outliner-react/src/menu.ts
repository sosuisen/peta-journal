import { useEffect } from 'react';

/**
 * Closes an open menu on a press outside `selector` and on Escape. The menu calls it while it
 * is drawn.
 *
 * A press, not a click: a box opened by a click of a button outside it, such as a menu item,
 * would be closed by that same click, as React adds these listeners before the click reaches
 * the document. And a box opened at a press of a pointer, as by a button of PixiJS, would
 * be closed by the click that ends that press. The press that opened the box is
 * still on its way while the listeners are added, so it is let through.
 */
export function useClosesOnOutside(selector: string, close: () => void): void {
  useEffect(() => {
    // The event that drew the menu, if it is still being sent
    const opening = window.event;
    const pressed = (event: PointerEvent) => {
      if (event === opening) {
        return;
      }
      if (
        !(event.target instanceof Element) ||
        event.target.closest(selector) === null
      ) {
        close();
      }
    };
    const typed = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        close();
      }
    };
    document.addEventListener('pointerdown', pressed);
    document.addEventListener('keydown', typed);
    return () => {
      document.removeEventListener('pointerdown', pressed);
      document.removeEventListener('keydown', typed);
    };
  }, [selector, close]);
}
