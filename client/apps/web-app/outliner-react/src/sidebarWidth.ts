import { useState, type MouseEvent } from 'react';

/** The settings of the browser that the Leptos shell kept, under the same key. */
const CONFIG_KEY = 'peta-journal-config';
const DEFAULT_WIDTH = 260;
const MIN_WIDTH = 200;
const MAX_WIDTH = 600;

function loadConfig(): Record<string, unknown> {
  try {
    return JSON.parse(localStorage.getItem(CONFIG_KEY) ?? '{}') as Record<
      string,
      unknown
    >;
  } catch {
    return {};
  }
}

function loadWidth(): number {
  const width = loadConfig().sidebarWidth;
  return typeof width === 'number' ? width : DEFAULT_WIDTH;
}

/** Keeps the width beside the other settings of the browser. */
function saveWidth(width: number): void {
  localStorage.setItem(
    CONFIG_KEY,
    JSON.stringify({ ...loadConfig(), sidebarWidth: width }),
  );
}

/**
 * The width of the sidebar, kept in the browser, and the press on its handle that starts a
 * resize. The width follows the mouse until the button comes up.
 */
export function useSidebarWidth(): [number, (event: MouseEvent) => void] {
  const [width, setWidth] = useState(loadWidth);
  const startResize = (event: MouseEvent) => {
    // The browser would select the text the mouse passes over
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = width;
    const moved = (move: globalThis.MouseEvent) => {
      const next = Math.min(
        Math.max(startWidth + move.clientX - startX, MIN_WIDTH),
        MAX_WIDTH,
      );
      setWidth(next);
      saveWidth(next);
    };
    const released = () => {
      document.removeEventListener('mousemove', moved);
      document.removeEventListener('mouseup', released);
    };
    document.addEventListener('mousemove', moved);
    document.addEventListener('mouseup', released);
  };
  return [width, startResize];
}
