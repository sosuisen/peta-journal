/**
 * Returns true when a keydown event belongs to an IME composition session,
 * e.g. the Enter that confirms a Japanese conversion candidate.
 *
 * Chromium reports this via `isComposing`. Safari on macOS fires the
 * confirming Enter after `compositionend` with `isComposing === false`
 * but with the legacy `keyCode` 229 ("Process"), so both are checked.
 * @param event - The keydown event to inspect.
 */
export function isImeComposingKeydown(event: KeyboardEvent): boolean {
  return event.isComposing || event.keyCode === 229;
}
