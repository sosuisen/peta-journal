import { CDPSession, Page } from '@playwright/test';
import { Server, editorOf, openEditor, openPage, test } from './support';

/**
 * How long the browser works for one key, on pages of every length (#859). It prints times
 * and checks none of them, so it runs only when asked:
 *
 *   PETA_TIMING=1 npx playwright test timing
 *
 * The time is the main thread's busy time from Chrome (`Performance.getMetrics`) while one key
 * is handled: the core in wasm, the React shell, the DOM, style and layout. A key that types
 * is measured before the auto-save of the core (300ms) can start.
 */
const lengths = [1, 10, 100, 1000];
const keys = 30;
/** Shorter than the auto-save of the core, so a typed key is measured without the save. */
const typedWaitMs = 150;
/** Long enough for the answer of the server to a change of the levels. */
const movedWaitMs = 400;

type Metrics = Record<string, number>;

async function metrics(cdp: CDPSession): Promise<Metrics> {
  const { metrics } = await cdp.send('Performance.getMetrics');
  return Object.fromEntries(metrics.map(m => [m.name, m.value]));
}

/** The busy time of the main thread, in ms, while `act` runs and `waitMs` passes. */
async function busy(page: Page, cdp: CDPSession, act: () => Promise<void>, waitMs: number) {
  const before = await metrics(cdp);
  await act();
  await page.waitForTimeout(waitMs);
  const after = await metrics(cdp);
  const ms = (name: string) => (after[name] - before[name]) * 1000;
  return { task: ms('TaskDuration'), script: ms('ScriptDuration'), layout: ms('LayoutDuration') + ms('RecalcStyleDuration') };
}

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

function report(what: string, length: number, runs: Array<{ task: number; script: number; layout: number }>) {
  const m = (key: 'task' | 'script' | 'layout') => median(runs.map(r => r[key])).toFixed(2);
  console.log(`${what.padEnd(7)} ${String(length).padStart(5)} items: busy ${m('task')}ms (script ${m('script')}ms, style and layout ${m('layout')}ms)`);
}

test.describe('timing', () => {
  test.skip(!process.env.PETA_TIMING, 'prints times; run with PETA_TIMING=1');
  test.setTimeout(600_000);

  // 文字を打つときと、行が変わるときに、ブラウザが働く時間を、項目の数ごとに表示する。
  test('a key and a change of the rows on pages of every length', async ({ page, request }) => {
    const server = new Server(request);
    const cdp = await page.context().newCDPSession(page);
    await cdp.send('Performance.enable');
    for (const length of lengths) {
      const { id, title } = await server.newDocPage(`timing-${length}`);
      const lines = Array.from({ length }, (_, n) => '  '.repeat(n % 3) + `item ${n}`);
      const items = await server.fill(id, lines);
      await openPage(page, title);

      await openEditor(page, items[0].id);
      const typed = [];
      for (let n = 0; n < keys; n++) {
        typed.push(await busy(page, cdp, () => page.keyboard.press('x'), typedWaitMs));
      }
      report('type', length, typed);

      if (length < 2) continue;
      // The second item goes out from under the first and back: every key changes a row
      await page.keyboard.press('Escape');
      await openEditor(page, items[1].id);
      const moved = [];
      for (let n = 0; n < keys; n++) {
        const key = n % 2 === 0 ? 'Shift+Tab' : 'Tab';
        moved.push(await busy(page, cdp, () => page.keyboard.press(key), movedWaitMs));
      }
      await editorOf(page, items[1].id).waitFor();
      report('indent', length, moved);
    }
  });
});
