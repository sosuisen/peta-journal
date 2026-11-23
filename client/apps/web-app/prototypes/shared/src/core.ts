// The core in wasm, as a prototype shell of #971 sees it. Both prototypes use this file, so
// that they differ only in how they draw.

import init, { Shell } from '../../pkg/web_prototype_bridge.js';
import type { Changes, ListChange, OutlinePanel, SearchView, UiOp } from './types';

/** What a prototype does with the changes and the requests of the core. */
export interface View {
  /** Applies the changes to the rows, in order. */
  rows(changes: readonly ListChange[]): void;
  outline(panel: OutlinePanel): void;
  search(view: SearchView): void;
  /** Puts the cursor into the editor of the item, once it is on the screen. */
  focus(itemId: string | null): void;
  scrollTo(itemId: string): void;
}

/** How the changes cross from wasm: as JSON text, or as JS values. */
export type Carry = 'json' | 'value';

export function carryOfUrl(): Carry {
  return new URLSearchParams(location.search).get('carry') === 'value' ? 'value' : 'json';
}

/** The core, and the way the changes reach the view. */
export class Core {
  private readonly shell: Shell;
  private view: View | undefined;
  /** Inside a call of the shell: the changes are taken once it returns. */
  private depth = 0;
  private renderAsked = false;

  constructor(readonly carry: Carry) {
    this.shell = new Shell(
      () => this.onRender(),
      (id: number, op: string) => this.onUi(id, JSON.parse(op) as UiOp),
      (url: string, replace: boolean) => this.onNavigate(url, replace),
    );
  }

  /** Connects the view, and opens the page of the URL of the browser. */
  start(view: View): void {
    this.view = view;
    addEventListener('popstate', () => this.call(() => this.shell.route(location.pathname)));
    this.call(() => {
      this.shell.start();
      if (!this.shell.route(location.pathname)) {
        this.shell.route('/today');
      }
    });
  }

  send(event: unknown): void {
    this.call(() => this.shell.update(JSON.stringify(event)));
  }

  /** False when the key means nothing to the outline. */
  press(key: unknown): boolean {
    return this.call(() => this.shell.press(JSON.stringify(key)));
  }

  typed(typing: unknown): void {
    this.call(() => this.shell.typed(JSON.stringify(typing)));
  }

  resolveUi(id: number, output: unknown): void {
    this.call(() => this.shell.resolveUi(id, JSON.stringify(output)));
  }

  /** Times a reset of every row on the screen, both ways, without changing the view. */
  timeReset(rounds: number): { json: number; value: number; rows: number } {
    const json: number[] = [];
    const value: number[] = [];
    let rows = 0;
    for (let round = 0; round < rounds; round++) {
      const t0 = performance.now();
      const parsed = JSON.parse(this.shell.resetJson()) as Changes;
      const t1 = performance.now();
      const values = this.shell.resetValue() as Changes;
      const t2 = performance.now();
      json.push(t1 - t0);
      value.push(t2 - t1);
      const reset = parsed.rows[0];
      rows = reset !== undefined && 'Reset' in reset ? reset.Reset.items.length : 0;
      if (values.rows.length !== parsed.rows.length) {
        throw new Error('the two ways gave different changes');
      }
    }
    return { json: median(json), value: median(value), rows };
  }

  /** Runs a call of the shell, and then gives the view what changed. */
  private call<T>(run: () => T): T {
    this.depth++;
    try {
      return run();
    } finally {
      this.depth--;
      if (this.depth === 0 && this.renderAsked) {
        this.draw();
      }
    }
  }

  private onRender(): void {
    this.renderAsked = true;
    // An answer of the server or a timer: no call of the shell is on its way
    if (this.depth === 0) {
      this.draw();
    }
  }

  private draw(): void {
    this.renderAsked = false;
    const view = this.view;
    if (view === undefined) {
      return;
    }
    const changes = this.takeChanges();
    if (changes.rows.length > 0) {
      view.rows(changes.rows);
    }
    for (const panel of changes.panels) {
      if ('Outline' in panel) {
        view.outline(panel.Outline as OutlinePanel);
      } else if ('Search' in panel) {
        view.search(panel.Search as SearchView);
      }
    }
  }

  private takeChanges(): Changes {
    if (this.carry === 'value') {
      return this.shell.changesValue() as Changes;
    }
    return JSON.parse(this.shell.changesJson()) as Changes;
  }

  private onUi(id: number, op: UiOp): void {
    const view = this.view;
    if ('FocusEditor' in op) {
      const focus = op.FocusEditor as { item_id: string | null };
      // After the changes of this call are drawn
      queueMicrotask(() => {
        view?.focus(focus.item_id);
        this.resolveUi(id, 'Done');
      });
    } else if ('ScrollToItem' in op) {
      const scroll = op.ScrollToItem as { item_id: string };
      queueMicrotask(() => view?.scrollTo(scroll.item_id));
    } else if ('Alert' in op) {
      console.info('alert:', op.Alert);
    } else if ('Confirm' in op || 'ConfirmDestructive' in op) {
      // The prototype says no to every question: it deletes nothing by accident
      queueMicrotask(() => this.resolveUi(id, { Bool: false }));
    } else if ('Prompt' in op) {
      queueMicrotask(() => this.resolveUi(id, { Text: null }));
    }
  }

  private onNavigate(url: string, replace: boolean): void {
    if (replace) {
      history.replaceState(null, '', url);
    } else {
      history.pushState(null, '', url);
    }
    queueMicrotask(() => this.call(() => this.shell.route(location.pathname)));
  }
}

function median(values: readonly number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)] ?? 0;
}

/** Loads the wasm, and makes the core. */
export async function startCore(): Promise<Core> {
  await init();
  return new Core(carryOfUrl());
}
