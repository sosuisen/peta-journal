import { APIRequestContext, Page, expect, test } from '@playwright/test';
import { serverPort } from '../playwright.config';

/** An item as the server stores it; only the fields the tests look at. */
interface StoredItem { id: string; level: number; body: string }

/** Talks to the journal server directly, past the dev server. */
class Server {
  constructor(private readonly request: APIRequestContext) {}

  /** Creates an empty doc page with a unique title and returns its id and title. */
  async newDocPage(prefix: string): Promise<{ id: string; title: string }> {
    const title = `${prefix}-${Date.now()}`;
    const resp = await this.request.post(`http://localhost:${serverPort}/api/pages/doc`, { data: { title, dirPath: '' } });
    expect(resp.ok()).toBe(true);
    const { page } = await resp.json();
    return { id: page.id, title };
  }

  async items(pageId: string): Promise<StoredItem[]> {
    const resp = await this.request.get(`http://localhost:${serverPort}/api/pages/${pageId}`);
    expect(resp.ok()).toBe(true);
    const { items } = await resp.json();
    return items.map((i: StoredItem) => ({ id: i.id, level: i.level, body: i.body }));
  }

  /** Waits until the stored items of the page satisfy the check; the auto-save needs a moment. */
  async itemsSettled(pageId: string, check: (items: StoredItem[]) => boolean): Promise<StoredItem[]> {
    await expect.poll(async () => check(await this.items(pageId)), { timeout: 5_000 }).toBe(true);
    return this.items(pageId);
  }
}

/** What the outline shows: one entry per item, with its stored level read from the server id. */
async function shownBodies(page: Page): Promise<string[]> {
  return page.locator('app-outline-item').evaluateAll(els =>
    els.map(e => (e.querySelector('textarea')?.value ?? e.querySelector('.item-body, app-item-view')?.textContent ?? '').trim()));
}

const editorOf = (page: Page, itemId: string) => page.locator(`#item-editor-${itemId}`);

test.describe('outline editing', () => {
  let server: Server;
  let pageId: string;

  test.beforeEach(async ({ page, request }) => {
    server = new Server(request);
    const doc = await server.newDocPage('e2e');
    pageId = doc.id;
    await page.goto(`/pages/${doc.title}`);
    await page.locator('app-outline-item').first().waitFor();
  });

  /** Opens the editor of the only item on the page and returns its id. */
  async function openFirstEditor(page: Page): Promise<string> {
    const [first] = await server.items(pageId);
    await page.locator(`[id="${first.id}"]`).click();
    await editorOf(page, first.id).waitFor();
    await expect(editorOf(page, first.id)).toBeFocused();
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
