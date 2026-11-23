/**
 * The pieces of the shell that read or write the page directly, ported from `dom.rs` of the
 * Leptos shell (ADR 080). The textarea counts UTF-16 units, and the core counts characters
 * (ADR 043).
 */

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

/** Grows the textarea to its content. */
export function autoResize(el: HTMLTextAreaElement): void {
  el.style.height = 'auto';
  el.style.height = `${el.scrollHeight}px`;
}

/**
 * True when a key belongs to the composition of an IME, as the Enter that confirms a Japanese
 * conversion. Safari sends that Enter after `compositionend`, with the old key code 229.
 */
export function isImeComposing(event: KeyboardEvent): boolean {
  return event.isComposing || event.keyCode === 229;
}

/**
 * Lays out a hidden copy of the text of the textarea, and measures where a place ends up.
 *
 * The whole text is copied, not only the part before the place: a word that did not fit on a
 * line would fit in a copy that stops in the middle of it. The place is measured at the
 * character after it, not at a marker after the part before it: at a wrap the browser draws
 * the caret before the first character of the next line.
 */
function measureMarker(
  el: HTMLTextAreaElement,
  units: number,
  markerText: string,
): { left: number; top: number } {
  const div = document.createElement('div');
  const style = getComputedStyle(el);
  Object.assign(div.style, {
    position: 'absolute',
    visibility: 'hidden',
    whiteSpace: 'pre-wrap',
    wordWrap: 'break-word',
    boxSizing: 'border-box',
    width: `${el.clientWidth}px`,
    font: style.font,
    letterSpacing: style.letterSpacing,
    lineHeight: style.lineHeight,
    padding: style.padding,
    wordBreak: style.wordBreak,
    overflowWrap: style.overflowWrap,
  });
  const value = el.value;
  const at = Math.min(units, value.length);
  const before = value.slice(0, at);
  const after = value.slice(at);
  // A newline is not drawn: the caret before it is at the end of its line, where a marker is
  const next = [...after][0];
  const [marked, rest] =
    next !== undefined && next !== '\n'
      ? [next, after.slice(next.length)]
      : [markerText, after];
  const marker = document.createElement('span');
  marker.textContent = marked;
  div.append(
    document.createTextNode(before),
    marker,
    document.createTextNode(rest),
  );
  document.body.append(div);
  const place = { left: marker.offsetLeft, top: marker.offsetTop };
  div.remove();
  return place;
}

function lineHeightOf(el: HTMLElement): number {
  const height = parseFloat(getComputedStyle(el).lineHeight);
  return Number.isNaN(height) ? 20 : height;
}

/** The caret at the UTF-16 place `units`, in the coordinates of the window. */
export function caretRect(
  el: HTMLTextAreaElement,
  units: number,
): { x: number; top: number; bottom: number } {
  const { left, top } = measureMarker(el, units, '​');
  const rect = el.getBoundingClientRect();
  const y = rect.top + top - el.scrollTop;
  return {
    x: rect.left + left - el.scrollLeft,
    top: y,
    bottom: y + lineHeightOf(el),
  };
}

function yAt(el: HTMLTextAreaElement, units: number): number {
  return measureMarker(el, units, '|').top;
}

export function isOnFirstLine(el: HTMLTextAreaElement): boolean {
  const start = el.selectionStart;
  if (el.value === '' || start === 0) {
    return true;
  }
  return Math.abs(yAt(el, start) - yAt(el, 0)) < 5;
}

export function isOnLastLine(el: HTMLTextAreaElement): boolean {
  const length = el.value.length;
  const start = el.selectionStart;
  if (el.value === '' || start === length) {
    return true;
  }
  return Math.abs(yAt(el, length) - yAt(el, start)) < 5;
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
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT);
  let before = '';
  for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
    const text = node.textContent ?? '';
    if (node === caret.offsetNode) {
      return before + text.slice(0, caret.offset);
    }
    before += text;
  }
  return null;
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
  const div = document.createElement('div');
  const style = getComputedStyle(el);
  Object.assign(div.style, {
    position: 'fixed',
    left: `${rect.left}px`,
    top: `${rect.top - el.scrollTop}px`,
    width: `${el.clientWidth}px`,
    // Not seen, but still hit by the point, unlike `visibility: hidden`
    opacity: '0',
    zIndex: '2147483647',
    whiteSpace: 'pre-wrap',
    wordWrap: 'break-word',
    font: style.font,
    letterSpacing: style.letterSpacing,
    lineHeight: style.lineHeight,
    padding: style.padding,
    boxSizing: style.boxSizing,
  });
  const text = document.createTextNode(el.value);
  div.append(text);
  document.body.append(div);
  const caret = caretAtPoint(x, y);
  div.remove();
  return caret !== null && caret.offsetNode === text
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
