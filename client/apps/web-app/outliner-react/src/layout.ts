/**
 * The pieces of the shell that measure where the browser lays out text. jsdom lays nothing
 * out, so the tests cannot check them: the coverage and the mutation testing leave this file
 * out, as agreed on 2026-11-23. They are checked in a real browser.
 */
/** Grows the textarea to its content. */
export function autoResize(el: HTMLTextAreaElement): void {
  el.style.height = 'auto';
  el.style.height = `${el.scrollHeight}px`;
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

/**
 * Lays a copy of the text of the textarea over it, where a point hits the copy. The caller
 * removes it once it has asked the point.
 */
export function mirrorOver(
  el: HTMLTextAreaElement,
  rect: DOMRect,
): { readonly text: Text; readonly remove: () => void } {
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
  return { text, remove: () => div.remove() };
}
