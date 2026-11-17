import { expect } from '@playwright/test';
import { Server, StoredItem, editorOf, openEditor, openPage, outlineOf, shownBodies, test } from './support';

/**
 * The server must get the requests in the order the core made them. A browser sends requests
 * side by side, so a slow one is passed by the next. The native shells lost typed text that
 * way on 2026-11-11: the save of an item arrived after the merge that removed the item.
 */
test.describe('request order', () => {
  let server: Server;
  let pageId: string;
  let items: StoredItem[];
  const byBody = (body: string) => items.find(i => i.body === body)!;

  test.beforeEach(async ({ page, request }) => {
    server = new Server(request);
    const doc = await server.newDocPage('order');
    pageId = doc.id;
    items = await server.fill(pageId, ['a', 'b']);
    await openPage(page, doc.title);
  });

  test('a join right after typing waits for the slow save of the typed text, and keeps the text', async ({ page }) => {
    const a = byBody('a').id;
    const b = byBody('b').id;
    // The save of `b` is slow on its way to the server, as on a server that has just started
    const sent: string[] = [];
    let release = () => {};
    const held = new Promise<void>(resolve => { release = resolve; });
    let sawSave = () => {};
    const saveOnItsWay = new Promise<void>(resolve => { sawSave = resolve; });
    await page.route('**/api/**', async route => {
      const request = route.request();
      if (request.method() === 'GET') return route.continue();
      const call = `${request.method()} ${new URL(request.url()).pathname}`;
      sent.push(call);
      if (call === `PUT /api/items/${b}`) {
        sawSave();
        await held;
      }
      await route.continue();
    });

    await openEditor(page, b);
    await page.keyboard.press('End');
    await page.keyboard.type('X');
    await saveOnItsWay;
    expect(sent).toEqual([`PUT /api/items/${b}`]);

    // Delete at the end of `a` joins `b` to it
    await openEditor(page, a);
    await page.keyboard.press('End');
    await page.keyboard.press('Delete');
    // Time enough for a request that does not wait to reach the server. The join waits: sent
    // side by side, it reached the server first and joined the text saved before, without the X
    await page.waitForTimeout(500);
    expect(sent).toEqual([`PUT /api/items/${b}`]);
    expect(outlineOf(await server.items(pageId))).toEqual(['1:a', '1:b']);
    release();

    expect(outlineOf(await server.itemsSettled(pageId, i => i.length === 1))).toEqual(['1:abX']);
    expect(sent.at(-1)).toBe('POST /api/items/merge');
    await expect.poll(() => shownBodies(page)).toEqual(['abX']);
    await expect(editorOf(page, a)).toBeFocused();
  });
});
