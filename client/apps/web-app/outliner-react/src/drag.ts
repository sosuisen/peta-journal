import type { Core } from './core';
import { textareaOffsetAtPoint, utf16OfChars } from './dom';

/** The listeners of the drag that is on. A new drag ends the one before. */
let ending: (() => void) | null = null;

/**
 * Follows a drag of the mouse that starts on the text of an item. The core decides when the
 * drag leaves the text and selects items (ADR 080): the shell only tells it where the mouse is.
 *
 * `selectFrom` is the character the mouse went down on, when the drag starts on the drawn
 * item. The editor that replaces it never saw that press, so the browser does not select its
 * text: the drag selects from that character to the one under the mouse. A drag that starts
 * on the editor gives null, and leaves the selection of the text to the browser.
 */
export function startDrag(
  core: Core,
  itemId: string,
  text: Element,
  selectFrom: number | null,
): void {
  ending?.();
  const { top, bottom } = text.getBoundingClientRect();
  core.drag({ kind: 'Pressed', item_id: itemId, top, bottom });
  const moved = (event: MouseEvent) => {
    const under = document
      .elementFromPoint(event.clientX, event.clientY)
      ?.closest('app-outline-item .outline-item');
    const tookIt = core.drag({
      kind: 'Moved',
      y: event.clientY,
      item_under: under?.id ?? null,
    });
    if (!tookIt && selectFrom !== null) {
      selectText(itemId, selectFrom, event.clientX, event.clientY);
    }
  };
  const released = () => {
    end();
    core.drag({ kind: 'Released' });
  };
  const end = () => {
    document.removeEventListener('mousemove', moved);
    document.removeEventListener('mouseup', released);
    ending = null;
  };
  document.addEventListener('mousemove', moved);
  document.addEventListener('mouseup', released);
  ending = end;
}

/** Selects the text of the editor from `anchor` to the character under the mouse. */
function selectText(
  itemId: string,
  anchor: number,
  x: number,
  y: number,
): void {
  const editor = document.getElementById(`item-editor-${itemId}`);
  if (!(editor instanceof HTMLTextAreaElement)) {
    return;
  }
  const focus = textareaOffsetAtPoint(editor, x, y);
  const value = editor.value;
  const [start, end, direction] =
    focus < anchor
      ? [focus, anchor, 'backward' as const]
      : [anchor, focus, 'forward' as const];
  editor.setSelectionRange(
    utf16OfChars(value, start),
    utf16OfChars(value, end),
    direction,
  );
}
