import { expect } from '@playwright/test';
import { Server, openEditor, openPage, shownBodies, test } from './support';

// A change by another client reaches the page through the server's events (ADR 015).
test.describe('a change by another client', () => {
  // ほかのクライアントが消した項目は、編集中は画面に残り、編集を終えると消える
  test('an item another client deletes while this page is edited goes away when the edit ends', async ({ page, request }) => {
    const server = new Server(request);
    const p = await server.newDocPage('other-client-delete');
    const [a, b] = await server.fill(p.id, ['a', 'b']);
    await openPage(page, p.title);
    await openEditor(page, a.id);

    await server.deleteItem(b.id);
    await server.itemsSettled(p.id, i => i.length === 1);
    // The page waits for the end of the edit: typing must not be overwritten
    await page.waitForTimeout(500);
    expect(await shownBodies(page)).toEqual(['a', 'b']);

    await page.evaluate(() => (document.activeElement as HTMLElement).blur());

    await expect.poll(() => shownBodies(page)).toEqual(['a']);
  });
});
