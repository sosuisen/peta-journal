import { WebShell } from './wasm/bridge';
import {
  Changes,
  DeferredKey,
  fromJsonReplay,
  fromJsonUiOp,
  toJsonDragMove,
  toJsonEvent,
  toJsonKeyPress,
  toJsonTyping,
  toJsonUiOutput,
  type DragMove,
  type Event,
  type KeyPress,
  type Panel,
  type Replay,
  type Row,
  type Typing,
  type UiOp,
  type UiOutput,
} from './border/peta-types';
import * as json from './border/serde/json';
import { panelsAfter, rowsAfter, type Panels } from './screen';

/** What the shell draws: the rows of the outline and the panels, as the core last told. */
export interface Screen {
  readonly rows: readonly Row[];
  readonly panels: Panels<Panel>;
}

/** An operation of the screen that only the shell can do, with the number to answer it by. */
export interface Asked {
  readonly id: number;
  readonly op: UiOp;
}

/**
 * The core in wasm, as the components use it (ADR 079). Every call hands the core what
 * happened, and puts the changes it answers on the screen. React reads the screen with
 * `useSyncExternalStore`.
 */
export class Core {
  private screen: Screen = { rows: [], panels: {} };
  private readonly listeners = new Set<() => void>();
  private readonly shell: WebShell;

  /** `onAsk` is given each operation the core asks the shell to do. */
  constructor(onAsk: (asked: Asked) => void) {
    this.shell = new WebShell(
      () => this.take(this.shell.changes()),
      (id: number, op: string) =>
        onAsk({ id, op: fromJsonUiOp(json.parse(op)) }),
    );
  }

  /** Starts the core, which opens the page of the address bar. */
  start(): void {
    this.take(this.shell.start());
  }

  send(event: Event): void {
    this.take(this.shell.update(json.stringify(toJsonEvent(event))));
  }

  /** Hands a key to the core. False when the key means nothing to it there. */
  press(key: KeyPress): boolean {
    const changes = this.shell.press(json.stringify(toJsonKeyPress(key)));
    if (changes === undefined) {
      return false;
    }
    this.take(changes);
    return true;
  }

  typed(typing: Typing): void {
    this.take(this.shell.typed(json.stringify(toJsonTyping(typing))));
  }

  /** Hands a step of a drag to the core. False while the drag stays in the text. */
  drag(step: DragMove): boolean {
    const changes = this.shell.drag(json.stringify(toJsonDragMove(step)));
    if (changes === undefined) {
      return false;
    }
    this.take(changes);
    return true;
  }

  pasteImage(itemId: string, file: File, cursor: number): void {
    this.take(this.shell.pasteImage(itemId, file, cursor));
  }

  /** Opens a link of the app, and keeps it in the history of the browser. */
  go(url: string): void {
    this.take(this.shell.go(url));
  }

  /** Answers the operation `id` that the core asked for. */
  resolve(id: number, output: UiOutput): void {
    this.take(this.shell.resolve(id, json.stringify(toJsonUiOutput(output))));
  }

  /**
   * What a held key does in an editor with `text` and the selection from `start` to `end`,
   * in UTF-16 units. The core decides it; the shell does it.
   */
  replayOf(key: DeferredKey, text: string, start: number, end: number): Replay {
    const answer = this.shell.replayOf(
      json.stringify(DeferredKey.toJson(key)),
      text,
      start,
      end,
    );
    return fromJsonReplay(json.parse(answer));
  }

  /**
   * The text typed inside the `[[wiki link]]` that the cursor at the UTF-16 place `cursor`
   * is in. Undefined outside a link.
   */
  wikiQueryAt(text: string, cursor: number): string | undefined {
    return this.shell.wikiQueryAt(text, cursor);
  }

  /** Begins the clipboard write of a copy, while the user's key still allows it. */
  beginCopy(): void {
    this.shell.beginCopy();
  }

  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  readonly current = (): Screen => this.screen;

  private take(changes: string): void {
    const { rows, panels } = Changes.fromJson(json.parse(changes));
    if (rows.length === 0 && panels.length === 0) {
      return;
    }
    this.screen = {
      rows: rowsAfter(this.screen.rows, rows),
      panels: panelsAfter(this.screen.panels, panels),
    };
    for (const listener of this.listeners) {
      listener();
    }
  }
}
