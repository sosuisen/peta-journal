import { describe, expect, it } from 'vitest';
import { KeyRelayService, isTypedCharacter } from './key-relay.service';

describe('KeyRelayService', () => {
  const press = (key: string, init: KeyboardEventInit = {}) => new KeyboardEvent('keydown', { key, ...init });

  it('keeps the keys pressed while holding, in order, and hands them over once', () => {
    const relay = new KeyRelayService();
    relay.hold(press('x'));  // not holding yet: dropped

    relay.holdUntilFocus();
    relay.hold(press('Tab'));
    relay.hold(press('a', { shiftKey: true }));

    expect(relay.isHolding()).toBe(true);
    expect(relay.release().map(k => k.key)).toEqual(['Tab', 'a']);
    expect(relay.isHolding()).toBe(false);
    expect(relay.release()).toEqual([]);
  });

  it('isTypedCharacter tells typed text from shortcuts and navigation', () => {
    const key = (k: string, init: Partial<KeyboardEventInit> = {}) =>
      ({ key: k, code: '', shiftKey: false, ctrlKey: false, altKey: false, metaKey: false, ...init });

    expect(isTypedCharacter(key('a'))).toBe(true);
    expect(isTypedCharacter(key('A', { shiftKey: true }))).toBe(true);
    expect(isTypedCharacter(key('Tab'))).toBe(false);
    expect(isTypedCharacter(key('z', { ctrlKey: true }))).toBe(false);
  });
});
