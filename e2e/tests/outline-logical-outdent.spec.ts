import { expect } from '@playwright/test';
import { Server, StoredItem, editorOf, openEditor, openPage, outlineOf, shownBodies, test } from './support';

test.describe('logical outdent', () => {
  let server: Server;
  let pageId: string;
  let items: StoredItem[];
  const byBody = (body: string) => items.find(i => i.body === body)!;

  // * p
  //     * a
  //     * b
  //         * c
  //     * d
  test.beforeEach(async ({ page, request }) => {
    server = new Server(request);
    await server.setLogicalOutdent(true);
    const doc = await server.newDocPage('logical-outdent');
    pageId = doc.id;
    items = await server.fill(pageId, ['p', '  a', '  b', '    c', '  d']);
    await openPage(page, doc.title);
  });

  test.afterEach(async () => {
    await server.setLogicalOutdent(false);
  });

  test('Shift+Tab puts the item with its children below its siblings, one level up, in one request that carries the typed text', async ({ page }) => {
    await openEditor(page, byBody('b').id);
    await page.keyboard.press('End');
    const writes: unknown[] = [];
    page.on('request', r => {
      if (r.method() !== 'GET') writes.push([`${r.method()} ${new URL(r.url()).pathname}`, r.postDataJSON()]);
    });

    // No pause between the keys: the auto-save has not sent the text yet when the item moves
    await page.keyboard.type('X');
    await page.keyboard.press('Shift+Tab');

    expect(outlineOf(await server.itemsSettled(pageId, i => i[2].body === 'd')))
      .toEqual(['1:p', '2:a', '2:d', '1:bX', '2:c']);
    await expect.poll(() => shownBodies(page)).toEqual(['p', 'a', 'd', 'bX', 'c']);
    await expect(editorOf(page, byBody('b').id)).toBeFocused();
    // Longer than the auto-save waits: the typed text must not be saved a second time
    await page.waitForTimeout(600);
    expect(writes).toEqual([['POST /api/items/move', {
      itemIds: [byBody('b').id, byBody('c').id],
      targetPageId: pageId,
      position: `after:${byBody('d').id}`,
      levelDelta: -1,
      body: 'bX',
    }]]);
  });

  test('Ctrl+Z takes back a logical outdent, place and level', async ({ page }) => {
    await openEditor(page, byBody('b').id);
    await page.keyboard.press('Shift+Tab');
    await server.itemsSettled(pageId, i => i[2].body === 'd');
    // The server knows the outdent before the screen does, and only then is there one to take back
    await expect.poll(() => shownBodies(page)).toEqual(['p', 'a', 'd', 'b', 'c']);

    await page.keyboard.press('Control+z');

    expect(outlineOf(await server.itemsSettled(pageId, i => i[2].body === 'b' && i[2].level === 2)))
      .toEqual(['1:p', '2:a', '2:b', '3:c', '2:d']);
  });
});
