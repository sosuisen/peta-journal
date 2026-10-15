import { expect, test } from '@playwright/test';
import { Server, editorOf, openEditor } from './support';

const isoDate = (d: Date) => d.toISOString().slice(0, 10);
const shiftDays = (iso: string, days: number) => {
  const d = new Date(iso + 'T00:00:00Z');
  d.setUTCDate(d.getUTCDate() + days);
  return isoDate(d);
};

test.describe('journal', () => {
  let server: Server;

  test.beforeEach(async ({ request }) => {
    server = new Server(request);
  });

  test('the app opens on today\'s journal, which has one empty item', async ({ page }) => {
    await page.goto('/');

    await expect(page).toHaveURL(/\/today$/);
    await page.locator('app-outline-item').first().waitFor();
    const [today] = await server.recentJournals();
    expect(today.name).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    expect((await server.items(today.id)).map(i => i.body)).toEqual(['']);
  });

  test('moving the top item up sends it to the next day and follows it there', async ({ page }) => {
    await page.goto('/today');
    await page.locator('app-outline-item').first().waitFor();
    const [today] = await server.recentJournals();
    const [item] = await server.items(today.id);
    await openEditor(page, item.id);
    await page.keyboard.type('tomorrow please');
    await server.itemsSettled(today.id, i => i[0].body === 'tomorrow please');

    await page.keyboard.press('Alt+Shift+ArrowUp');

    const nextDay = shiftDays(today.name, 1);
    await expect(page).toHaveURL(new RegExp(`/pages/${nextDay}$`));
    const next = (await server.recentJournals()).find(p => p.name === nextDay)!;
    expect((await server.itemsSettled(next.id, i => i.some(x => x.body === 'tomorrow please'))).map(i => i.body))
      .toEqual(['tomorrow please']);
    // The day it left keeps an empty item
    expect((await server.itemsSettled(today.id, i => i.length === 1))[0].body).toBe('');
    await expect(editorOf(page, item.id)).toBeFocused();
  });

  test('the previous-day button opens the day before', async ({ page }) => {
    await page.goto('/today');
    await page.locator('app-outline-item').first().waitFor();
    const [today] = await server.recentJournals();

    await page.getByRole('button', { name: '<', exact: true }).last().click();

    await expect(page).toHaveURL(new RegExp(`/pages/${shiftDays(today.name, -1)}$`));
    await page.locator('app-outline-item').first().waitFor();
  });
});
