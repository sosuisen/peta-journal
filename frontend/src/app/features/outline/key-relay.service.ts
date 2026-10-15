import { Injectable } from '@angular/core';

/** A key press kept for an editor that was not yet on screen when it was pressed. */
export interface DeferredKey {
  key: string;
  code: string;
  shiftKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
}

/** True for a key that types a character; other keys are shortcuts or navigation. */
export function isTypedCharacter(key: DeferredKey): boolean {
  return key.key.length === 1 && !key.ctrlKey && !key.altKey && !key.metaKey;
}

/**
 * Carries key presses from one editor to the next. Enter shows the next item at once, but
 * its editor gets the focus only after the next render; keys pressed in between would go
 * to the old editor (a Tab would indent the old item). While the relay holds, the old
 * editor hands its keys here instead, and the outline plays them in the editor it focuses.
 */
@Injectable({ providedIn: 'root' })
export class KeyRelayService {
  private held: DeferredKey[] | null = null;

  /** Starts holding keys until the next editor is focused. */
  holdUntilFocus(): void {
    this.held = [];
  }

  isHolding(): boolean {
    return this.held !== null;
  }

  /** Keeps a key press for the next focused editor; does nothing when not holding. */
  hold(event: KeyboardEvent): void {
    this.holdKey({
      key: event.key,
      code: event.code,
      shiftKey: event.shiftKey,
      ctrlKey: event.ctrlKey,
      altKey: event.altKey,
      metaKey: event.metaKey,
    });
  }

  /** Keeps a key that was already taken off an event, e.g. one left over from a replay. */
  holdKey(key: DeferredKey): void {
    this.held?.push(key);
  }

  /** Stops holding and returns the keys in the order they were pressed. */
  release(): DeferredKey[] {
    const keys = this.held ?? [];
    this.held = null;
    return keys;
  }
}
