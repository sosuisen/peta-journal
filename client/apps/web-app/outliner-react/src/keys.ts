import { Chord, EditorState, type Key, type OpenIn } from './border/peta-types';
import { isOnFirstLine, isOnLastLine } from './layout';

/** The keys of the browser by name, as the core knows them (ADR 052). */
const NAMED: Readonly<Record<string, Key>> = {
  Enter: { kind: 'Enter' },
  Tab: { kind: 'Tab' },
  ArrowUp: { kind: 'Up' },
  ArrowDown: { kind: 'Down' },
  ArrowLeft: { kind: 'Left' },
  ArrowRight: { kind: 'Right' },
  Backspace: { kind: 'Backspace' },
  Delete: { kind: 'Delete' },
  Escape: { kind: 'Escape' },
};

/**
 * The chord of a key, which the core looks up in its key map. Null for a key that means
 * nothing to the outline in any map. What a chord means is the core's (ADR 052).
 */
export function chordOf(event: KeyboardEvent): Chord | null {
  const named = NAMED[event.key];
  // Shift makes a letter upper case
  const key: Key | undefined =
    named ??
    ([...event.key].length === 1
      ? { kind: 'Letter', value: event.key.toLowerCase() }
      : undefined);
  if (key === undefined) {
    return null;
  }
  return new Chord(
    key,
    event.shiftKey,
    event.ctrlKey,
    event.altKey,
    event.metaKey,
  );
}

/**
 * The editor as a key finds it. The lines are measured only for Up and Down, which need
 * them: the measure lays out a copy of the text.
 */
export function editorState(el: HTMLTextAreaElement, key: string): EditorState {
  return new EditorState(
    el.value,
    BigInt(el.selectionStart),
    BigInt(el.selectionEnd),
    key === 'ArrowUp' && isOnFirstLine(el),
    key === 'ArrowDown' && isOnLastLine(el),
  );
}

/** Where a click opens a page: Shift the browser, Ctrl or Command a new view. Shift wins over both. */
export function openInOf(event: MouseEvent): OpenIn {
  if (event.shiftKey) {
    return { kind: 'Browser' };
  }
  return event.ctrlKey || event.metaKey
    ? { kind: 'NewView' }
    : { kind: 'Here' };
}
