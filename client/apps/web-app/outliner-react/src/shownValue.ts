import { useEffect, useLayoutEffect, useRef, type RefObject } from 'react';

/**
 * An input that shows the value of the core. The value is written when the core changes it.
 * The input is not controlled by React: it keeps its own value in between, so typing and an
 * IME composition are never written over.
 */
export function useShownValue(
  value: string,
): RefObject<HTMLInputElement | null> {
  const input = useRef<HTMLInputElement>(null);
  useLayoutEffect(() => {
    // The same value is not written again: a write may end an IME composition in progress
    const el = input.current as HTMLInputElement;
    if (el.value !== value) {
      el.value = value;
    }
  }, [value]);
  return input;
}

/**
 * Saves the value of the input on its change: when the focus leaves or on Enter. This is the
 * change of the DOM, not the one of React, which comes on every letter. Nothing is saved
 * without `save`.
 */
export function useSavedOnChange(
  input: RefObject<HTMLInputElement | null>,
  save: ((value: string) => void) | undefined,
): void {
  useEffect(() => {
    const el = input.current as HTMLInputElement;
    if (save === undefined) {
      return;
    }
    const changed = () => save(el.value);
    el.addEventListener('change', changed);
    return () => el.removeEventListener('change', changed);
  }, [input, save]);
}
