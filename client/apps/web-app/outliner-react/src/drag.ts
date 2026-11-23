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
  const tell = (x: number, y: number) => {
    const under = document
      .elementFromPoint(x, y)
      ?.closest('app-outline-item .outline-item');
    return core.drag({ kind: 'Moved', y, item_under: under?.id ?? null });
  };
  const viewport = text.closest('.outline-viewport');
  const edge = viewport === null ? null : edgeScroll(core, viewport, tell);
  // Once the core has taken the drag, it selects items to the end: a move over the item the
  // selection already reaches changes nothing in the core, but is not a drag of letters
  let selecting = false;
  const moved = (event: MouseEvent) => {
    selecting = tell(event.clientX, event.clientY) || selecting;
    if (selecting) {
      edge?.follow(event.clientX, event.clientY);
    } else if (selectFrom !== null) {
      selectText(itemId, selectFrom, event.clientX, event.clientY);
    }
  };
  const released = () => {
    end();
    core.drag({ kind: 'Released' });
  };
  const end = () => {
    edge?.stop();
    document.removeEventListener('mousemove', moved);
    document.removeEventListener('mouseup', released);
    ending = null;
  };
  document.addEventListener('mousemove', moved);
  document.addEventListener('mouseup', released);
  ending = end;
}

/** The scroll of the outline while the mouse of a drag that selects items is at its edge. */
interface EdgeScroll {
  /** The mouse moved: the scroll runs while it is in a band at the top or the bottom. */
  follow(x: number, y: number): void;
  stop(): void;
}

/**
 * Scrolls `viewport` while the mouse stays in its edge bands, as the core says how far. The
 * rows beyond the edge cannot be reached otherwise. After each step, `tell` gives the core the
 * item now under the mouse: the browser fires no move for rows that scroll under a still
 * mouse.
 */
function edgeScroll(
  core: Core,
  viewport: Element,
  tell: (x: number, y: number) => void,
): EdgeScroll {
  let step = 0;
  let pointer = { x: 0, y: 0 };
  let running = false;
  const frame = () => {
    if (step === 0) {
      running = false;
      return;
    }
    viewport.scrollTop = Math.max(0, viewport.scrollTop + step);
    tell(pointer.x, pointer.y);
    setTimeout(frame, core.edgeStepMs());
  };
  return {
    follow(x, y) {
      pointer = { x, y };
      const { top, bottom } = viewport.getBoundingClientRect();
      step = core.edgeStep(y, top, bottom);
      if (step !== 0 && !running) {
        running = true;
        setTimeout(frame, core.edgeStepMs());
      }
    },
    stop() {
      step = 0;
    },
  };
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
