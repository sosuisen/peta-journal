import { expect, test } from '@playwright/test';
import { serverPort } from '../playwright.config';
import { Server, StoredItem, openEditor, openPage, outlineOf } from './support';

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
