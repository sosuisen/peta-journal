import { useEffect } from 'react';

/**
 * Closes an open menu on a click outside `selector` and on Escape. The menu calls it while it
 * is drawn. The button that opens the menu must be inside `selector` or stop its click: React
 * adds these listeners before that click reaches the document, which would close the menu.
 */
export function useClosesOnOutside(selector: string, close: () => void): void {
  useEffect(() => {
    const clicked = (event: MouseEvent) => {
      if (
        !(event.target instanceof Element) ||
        event.target.closest(selector) === null
      ) {
        close();
      }
    };
    const pressed = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        close();
      }
    };
    document.addEventListener('click', clicked);
    document.addEventListener('keydown', pressed);
    return () => {
      document.removeEventListener('click', clicked);
      document.removeEventListener('keydown', pressed);
    };
  }, [selector, close]);
}
