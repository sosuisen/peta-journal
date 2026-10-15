import { Page, expect, test } from '@playwright/test';
import { Server, editorOf, openEditor, openPage, shownBodies } from './support';

test.describe('outline editing', () => {
  let server: Server;
  let pageId: string;

  test.beforeEach(async ({ page, request }) => {
    server = new Server(request);
    const doc = await server.newDocPage('e2e');
    pageId = doc.id;
    await openPage(page, doc.title);
  });

  /** Opens the editor of the only item on the page and returns its id. */
  async function openFirstEditor(page: Page): Promise<string> {
    const [first] = await server.items(pageId);
    await openEditor(page, first.id);
    return first.id;
  }

  test('typing and Enter make a second item, and both reach the server', async ({ page }) => {
    const firstId = await openFirstEditor(page);

    await page.keyboard.type('first line');
    await page.keyboard.press('Enter');
    await page.keyboard.type('second line');

    const items = await server.itemsSettled(pageId, items => items.length === 2 && items[1].body === 'second line');
    expect(items.map(i => i.body)).toEqual(['first line', 'second line']);
    expect(items[0].id).toBe(firstId);
    await expect(editorOf(page, items[1].id)).toBeFocused();
  });

  test('Tab right after Enter indents the new item, not the one above', async ({ page }) => {
    await openFirstEditor(page);
    await page.keyboard.type('parent');

    // No pause between the keys: the new editor is not on screen yet when Tab arrives
    await page.keyboard.press('Enter');
    await page.keyboard.press('Tab');
    await page.keyboard.type('child');

    const items = await server.itemsSettled(pageId, items => items.length === 2 && items[1].body === 'child');
    expect(items.map(i => `${i.level}:${i.body}`)).toEqual(['1:parent', '2:child']);
  });

  test('text typed right after Enter goes into the new item only', async ({ page }) => {
    await openFirstEditor(page);
    await page.keyboard.type('alpha');

    await page.keyboard.press('Enter');
    await page.keyboard.type('beta');

    const items = await server.itemsSettled(pageId, items => items.length === 2 && items[1].body === 'beta');
    expect(items.map(i => i.body)).toEqual(['alpha', 'beta']);
    expect(await shownBodies(page)).toEqual(['alpha', 'beta']);
  });

  test('Enter in the middle of the text moves the rest to the new item', async ({ page }) => {
    await openFirstEditor(page);
    await page.keyboard.type('headtail');
    await page.keyboard.press('ArrowLeft', { delay: 0 });
    await page.keyboard.press('ArrowLeft');
    await page.keyboard.press('ArrowLeft');
    await page.keyboard.press('ArrowLeft');

    await page.keyboard.press('Enter');
    await page.keyboard.type('X');

    const items = await server.itemsSettled(pageId, items => items.length === 2 && items[1].body === 'Xtail');
    expect(items.map(i => i.body)).toEqual(['head', 'Xtail']);
  });

  test('Backspace at the start joins the item with the one above', async ({ page }) => {
    await openFirstEditor(page);
    await page.keyboard.type('one');
    await page.keyboard.press('Enter');
    await page.keyboard.type('two');
    await server.itemsSettled(pageId, items => items.length === 2 && items[1].body === 'two');

    await page.keyboard.press('Home');
    await page.keyboard.press('Backspace');

    const items = await server.itemsSettled(pageId, items => items.length === 1);
    expect(items[0].body).toBe('onetwo');
    await expect(editorOf(page, items[0].id)).toBeFocused();
  });

  test('Ctrl+Z takes back the split', async ({ page }) => {
    await openFirstEditor(page);
    await page.keyboard.type('keep');
    await page.keyboard.press('Enter');
    await server.itemsSettled(pageId, items => items.length === 2);

    await page.keyboard.press('Control+z');

    const items = await server.itemsSettled(pageId, items => items.length === 1);
    expect(items[0].body).toBe('keep');
  });
});
