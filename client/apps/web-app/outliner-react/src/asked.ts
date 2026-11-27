import { DeferredKey } from './border/peta-types';
import type { Asked, Core } from './core';
import { checkWikiLink } from './WikiSuggestions';
import { focusWithoutScroll, isImeComposing } from './dom';
import { autoResize } from './layout';

/**
 * The operations of the screen that only the shell can do (ADR 080): the focus of an editor,
 * the scroll to an item, and the confirm of a deletion. Wasm does the rest with `window`.
 */
export class AskedOps {
  /** Scrolls the outline to an item; the outline sets it while it is on screen. */
  scroller: ((itemId: string, smooth: boolean) => void) | null = null;
  /** The confirm that waits for an answer: its number and its message. */
  private confirm: { readonly id: number; readonly message: string } | null =
    null;
  private readonly listeners = new Set<() => void>();
  private core: Core | null = null;
  /**
   * The keys pressed while an editor waits for its focus. They are played into it after the
   * keys the core held, or they would overtake them. Null while no editor waits.
   */
  private lateKeys: DeferredKey[] | null = null;

  /** The core that the answers go to. Set once, right after the core is made. */
  answerTo(core: Core): void {
    this.core = core;
  }

  /** The core asks only after it is made, so the core is always given by then. */
  private get answering(): Core {
    if (this.core === null) {
      throw new Error('answerTo is not called before the core asks');
    }
    return this.core;
  }

  readonly take = (asked: Asked): void => {
    const { id, op } = asked;
    switch (op.kind) {
      case 'FocusEditor': {
        // Kept even when the core holds no key: a row put at another place has lost the
        // focus of its editor, and a key pressed before the focus is back would reach none
        if (op.item_id !== null || op.replay_keys.length > 0) {
          this.lateKeys ??= [];
        }
        // After the render: the editor may come in this very change of the screen
        setTimeout(() => {
          const selector =
            op.item_id === null
              ? 'app-item-editor textarea'
              : `#item-editor-${CSS.escape(op.item_id)}`;
          const editor = document.querySelector<HTMLTextAreaElement>(selector);
          if (editor !== null) {
            focusWithoutScroll(editor);
            autoResize(editor);
          }
          // Taken before the replay: a key it plays must not be kept again
          const keys = [...op.replay_keys, ...(this.lateKeys ?? [])];
          this.lateKeys = null;
          // Answered before the replay: the core holds keys until the answer (ADR 087), and
          // a key played while it holds would go back to it, again and again
          this.answering.resolve(id, { kind: 'Done' });
          if (editor !== null && op.item_id !== null) {
            this.replay(editor, op.item_id, keys);
          }
        }, 0);
        break;
      }
      case 'ScrollToItem':
        // A notice: the core waits for no answer
        this.scroller?.(op.item_id, op.smooth);
        break;
      case 'ConfirmDestructive':
        this.confirm = { id, message: op.value };
        this.changed();
        break;
      default:
        console.error('An operation the web shell does not do', op);
    }
  };

  /**
   * Keeps a key for the editor that waits for its focus, and says whether it did. Ctrl and
   * Command keys are not kept: they cannot be played again.
   */
  keepForReplay(event: KeyboardEvent): boolean {
    if (
      this.lateKeys === null ||
      event.ctrlKey ||
      event.metaKey ||
      isImeComposing(event)
    ) {
      return false;
    }
    this.lateKeys.push(deferredKey(event));
    return true;
  }

  /**
   * Plays the held keys into the editor. The core says what each key does there: a letter
   * goes into the text, a key that deletes or moves does that, and any other key is pressed.
   */
  private replay(
    editor: HTMLTextAreaElement,
    itemId: string,
    keys: readonly DeferredKey[],
  ): void {
    const core = this.answering;
    const changed = () => {
      core.typed({ kind: 'Changed', item_id: itemId, body: editor.value });
      autoResize(editor);
      checkWikiLink(core, itemId, editor);
    };
    for (const key of keys) {
      // A played Enter holds the keys again: those after it belong to the item it makes
      if (core.current().panels.Outline?.key_relay_holding === true) {
        core.send({ kind: 'KeyHeld', value: key });
        continue;
      }
      const replay = core.replayOf(
        key,
        editor.value,
        editor.selectionStart,
        editor.selectionEnd,
      );
      switch (replay.kind) {
        case 'Type':
          editor.setRangeText(
            replay.value,
            editor.selectionStart,
            editor.selectionEnd,
            'end',
          );
          changed();
          break;
        case 'Delete':
          editor.setRangeText(
            '',
            Number(replay.from),
            Number(replay.to),
            'end',
          );
          changed();
          break;
        case 'MoveTo':
          editor.setSelectionRange(Number(replay.value), Number(replay.value));
          break;
        case 'Press':
          editor.dispatchEvent(
            new KeyboardEvent('keydown', {
              key: key.key,
              code: key.code,
              shiftKey: key.shift_key,
              ctrlKey: key.ctrl_key,
              altKey: key.alt_key,
              metaKey: key.meta_key,
              bubbles: true,
              cancelable: true,
            }),
          );
          break;
      }
    }
  }

  /** The message of the confirm that waits for an answer. */
  readonly confirmMessage = (): string | null => this.confirm?.message ?? null;

  answerConfirm(yes: boolean): void {
    const confirm = this.confirm;
    if (confirm === null) {
      return;
    }
    this.confirm = null;
    this.changed();
    this.answering.resolve(confirm.id, { kind: 'Bool', value: yes });
  }

  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  private changed(): void {
    for (const listener of this.listeners) {
      listener();
    }
  }
}

/** A key as the core keeps it. */
export function deferredKey(event: KeyboardEvent): DeferredKey {
  return new DeferredKey(
    event.key,
    event.code,
    event.shiftKey,
    event.ctrlKey,
    event.altKey,
    event.metaKey,
  );
}
