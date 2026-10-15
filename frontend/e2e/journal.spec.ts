import { expect, test } from '@playwright/test';
import { Server, editorOf, openEditor } from './support';

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

  test('moving the top item up sends it to the next day and follows it there', async ({ page }) => {
    await page.goto('/today');
    await page.locator('app-outline-item').first().waitFor();
    const today = await server.journal(todayName);
    const [item] = await server.items(today.id);
    await openEditor(page, item.id);
    await page.keyboard.type('tomorrow please');
    await server.itemsSettled(today.id, i => i[0].body === 'tomorrow please');

    await page.keyboard.press('Alt+Shift+ArrowUp');

    const nextDay = shiftDays(todayName, 1);
    await expect(page).toHaveURL(new RegExp(`/pages/${nextDay}$`));
    const next = await server.journal(nextDay);
    expect((await server.itemsSettled(next.id, i => i.some(x => x.body === 'tomorrow please'))).map(i => i.body))
      .toEqual(['tomorrow please']);
    // The day it left keeps an empty item
    expect((await server.itemsSettled(today.id, i => i.length === 1))[0].body).toBe('');
    await expect(editorOf(page, item.id)).toBeFocused();
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
      await page.waitForTimeout(8);
      positions.push(Math.round(await viewport.evaluate(el => el.scrollTop)));
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
});
