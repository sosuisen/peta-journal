import { expect } from '@playwright/test';
import { Server, editorOf, openEditor, test } from './support';

/** The date in the local time zone as YYYY-MM-DD, the way the server names journal pages. */
const isoDate = (d: Date) =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
const shiftDays = (iso: string, days: number) => {
  const d = new Date(iso + 'T00:00:00');
  d.setDate(d.getDate() + days);
  return isoDate(d);
};
const todayName = isoDate(new Date());

test.describe('journal', () => {
  let server: Server;

  test.beforeEach(async ({ request }) => {
    server = new Server(request);
  });

  test('the app opens on today\'s journal, which has one empty item', async ({ page }) => {
    await page.goto('/');

    await expect(page).toHaveURL(/\/today$/);
    await page.locator('app-outline-item').first().waitFor();
    const today = await server.journal(todayName);
    expect((await server.items(today.id)).map(i => i.body)).toEqual(['']);
  });

  test('moving the top item up sends it to the next day in one request, which carries the typed text, and follows it there', async ({ page }) => {
    await page.goto('/today');
    await page.locator('app-outline-item').first().waitFor();
    const today = await server.journal(todayName);
    const [item] = await server.items(today.id);
    await openEditor(page, item.id);
    const writes: unknown[] = [];
    page.on('request', r => {
      if (r.method() !== 'GET') writes.push([`${r.method()} ${new URL(r.url()).pathname}`, r.postDataJSON()]);
    });

    // No pause between the keys: the auto-save has not sent the text yet when the item moves
    await page.keyboard.type('tomorrow please');
    await page.keyboard.press('Alt+Shift+ArrowUp');

    const nextDay = shiftDays(todayName, 1);
    await expect(page).toHaveURL(new RegExp(`/pages/${nextDay}$`));
    const next = await server.journal(nextDay);
    expect((await server.itemsSettled(next.id, i => i.some(x => x.body === 'tomorrow please'))).map(i => i.body))
      .toEqual(['tomorrow please']);
    // The day it left keeps an empty item
    expect((await server.itemsSettled(today.id, i => i.length === 1))[0].body).toBe('');
    await expect(editorOf(page, item.id)).toBeFocused();
    // Longer than the auto-save waits: the typed text must not be saved a second time
    await page.waitForTimeout(600);
    // The journal that opens reads its day with a POST, which is not a write of the move
    expect(writes.filter(([name]) => name !== `POST /api/journals/${nextDay}`)).toEqual([[
      `POST /api/journals/${nextDay}/move`,
      { itemIds: [item.id], position: 'last', body: 'tomorrow please' },
    ]]);
  });

  test('slow scrolling across day boundaries keeps a steady pace', async ({ page }) => {
    test.setTimeout(120_000);
    for (let d = 1; d <= 6; d++) {
      const date = shiftDays(todayName, -d);
      const day = await server.journal(date);
      await server.fill(day.id, Array.from({ length: 8 }, (_, i) => `${date} item ${i}\n` + 'text '.repeat(10)));
    }
    await page.goto('/today');
    await page.locator('app-outline-item').first().waitFor();
    await page.waitForTimeout(500);
    const viewport = page.locator('cdk-virtual-scroll-viewport');
    await viewport.hover();

    const step = 6;
    const positions: number[] = [];
    for (let t = 0; t < 400; t++) {
      await page.mouse.wheel(0, step);
      // Why not a fixed wait: the browser scrolls with the next frame. A wait shorter than a
      // frame reads the position before the step is there, and the next read then sees two steps
      positions.push(Math.round(await viewport.evaluate(el =>
        new Promise<number>(done => requestAnimationFrame(() => requestAnimationFrame(() => done(el.scrollTop)))))));
    }

    // Every wheel step moves the viewport by that step until the end is reached: no jumps, no way back
    const moves = positions.map((p, i) => (i === 0 ? p : p - positions[i - 1]));
    expect(moves.filter(m => m < 0 || m > step)).toEqual([]);
    expect(positions[positions.length - 1]).toBeGreaterThan(1000);
  });

  test('the previous-day button opens the day before', async ({ page }) => {
    await page.goto('/today');
    await page.locator('app-outline-item').first().waitFor();

    await page.getByRole('button', { name: '<', exact: true }).last().click();

    await expect(page).toHaveURL(new RegExp(`/pages/${shiftDays(todayName, -1)}$`));
    await page.locator('app-outline-item').first().waitFor();
  });

  test('Ctrl+Enter on an item of a day makes it a task planned for that day, in one request', async ({ page }) => {
    // A day no other run has used: the server keeps its pages from run to run
    const day = shiftDays('2000-01-01', Math.floor(Math.random() * 3000));
    const journal = await server.journal(day);
    const [item] = await server.items(journal.id);
    await page.goto(`/pages/${day}`);
    await openEditor(page, item.id);
    const writes: string[] = [];
    page.on('request', r => {
      if (r.method() !== 'GET') writes.push(`${r.method()} ${new URL(r.url()).pathname}`);
    });

    await page.keyboard.press('Control+Enter');

    const [task] = await server.itemsSettled(journal.id, i => i[0].types.includes('task'));
    expect(task.properties.plan).toBe(day);
    await expect(page.locator(`[id="${item.id}"] .task-checkbox`)).toHaveCount(1);
    // The server plans the task in the request that makes it one: no second request writes the plan
    expect(writes).toEqual([`POST /api/items/${item.id}/cycle-task`]);
  });

  test('the query today in any case puts Open Today on top of the search list, and choosing it opens today\'s journal', async ({ page }) => {
    await page.goto(`/pages/${shiftDays(todayName, -1)}`);
    await page.locator('app-outline-item').first().waitFor();

    await page.locator('.search-input').fill('Today');
    await expect(page.locator('.search-dropdown > *').first()).toHaveText('Open Today');
    await page.keyboard.press('ArrowDown');
    await expect(page.locator('.search-today')).toHaveClass(/selected/);
    await page.keyboard.press('Enter');

    await expect(page).toHaveURL(/\/today$/);
    await expect(page.locator('.search-input')).toHaveValue('');
  });

  test('ArrowDown and Enter pressed before the answer to a new query open its first result, not one of the old query', async ({ page }) => {
    const stamp = Date.now();
    const older = await server.newDocPage(`older-${stamp}`);
    const newer = await server.newDocPage(`newer-${stamp}`);
    await page.goto('/today');
    await page.locator('app-outline-item').first().waitFor();
    await page.locator('.search-input').fill(older.title);
    await expect(page.locator('.search-result').first()).toContainText(older.title);

    // The answer to the new query is on its way while the keys are pressed
    let release = () => {};
    const held = new Promise<void>(resolve => { release = resolve; });
    let sawSearch = () => {};
    const searchOnItsWay = new Promise<void>(resolve => { sawSearch = resolve; });
    await page.route('**/api/search/combined**', async route => {
      sawSearch();
      await held;
      await route.continue();
    });
    await page.locator('.search-input').fill(newer.title);
    await searchOnItsWay;
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');
    release();

    await expect(page).toHaveURL(new RegExp(`/pages/${newer.title}$`));
  });
});
