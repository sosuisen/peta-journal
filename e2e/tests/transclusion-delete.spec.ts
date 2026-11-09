import { expect } from '@playwright/test';
import { serverPort } from '../playwright.config';
import { Server, StoredItem, editorOf, openEditor, openPage, outlineOf, test } from './support';

// An item another page transcludes is deleted only after the user says OK in a dialog whose
// Cancel has the focus, so Enter alone keeps the item.
test.describe('deleting a transcluded item', () => {
  let server: Server;
  let pageId: string;
  let items: StoredItem[];

  test.beforeEach(async ({ page, request }) => {
    server = new Server(request);
    const source = await server.newDocPage('transcluded-source');
    pageId = source.id;
    items = await server.fill(pageId, ['shown elsewhere', 'plain']);
    const host = await server.newDocPage('transcluded-host');
    const [placeholder] = await server.fill(host.id, ['placeholder']);
    const resp = await request.post(
      `http://localhost:${serverPort}/api/items/${placeholder.id}/transclude`,
      { data: { sourceItemId: items[0].id } },
    );
    expect(resp.ok()).toBe(true);
    await openPage(page, source.title);
  });

  async function selectFirstAndDelete(page: import('@playwright/test').Page) {
    await openEditor(page, items[0].id);
    await page.keyboard.press('Shift+ArrowDown');  // leaves the editor: the item is selected
    await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(1);
    await page.keyboard.press('Delete');
  }

  test('the dialog asks first, and Enter on its focused Cancel keeps the item and the selection', async ({ page }) => {
    await selectFirstAndDelete(page);

    const dialog = page.locator('.confirm-dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText('transcluded by another item');
    await expect(dialog.getByRole('button', { name: 'Cancel' })).toBeFocused();

    await page.keyboard.press('Enter');

    await expect(dialog).toBeHidden();
    await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(1);
    expect(outlineOf(await server.items(pageId))).toEqual(['1:shown elsewhere', '1:plain']);
  });

  test('Escape closes the dialog and keeps the item and the selection', async ({ page }) => {
    await selectFirstAndDelete(page);
    await expect(page.locator('.confirm-dialog')).toBeVisible();

    await page.keyboard.press('Escape');

    await expect(page.locator('.confirm-dialog')).toBeHidden();
    await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(1);
    expect(outlineOf(await server.items(pageId))).toEqual(['1:shown elsewhere', '1:plain']);
  });

  test('OK deletes the item', async ({ page }) => {
    await selectFirstAndDelete(page);

    await page.locator('.confirm-dialog').getByRole('button', { name: 'OK' }).click();

    expect(outlineOf(await server.itemsSettled(pageId, i => i.length === 1))).toEqual(['1:plain']);
  });

  test('an item nothing transcludes is deleted without a dialog', async ({ page }) => {
    await openEditor(page, items[1].id);
    await page.keyboard.press('Shift+ArrowDown');  // leaves the editor: the item is selected
    await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(1);
    await page.keyboard.press('Delete');

    expect(outlineOf(await server.itemsSettled(pageId, i => i.length === 1))).toEqual(['1:shown elsewhere']);
    await expect(page.locator('.confirm-dialog')).toHaveCount(0);
  });
});

// A1, A2 and A3 in this order, with A2 transcluded by an item on another page. Joining A2
// into A1 deletes A2, so the editor asks first, as the selection does.
test.describe('joining a transcluded item in the editor', () => {
  let server: Server;
  let pageId: string;
  let items: StoredItem[];

  test.beforeEach(async ({ page, request }) => {
    server = new Server(request);
    const source = await server.newDocPage('joined-source');
    pageId = source.id;
    items = await server.fill(pageId, ['one', 'two', 'three']);
    const host = await server.newDocPage('joined-host');
    const [placeholder] = await server.fill(host.id, ['placeholder']);
    const resp = await request.post(
      `http://localhost:${serverPort}/api/items/${placeholder.id}/transclude`,
      { data: { sourceItemId: items[1].id } },
    );
    expect(resp.ok()).toBe(true);
    await openPage(page, source.title);
  });

  test('Backspace at the start of A2 asks first, and Cancel keeps A2 with the cursor at its start', async ({ page }) => {
    await openEditor(page, items[1].id);
    await page.keyboard.press('Home');
    await page.keyboard.press('Backspace');

    const dialog = page.locator('.confirm-dialog');
    await expect(dialog).toBeVisible();
    await page.keyboard.press('Enter');  // Cancel has the focus

    await expect(dialog).toBeHidden();
    const editor = editorOf(page, items[1].id);
    await expect(editor).toBeFocused();
    expect(await editor.evaluate((el: HTMLTextAreaElement) => el.selectionStart)).toBe(0);
    expect(outlineOf(await server.items(pageId))).toEqual(['1:one', '1:two', '1:three']);
  });

  test('Backspace at the start of A2 joins it into A1 after OK', async ({ page }) => {
    await openEditor(page, items[1].id);
    await page.keyboard.press('Home');
    await page.keyboard.press('Backspace');

    await page.locator('.confirm-dialog').getByRole('button', { name: 'OK' }).click();

    expect(outlineOf(await server.itemsSettled(pageId, i => i.length === 2))).toEqual(['1:onetwo', '1:three']);
    await expect(editorOf(page, items[0].id)).toBeFocused();
  });

  test('Delete at the end of A1 asks first, and Escape keeps A2 with the cursor at the end of A1', async ({ page }) => {
    await openEditor(page, items[0].id);
    await page.keyboard.press('End');
    await page.keyboard.press('Delete');

    await expect(page.locator('.confirm-dialog')).toBeVisible();
    await page.keyboard.press('Escape');

    await expect(page.locator('.confirm-dialog')).toBeHidden();
    const editor = editorOf(page, items[0].id);
    await expect(editor).toBeFocused();
    expect(await editor.evaluate((el: HTMLTextAreaElement) => el.selectionStart)).toBe(3);
    expect(outlineOf(await server.items(pageId))).toEqual(['1:one', '1:two', '1:three']);
  });
});

// Item B on the host page transcludes item A, which has a child. Deleting a selection with B
// in it turns B into a plain item first and then deletes B. A and its child stay on their page.
test.describe('deleting a selected transclusion', () => {
  let server: Server;
  let sourceId: string;
  let hostId: string;
  let host: StoredItem[];

  test.beforeEach(async ({ page, request }) => {
    server = new Server(request);
    const source = await server.newDocPage('selected-source');
    sourceId = source.id;
    const [a] = await server.fill(sourceId, ['source', '  child']);
    const hostPage = await server.newDocPage('selected-host');
    hostId = hostPage.id;
    host = await server.fill(hostId, ['before', 'placeholder', 'after']);
    const resp = await request.post(
      `http://localhost:${serverPort}/api/items/${host[1].id}/transclude`,
      { data: { sourceItemId: a.id } },
    );
    expect(resp.ok()).toBe(true);
    await openPage(page, hostPage.title);
    await expect(page.getByText('child')).toBeVisible();
  });

  for (const key of ['Delete', 'Backspace', 'Control+x']) {
    test(`${key} deletes the transclusion and keeps its source`, async ({ page }) => {
      await openEditor(page, host[0].id);
      await page.keyboard.press('Shift+ArrowDown');  // leaves the editor: "before" is selected
      await page.keyboard.press('Shift+ArrowDown');  // B and what it shows join the selection
      await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(3);
      await page.keyboard.press(key);

      expect(outlineOf(await server.itemsSettled(hostId, i => i.length === 1))).toEqual(['1:after']);
      expect(outlineOf(await server.items(sourceId))).toEqual(['1:source', '2:child']);
      await expect(page.locator('.confirm-dialog')).toHaveCount(0);
    });
  }
});
