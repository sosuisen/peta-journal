import { expect } from '@playwright/test';
import { Server, openEditor, openPage, test } from './support';

// The server keeps every page with at least one item, whoever deletes the last one.
test.describe('a page keeps one item', () => {
  test('an undo that takes away the last item leaves the empty item the server made', async ({ page, request }) => {
    const server = new Server(request);
    const p = await server.newDocPage('undo-empties');
    const [a] = await server.fill(p.id, ['a']);
    await openPage(page, p.title);
    await openEditor(page, a.id);
    await page.keyboard.press('End');
    await page.keyboard.press('Enter');
    const [, b] = await server.itemsSettled(p.id, i => i.length === 2);
    await expect(page.locator(`#item-editor-${b.id}`)).toBeFocused();
    // Another client deletes the first item: this undo history does not know of it
    await server.deleteItem(a.id);
    await server.itemsSettled(p.id, i => i.length === 1);

    await page.keyboard.press('Control+z');

    const left = await server.itemsSettled(p.id, i => i.length === 1 && i[0].id !== b.id);
    expect(left[0].body).toBe('');
    await expect(page.locator(`[id="${left[0].id}"]`)).toHaveCount(1);
  });
});
