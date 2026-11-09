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

  test('text typed right after Enter in the middle goes in front of the text that moved down', async ({ page }) => {
    await openFirstEditor(page);
    await page.keyboard.type('alpha-beta');
    for (let i = 0; i < 4; i++) await page.keyboard.press('ArrowLeft');

    await page.keyboard.press('Enter');
    await page.keyboard.type('gamma-');

    const items = await server.itemsSettled(pageId, items => items.length === 2 && items[1].body.length === 10);
    expect(items.map(i => i.body)).toEqual(['alpha-', 'gamma-beta']);
    expect(await shownBodies(page)).toEqual(['alpha-', 'gamma-beta']);
  });

  test('Enter sends one request, which carries the text typed and not saved yet', async ({ page }) => {
    const firstId = await openFirstEditor(page);
    const writes: string[] = [];
    page.on('request', r => {
      if (r.method() !== 'GET') writes.push(`${r.method()} ${new URL(r.url()).pathname} ${r.postData()}`);
    });

    await page.keyboard.type('alpha-beta');
    for (let i = 0; i < 4; i++) await page.keyboard.press('ArrowLeft');
    await page.keyboard.press('Enter');

    const items = await server.itemsSettled(pageId, items => items.length === 2);
    expect(items.map(i => i.body)).toEqual(['alpha-', 'beta']);
    await expect(editorOf(page, items[1].id)).toBeFocused();
    // Longer than the auto-save waits: the typed text must not be saved a second time
    await page.waitForTimeout(600);
    expect(writes).toEqual([`POST /api/items/${firstId}/split {"body":"alpha-beta","cursor":6}`]);
  });

  test('two quick Enters in a row make two items, each with its own text', async ({ page }) => {
    await openFirstEditor(page);

    await page.keyboard.type('a');
    await page.keyboard.press('Enter');
    await page.keyboard.type('b');
    await page.keyboard.press('Enter');
    await page.keyboard.type('c');

    const items = await server.itemsSettled(pageId, items => items.length === 3 && items[2].body === 'c');
    expect(items.map(i => i.body)).toEqual(['a', 'b', 'c']);
    expect(await shownBodies(page)).toEqual(['a', 'b', 'c']);
  });

  test('Backspace right after Enter deletes a typed character instead of being lost', async ({ page }) => {
    await openFirstEditor(page);
    await page.keyboard.type('one');

    await page.keyboard.press('Enter');
    await page.keyboard.type('ab');
    await page.keyboard.press('Backspace');

    const items = await server.itemsSettled(pageId, items => items.length === 2 && items[1].body === 'a');
    expect(items.map(i => i.body)).toEqual(['one', 'a']);
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

  test('the joined item keeps showing the joined text when the cursor moves down right after Backspace', async ({ page }) => {
    await openFirstEditor(page);
    await page.keyboard.type('one');
    await page.keyboard.press('Enter');
    await page.keyboard.type('two');
    await page.keyboard.press('Enter');
    await page.keyboard.type('three');
    const created = await server.itemsSettled(pageId, items => items.length === 3 && items[2].body === 'three');
    await page.keyboard.press('ArrowUp');
    await expect(editorOf(page, created[1].id)).toBeFocused();

    await page.keyboard.press('Home');
    await page.keyboard.press('Backspace');
    await expect(editorOf(page, created[0].id)).toBeFocused();
    await page.keyboard.press('ArrowDown');

    await expect(editorOf(page, created[2].id)).toBeFocused();
    await server.itemsSettled(pageId, items => items.length === 2 && items[0].body === 'onetwo');
    await expect.poll(() => shownBodies(page)).toEqual(['onetwo', 'three']);
  });

  test('the item shows the redone text when the cursor moves down after Redo', async ({ page }) => {
    await openFirstEditor(page);
    await page.keyboard.type('alpha');
    await page.keyboard.press('Enter');
    await page.keyboard.type('beta');
    const created = await server.itemsSettled(pageId, items => items.length === 2 && items[1].body === 'beta');
    await page.keyboard.press('ArrowUp');
    await expect(editorOf(page, created[0].id)).toBeFocused();

    await page.keyboard.press('End');
    await page.keyboard.type('X');
    await page.keyboard.press('Control+z');
    await expect(editorOf(page, created[0].id)).toHaveValue('alpha');
    await page.keyboard.press('Control+Shift+z');
    await expect(editorOf(page, created[0].id)).toHaveValue('alphaX');
    await server.itemsSettled(pageId, items => items[0].body === 'alphaX');
    await page.keyboard.press('ArrowDown');

    await expect(editorOf(page, created[1].id)).toBeFocused();
    await expect.poll(() => shownBodies(page)).toEqual(['alphaX', 'beta']);
  });

  test('text typed right before Backspace at the start reaches the server in the joined item', async ({ page }) => {
    await openFirstEditor(page);
    await page.keyboard.type('one');
    await page.keyboard.press('Enter');
    await server.itemsSettled(pageId, items => items.length === 2 && items[0].body === 'one');

    // No pause between the keys: the auto-save has not sent the text yet when the items join
    await page.keyboard.type('two');
    await page.keyboard.press('Home');
    await page.keyboard.press('Backspace');

    const items = await server.itemsSettled(pageId, items => items.length === 1 && items[0].body === 'onetwo');
    expect(items[0].body).toBe('onetwo');
    expect(await shownBodies(page)).toEqual(['onetwo']);
  });

  test('text typed right before Delete at the end reaches the server in the joined item', async ({ page }) => {
    const firstId = await openFirstEditor(page);
    await page.keyboard.type('one');
    await page.keyboard.press('Enter');
    await page.keyboard.type('two');
    await server.itemsSettled(pageId, items => items.length === 2 && items[1].body === 'two');
    await openEditor(page, firstId);

    // No pause between the keys: the auto-save has not sent the text yet when the items join
    await page.keyboard.press('End');
    await page.keyboard.type('!');
    await page.keyboard.press('Delete');

    const items = await server.itemsSettled(pageId, items => items.length === 1 && items[0].body === 'one!two');
    expect(items[0].body).toBe('one!two');
    // The pending auto-save must not arrive after the join and overwrite it
    await page.waitForTimeout(600);
    expect((await server.items(pageId)).map(i => i.body)).toEqual(['one!two']);
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
