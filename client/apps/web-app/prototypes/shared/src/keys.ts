// The keys and the text of the editor of an item, as the core reads them (`client_core::keys`,
// `client_core::typing`).

import type { Core } from './core';

type Key =
  | 'Enter'
  | 'Tab'
  | 'Up'
  | 'Down'
  | 'Left'
  | 'Right'
  | 'Backspace'
  | 'Delete'
  | 'Escape'
  | { readonly Letter: string };

const NAMED: Readonly<Record<string, Key>> = {
  Enter: 'Enter',
  Tab: 'Tab',
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  Backspace: 'Backspace',
  Delete: 'Delete',
  Escape: 'Escape',
};

function keyOf(event: KeyboardEvent): Key | undefined {
  const named = NAMED[event.key];
  if (named !== undefined) {
    return named;
  }
  if (event.key.length === 1 && /[a-z]/i.test(event.key)) {
    return { Letter: event.key.toLowerCase() };
  }
  return undefined;
}

/**
 * The editor as a key finds it. The places of a textarea count UTF-16 units, as the core
 * does. A line is a line of the Markdown here, not a line as it is drawn.
 */
function editorState(area: HTMLTextAreaElement) {
  const text = area.value;
  const start = area.selectionStart;
  const end = area.selectionEnd;
  return {
    text,
    selection_start: start,
    selection_end: end,
    on_first_line: !text.slice(0, start).includes('\n'),
    on_last_line: !text.slice(end).includes('\n'),
  };
}

/**
 * Hands a key of the editor `area` of `itemId` to the core. True when the core took it, and the
 * browser must not.
 */
export function pressInEditor(
  core: Core,
  itemId: string,
  area: HTMLTextAreaElement,
  event: KeyboardEvent,
): boolean {
  // A key that ends the composition of the IME belongs to the IME
  if (event.isComposing || event.keyCode === 229) {
    return false;
  }
  const key = keyOf(event);
  if (key === undefined) {
    return false;
  }
  const chord = {
    key,
    shift: event.shiftKey,
    ctrl: event.ctrlKey,
    alt: event.altKey,
    cmd: event.metaKey,
  };
  const taken = core.press({ InEditor: { item_id: itemId, chord, editor: editorState(area) } });
  if (taken) {
    event.preventDefault();
  }
  return taken;
}

/**
 * Puts the cursor `chars` characters into the editor, as the core counts them in a
 * `FocusRequest`, and gives the editor the focus. The textarea counts UTF-16 units.
 */
export function placeCursor(area: HTMLTextAreaElement, chars: number | null): void {
  if (chars !== null) {
    const units = [...area.value].slice(0, chars).join('').length;
    area.setSelectionRange(units, units);
  }
  area.focus({ preventScroll: true });
}

/** Tells the core the text of the editor of `itemId` after a change. */
export function typedInEditor(core: Core, itemId: string, area: HTMLTextAreaElement): void {
  core.typed({
    Typed: { item_id: itemId, body: area.value, cursor: area.selectionStart },
  });
}
