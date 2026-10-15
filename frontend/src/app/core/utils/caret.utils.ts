/** The text node and the offset inside it that lie under a point on the screen. */
export interface CaretPoint {
  node: Node;
  offset: number;
}

type DocumentWithCaretApis = Document & {
  caretPositionFromPoint?: (x: number, y: number) => { offsetNode: Node; offset: number } | null;
  caretRangeFromPoint?: (x: number, y: number) => Range | null;
};

/**
 * Finds the caret position under a point on the screen.
 * Firefox has caretPositionFromPoint; Chrome and Safari have caretRangeFromPoint.
 * @param x - The x coordinate relative to the viewport.
 * @param y - The y coordinate relative to the viewport.
 * @returns The node and offset, or null when the browser cannot tell.
 */
export function caretPointFromPoint(x: number, y: number): CaretPoint | null {
  const doc = document as DocumentWithCaretApis;
  if (doc.caretPositionFromPoint) {
    const pos = doc.caretPositionFromPoint(x, y);
    return pos ? { node: pos.offsetNode, offset: pos.offset } : null;
  }
  if (doc.caretRangeFromPoint) {
    const range = doc.caretRangeFromPoint(x, y);
    return range ? { node: range.startContainer, offset: range.startOffset } : null;
  }
  return null;
}
