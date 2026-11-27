import { WebShell } from './wasm/bridge';
import {
  AtlasCard,
  Callout,
  Changes,
  LinkCard,
  DeferredKey,
  PropertyDefinition,
  fromJsonReplay,
  fromJsonUiOp,
  toJsonDragMove,
  toJsonEvent,
  toJsonKeyPress,
  toJsonTyping,
  toJsonUiOutput,
  type DragMove,
  type Event,
  type Heading,
  type KeyPress,
  type Panel,
  type Replay,
  type Row,
  type Typing,
  type UiOp,
  type UiOutput,
} from './border/peta-types';
import * as json from './border/serde/json';
import { panelsAfter, listAfter, type Panels } from './screen';

/**
 * What the shell draws: the rows of the outline, the headings of the page and the panels, as
 * the core last told.
 */
export interface Screen {
  readonly rows: readonly Row[];
  readonly headings: readonly Heading[];
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
  private screen: Screen = { rows: [], headings: [], panels: {} };
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

  /** The URL of the page of the tag `tag`, in the folder of the tags `tagsDir`. */
  tagUrl(tagsDir: string, tag: string): string {
    return this.shell.tagUrl(tagsDir, tag);
  }

  /** The names the settings show for the keys of `definitions`, in their order. */
  shownNames(definitions: readonly PropertyDefinition[]): string[] {
    return this.shell.shownNames(
      json.stringify(definitions.map(d => PropertyDefinition.toJson(d))),
    );
  }

  /** The types a user may give a new key, in the order the settings offer them. */
  userValueTypes(): string[] {
    return this.shell.userValueTypes();
  }

  /** The placed cards of an atlas and its link cards, as `setCards` of `atlas.js` takes them. */
  atlasCards(cards: readonly AtlasCard[], links: readonly LinkCard[]): string {
    return this.shell.atlasCards(
      json.stringify(cards.map(c => AtlasCard.toJson(c))),
      json.stringify(links.map(l => LinkCard.toJson(l))),
    );
  }

  /** The callouts of an atlas, as `setCallouts` of `atlas.js` takes them. */
  atlasCallouts(callouts: readonly Callout[]): string {
    return this.shell.atlasCallouts(
      json.stringify(callouts.map(c => Callout.toJson(c))),
    );
  }

  /** The folder above the folder `dirPath`, as the up button of an atlas names it. */
  parentDirLabel(dirPath: string): string {
    return this.shell.parentDirLabel(dirPath);
  }

  /**
   * How far the outline scrolls in one step while a drag selects items, with the mouse at
   * `y` in a list seen from `top` to `bottom`. Below zero is up; zero outside the bands.
   */
  edgeStep(y: number, top: number, bottom: number): number {
    return this.shell.edgeStep(y, top, bottom);
  }

  /** How long one step of the scroll at an edge takes, in milliseconds. */
  edgeStepMs(): number {
    return this.shell.edgeStepMs();
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
    const { rows, headings, panels } = Changes.fromJson(json.parse(changes));
    if (rows.length === 0 && headings.length === 0 && panels.length === 0) {
      return;
    }
    this.screen = {
      rows: listAfter(this.screen.rows, rows),
      headings: listAfter(this.screen.headings, headings),
      panels: panelsAfter(this.screen.panels, panels),
    };
    for (const listener of this.listeners) {
      listener();
    }
  }
}
