import { expect, test } from '@playwright/test';
import { Server, openEditor, openPage, outlineOf } from './support';

// A selected item always comes with its descendants, so deleting it deletes them too.
test.describe('deleting a selected parent', () => {
  test('a selection started in the editor of a parent takes its children, and Delete removes them', async ({ page, request }) => {
    const server = new Server(request);
    const p = await server.newDocPage('parent-from-editor');
    const items = await server.fill(p.id, ['a', '  a1', 'b']);
    await openPage(page, p.title);
    await openEditor(page, items[0].id);

    await page.keyboard.press('Shift+ArrowDown');  // leaves the editor: a and a1 are selected
    await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(2);
    await page.keyboard.press('Delete');

    expect(outlineOf(await server.itemsSettled(p.id, i => i.length === 1))).toEqual(['1:b']);
  });

  test('undo of the deletion of a parent and its child brings both back on the server, with their ids', async ({ page, request }) => {
    const server = new Server(request);
    const p = await server.newDocPage('undo-parent-delete');
    const items = await server.fill(p.id, ['a', '  a1', 'b']);
    await openPage(page, p.title);
    await openEditor(page, items[0].id);
    await page.keyboard.press('Shift+ArrowDown');  // leaves the editor: a and a1 are selected
    await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(2);
    await page.keyboard.press('Delete');
    await server.itemsSettled(p.id, i => i.length === 1);

    await page.keyboard.press('Control+z');

    const back = await server.itemsSettled(p.id, i => i.length === 3);
    expect(outlineOf(back)).toEqual(['1:a', '2:a1', '1:b']);
    expect(back.map(i => i.id)).toEqual(items.map(i => i.id));
  });

  test('Delete removes a collapsed parent with its hidden children', async ({ page, request }) => {
    const server = new Server(request);
    const p = await server.newDocPage('collapsed-parent');
    const items = await server.fill(p.id, ['x', 'a', '  a1', 'b']);
    await openPage(page, p.title);
    await openEditor(page, items[1].id);
    await page.keyboard.press('Control+ArrowUp');
    await expect(page.locator('app-outline-item')).toHaveCount(3);

    await openEditor(page, items[0].id);
    await page.keyboard.press('Shift+ArrowDown');  // leaves the editor: x is selected
    await page.keyboard.press('Shift+ArrowDown');  // a joins, with its hidden child
    await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(2);
    await page.keyboard.press('Delete');

    expect(outlineOf(await server.itemsSettled(p.id, i => i.length === 1))).toEqual(['1:b']);
  });
});
