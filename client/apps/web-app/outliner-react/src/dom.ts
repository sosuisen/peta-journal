/**
 * The pieces of the shell that read or write the page directly, ported from `dom.rs` of the
 * Leptos shell (ADR 080). The textarea counts UTF-16 units, and the core counts characters
 * (ADR 043). What measures the layout of text is in `layout.ts`.
 */
import { mirrorOver } from './layout';

/** The number of characters before a UTF-16 offset of `text`. */
export function charsOfUtf16(text: string, units: number): number {
  return [...text.slice(0, units)].length;
}

/** The UTF-16 offset of the character `chars` of `text`. */
export function utf16OfChars(text: string, chars: number): number {
  return [...text].slice(0, chars).join('').length;
}

/** Puts the cursor before the character `chars` of the textarea. */
export function setCursorChars(el: HTMLTextAreaElement, chars: number): void {
  const at = utf16OfChars(el.value, chars);
  el.setSelectionRange(at, at);
}

export function focusWithoutScroll(el: HTMLElement): void {
  el.focus({ preventScroll: true });
}

/**
 * True when a key belongs to the composition of an IME, as the Enter that confirms a Japanese
 * conversion. Safari sends that Enter after `compositionend`, with the old key code 229.
 */
export function isImeComposing(event: KeyboardEvent): boolean {
  return event.isComposing || event.keyCode === 229;
}

interface CaretPosition {
  readonly offsetNode: Node;
  readonly offset: number;
}

/**
 * The node and the offset in it under a point of the screen. Firefox and Chrome 128+ have
 * `caretPositionFromPoint`; Safari has only `caretRangeFromPoint`.
 */
export function caretAtPoint(x: number, y: number): CaretPosition | null {
  const doc = document as Document & {
    caretPositionFromPoint?: (x: number, y: number) => CaretPosition | null;
  };
  if (doc.caretPositionFromPoint !== undefined) {
    return doc.caretPositionFromPoint(x, y);
  }
  const range = document.caretRangeFromPoint(x, y);
  return range === null
    ? null
    : { offsetNode: range.startContainer, offset: range.startOffset };
}

/**
 * The letters of the text of `container` before a point. Null when the point is on no text
 * of it, as beside its letters.
 */
export function textBeforePoint(
  container: Element,
  x: number,
  y: number,
): string | null {
  const caret = caretAtPoint(x, y);
  if (
    caret === null ||
    caret.offsetNode.nodeType !== Node.TEXT_NODE ||
    !container.contains(caret.offsetNode)
  ) {
    return null;
  }
  // The walker shows only text nodes, and it reaches the caret: the container holds it
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT);
  let before = '';
  for (
    let node = walker.nextNode() as Text;
    node !== caret.offsetNode;
    node = walker.nextNode() as Text
  ) {
    before += node.data;
  }
  return before + (caret.offsetNode as Text).data.slice(0, caret.offset);
}

/**
 * The place in characters in the text of the textarea under a point. A point outside the
 * textarea counts as the nearest edge, so that a drag that strays sideways still selects.
 */
export function textareaOffsetAtPoint(
  el: HTMLTextAreaElement,
  x: number,
  y: number,
): number {
  const rect = el.getBoundingClientRect();
  const px = Math.min(Math.max(x, rect.left + 1), rect.right - 1);
  const py = Math.min(Math.max(y, rect.top + 1), rect.bottom - 1);
  const caret = caretAtPoint(px, py);
  if (caret !== null && caret.offsetNode === el) {
    return charsOfUtf16(el.value, caret.offset);
  }
  return mirrorOffsetAtPoint(el, rect, px, py) ?? 0;
}

/** Asks a copy of the text laid over the textarea: its own text is out of reach elsewhere. */
function mirrorOffsetAtPoint(
  el: HTMLTextAreaElement,
  rect: DOMRect,
  x: number,
  y: number,
): number | null {
  const mirror = mirrorOver(el, rect);
  const caret = caretAtPoint(x, y);
  mirror.remove();
  return caret !== null && caret.offsetNode === mirror.text
    ? charsOfUtf16(el.value, caret.offset)
    : null;
}

/** The first image file of a paste or a drop. */
export function imageFile(data: DataTransfer): File | null {
  for (const item of data.items) {
    if (item.type.startsWith('image/')) {
      const file = item.getAsFile();
      if (file !== null) {
        return file;
      }
    }
  }
  return null;
}

/** `HH:MM` of an ISO time in the time zone of the browser. */
export function localTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  const two = (n: number) => String(n).padStart(2, '0');
  return `${two(date.getHours())}:${two(date.getMinutes())}`;
}
