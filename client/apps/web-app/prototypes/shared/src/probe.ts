// What the measurements of #971 call from the console of the browser, as `window.proto`.

import type { Core } from './core';
import type { Row } from './types';
import { clear, measureUntil, report } from './metrics';

export interface Probe {
  readonly framework: string;
  readonly carry: string;
  report: typeof report;
  clear: typeof clear;
  timeReset(rounds: number): ReturnType<Core['timeReset']>;
  /** Folds or unfolds the item, and measures until the number of rows has changed. */
  toggle(itemId: string): void;
  /** The ID of the first row whose Markdown starts with `start`. */
  idOf(start: string): string | undefined;
  rowCount(): number;
}

declare global {
  interface Window {
    proto?: Probe;
  }
}

export function installProbe(framework: string, core: Core, rows: () => readonly Row[]): void {
  const rowCount = () => rows().length;
  window.proto = {
    framework,
    carry: core.carry,
    report,
    clear,
    timeReset: rounds => core.timeReset(rounds),
    toggle: itemId => {
      const before = rowCount();
      measureUntil('expand', () => rowCount() !== before);
      core.send({ CollapseToggle: { item_id: itemId } });
    },
    idOf: start => rows().find(row => row.item.body.startsWith(start))?.item.id,
    rowCount,
  };
}
