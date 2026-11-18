import { expect } from '@playwright/test';
import { serverPort } from '../playwright.config';
import { Server, editorOf, openEditor, openPage, outlineOf, shownBodies, test } from './support';

// The item "copy" of the host page transcludes "top" of the source page, and shows its
// children below itself. A move of a child stays inside what the transclusion shows, and is
// written on the source page.
test.describe('moving inside a transclusion', () => {
  test('a move of a shown child is one request, which carries the text typed and not saved yet', async ({ page, request }) => {
    const server = new Server(request);
    const sourcePage = await server.newDocPage('move-source');
    const source = await server.fill(sourcePage.id, ['top', '  one', '  two']);
    const hostPage = await server.newDocPage('move-host');
    const host = await server.fill(hostPage.id, ['copy']);
    const transclude = `http://localhost:${serverPort}/api/items/${host[0].id}/transclude`;
    expect((await request.post(transclude, { data: { sourceItemId: source[0].id } })).ok()).toBe(true);
    await openPage(page, hostPage.title);
    await openEditor(page, source[2].id);
    await page.keyboard.press('End');
    const writes: unknown[] = [];
    page.on('request', r => {
      if (r.method() !== 'GET') writes.push([`${r.method()} ${new URL(r.url()).pathname}`, r.postDataJSON()]);
    });

    // No pause between the keys: the auto-save has not sent the text yet when the item moves
    await page.keyboard.type('X');
    await page.keyboard.press('Alt+Shift+ArrowUp');

    expect(outlineOf(await server.itemsSettled(sourcePage.id, i => i[1].body === 'twoX')))
      .toEqual(['1:top', '2:twoX', '2:one']);
    await expect.poll(() => shownBodies(page)).toEqual(['top', 'twoX', 'one']);
    await expect(editorOf(page, source[2].id)).toBeFocused();
    // Longer than the auto-save waits: the typed text must not be saved a second time
    await page.waitForTimeout(600);
    expect(writes).toEqual([['POST /api/items/move', {
      itemIds: [source[2].id],
      targetPageId: sourcePage.id,
      position: `after:${source[0].id}`,
      body: 'twoX',
    }]]);
    expect(outlineOf(await server.items(hostPage.id))).toEqual(['1:top']);
  });
});

test.describe('undoing a move inside a transclusion', () => {
  test('Ctrl+Z puts the moved child back on the source page, and the transclusion shows it there', async ({ page, request }) => {
    const server = new Server(request);
    const sourcePage = await server.newDocPage('undo-move-source');
    const source = await server.fill(sourcePage.id, ['top', '  one', '  two']);
    const hostPage = await server.newDocPage('undo-move-host');
    const host = await server.fill(hostPage.id, ['copy']);
    const transclude = `http://localhost:${serverPort}/api/items/${host[0].id}/transclude`;
    expect((await request.post(transclude, { data: { sourceItemId: source[0].id } })).ok()).toBe(true);
    await openPage(page, hostPage.title);
    await openEditor(page, source[2].id);
    await page.keyboard.press('Alt+Shift+ArrowUp');
    await server.itemsSettled(sourcePage.id, i => i[1].body === 'two');
    await expect.poll(() => shownBodies(page)).toEqual(['top', 'two', 'one']);

    await page.keyboard.press('Control+z');

    expect(outlineOf(await server.itemsSettled(sourcePage.id, i => i[1].body === 'one')))
      .toEqual(['1:top', '2:one', '2:two']);
    await expect.poll(() => shownBodies(page)).toEqual(['top', 'one', 'two']);
  });
});
