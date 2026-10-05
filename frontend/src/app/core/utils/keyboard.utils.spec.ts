import { describe, it, expect } from 'vitest';
import { isImeComposingKeydown } from './keyboard.utils';

function keydown(init: KeyboardEventInit & { keyCode?: number }): KeyboardEvent {
  const event = new KeyboardEvent('keydown', init);
  if (init.keyCode !== undefined) {
    Object.defineProperty(event, 'keyCode', { value: init.keyCode });
  }
  return event;
}

describe('isImeComposingKeydown', () => {
  it('returns false for a plain Enter', () => {
    expect(isImeComposingKeydown(keydown({ key: 'Enter', keyCode: 13 }))).toBe(false);
  });

  it('returns true when the browser flags the event as composing (Chromium)', () => {
    expect(isImeComposingKeydown(keydown({ key: 'Enter', isComposing: true }))).toBe(true);
  });

  it('returns true for keyCode 229 even when isComposing is false (Safari on macOS)', () => {
    expect(isImeComposingKeydown(keydown({ key: 'Enter', isComposing: false, keyCode: 229 }))).toBe(true);
  });
});
