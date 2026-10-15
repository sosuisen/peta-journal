import { afterEach, describe, expect, it } from 'vitest';
import { caretPointFromPoint } from './caret.utils';

type CaretApis = { caretPositionFromPoint?: unknown; caretRangeFromPoint?: unknown };
const doc = document as unknown as CaretApis;
const node = document.createTextNode('text');

describe('caretPointFromPoint', () => {
  afterEach(() => {
    delete doc.caretPositionFromPoint;
    delete doc.caretRangeFromPoint;
  });

  it('prefers the standard caretPositionFromPoint (Firefox)', () => {
    doc.caretPositionFromPoint = () => ({ offsetNode: node, offset: 2 });
    doc.caretRangeFromPoint = () => ({ startContainer: node, startOffset: 9 });
    expect(caretPointFromPoint(0, 0)).toEqual({ node, offset: 2 });
  });

  it('falls back to caretRangeFromPoint (Chrome, Safari)', () => {
    doc.caretRangeFromPoint = () => ({ startContainer: node, startOffset: 3 });
    expect(caretPointFromPoint(0, 0)).toEqual({ node, offset: 3 });
  });

  it('returns null when the browser has neither API or finds nothing', () => {
    expect(caretPointFromPoint(0, 0)).toBeNull();
    doc.caretRangeFromPoint = () => null;
    expect(caretPointFromPoint(0, 0)).toBeNull();
  });
});
