/**
 * The wasm module of the tests: a `WebShell` that keeps what the shell hands it and answers
 * the changes a test gives it. The setup puts it in the place of `src/wasm/bridge`.
 */
import {
  Changes,
  DeferredKey,
  fromJsonDragMove,
  fromJsonEvent,
  fromJsonKeyPress,
  fromJsonTyping,
  fromJsonUiOutput,
  toJsonReplay,
  toJsonUiOp,
  type DragMove,
  type Event,
  type HeadingChange,
  type PageChange,
  type KeyPress,
  type RowChange,
  type Panel,
  type Replay,
  type Typing,
  type UiOp,
  type UiOutput,
} from '../border/peta-types';
import * as json from '../border/serde/json';

/** Nothing to load: the shell of the tests is plain TypeScript. */
export default async function init(): Promise<void> {}

/** The changes of the lists of the sidebar, each empty unless a test gives some. */
export interface Lists {
  readonly headings?: HeadingChange[];
  readonly index_pages?: PageChange[];
  readonly recent_pages?: PageChange[];
}

/** The changes the core answers: none, unless a test gives some. */
function written(
  rows: RowChange[] = [],
  panels: Panel[] = [],
  lists: Lists = {},
): string {
  const { headings = [], index_pages = [], recent_pages = [] } = lists;
  return json.stringify(
    Changes.toJson(
      new Changes(rows, headings, index_pages, recent_pages, panels),
    ),
  );
}

export class WebShell {
  /** The shell the last `Core` made. */
  static last: WebShell;

  readonly events: Event[] = [];
  readonly presses: KeyPress[] = [];
  readonly typings: Typing[] = [];
  readonly drags: DragMove[] = [];
  readonly pastes: { itemId: string; file: File; cursor: number }[] = [];
  readonly links: string[] = [];
  readonly answers: { id: number; output: UiOutput }[] = [];
  readonly replays: {
    key: DeferredKey;
    text: string;
    start: number;
    end: number;
  }[] = [];
  copies = 0;
  /** Whether the core takes a key; when not, the key is the editor's. */
  takesKeys = true;
  /** Whether the core takes a step of a drag; when not, the drag selects text. */
  takesDrags = true;
  /** What a held key does in the editor. */
  replay: (key: DeferredKey) => Replay = () => ({ kind: 'Press' });
  /** The text inside the wiki link at the cursor. */
  wikiQuery: (text: string, cursor: number) => string | undefined = () =>
    undefined;
  /** The changes the next call answers, once. */
  private reply: string | null = null;
  /** The changes that `changes()` gives after a render. */
  private pending = written();

  constructor(
    private readonly onRender: () => void,
    private readonly onAsk: (id: number, op: string) => void,
  ) {
    WebShell.last = this;
  }

  /** The next call answers these changes. */
  answerNext(rows: RowChange[], panels: Panel[] = []): void {
    this.reply = written(rows, panels);
  }

  /** The screen changes outside a call of the shell, as an answer of the server does. */
  push(rows: RowChange[], panels: Panel[] = [], lists: Lists = {}): void {
    this.pending = written(rows, panels, lists);
    this.onRender();
  }

  /** The core asks the shell for an operation of the screen. */
  ask(id: number, op: UiOp): void {
    this.onAsk(id, json.stringify(toJsonUiOp(op)));
  }

  start(): string {
    return this.answer();
  }

  update(event: string): string {
    this.events.push(fromJsonEvent(json.parse(event)));
    return this.answer();
  }

  press(key: string): string | undefined {
    this.presses.push(fromJsonKeyPress(json.parse(key)));
    return this.takesKeys ? this.answer() : undefined;
  }

  typed(typing: string): string {
    this.typings.push(fromJsonTyping(json.parse(typing)));
    return this.answer();
  }

  drag(step: string): string | undefined {
    this.drags.push(fromJsonDragMove(json.parse(step)));
    return this.takesDrags ? this.answer() : undefined;
  }

  pasteImage(itemId: string, file: File, cursor: number): string {
    this.pastes.push({ itemId, file, cursor });
    return this.answer();
  }

  go(url: string): string {
    this.links.push(url);
    return this.answer();
  }

  resolve(id: number, output: string): string {
    this.answers.push({ id, output: fromJsonUiOutput(json.parse(output)) });
    return this.answer();
  }

  replayOf(key: string, text: string, start: number, end: number): string {
    const held = DeferredKey.fromJson(json.parse(key));
    this.replays.push({ key: held, text, start, end });
    return json.stringify(toJsonReplay(this.replay(held)));
  }

  wikiQueryAt(text: string, cursor: number): string | undefined {
    return this.wikiQuery(text, cursor);
  }

  // The ids alone, so a test sees which cards and callouts were given
  atlasCards(cards: string, links: string): string {
    const ids = (text: string, id: string) =>
      (json.parse(text) as Record<string, string>[]).map(c => c[id]);
    return JSON.stringify({
      cards: ids(cards, 'page_id'),
      links: ids(links, 'item_id'),
    });
  }

  atlasCallouts(callouts: string): string {
    return JSON.stringify(
      (json.parse(callouts) as { key: string }[]).map(c => c.key),
    );
  }

  parentDirLabel(dirPath: string): string {
    return `above ${dirPath}`;
  }

  /** Five pixels a step, in a band of ten inside each edge. */
  edgeStep(y: number, top: number, bottom: number): number {
    if (y < top + 10) {
      return -5;
    }
    return y > bottom - 10 ? 5 : 0;
  }

  edgeStepMs(): number {
    return 16;
  }

  // The names are marked, so a test sees that they come from here
  shownNames(definitions: string): string[] {
    return (json.parse(definitions) as { key: string }[]).map(
      d => `shown ${d.key}`,
    );
  }

  userValueTypes(): string[] {
    return ['text', 'number'];
  }

  // The rule of page-urls, without its encoding: the tests use plain names
  tagUrl(tagsDir: string, tag: string): string {
    return `/pages/${tagsDir}${tag}`;
  }

  beginCopy(): void {
    this.copies += 1;
  }

  changes(): string {
    const changes = this.pending;
    this.pending = written();
    return changes;
  }

  private answer(): string {
    const changes = this.reply ?? written();
    this.reply = null;
    return changes;
  }
}
