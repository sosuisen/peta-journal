import { APIRequestContext, Page, expect } from '@playwright/test';
import { serverPort } from '../playwright.config';
import { Server, StoredItem, editorOf, openEditor, openPage, outlineOf, shownBodies, test } from './support';

// The item "copy" of the host page transcludes an item of the source page. Enter inside what
// it shows is one request: the server splits the source item. Text of the source page never
// moves to the host page; only an empty item is made there.
test.describe('Enter inside a transclusion', () => {
  let server: Server;
  let sourceId: string;
  let hostId: string;
  let source: StoredItem[];
  let host: StoredItem[];

  /** Fills both pages, makes the row `root` of the host page show the row `shown` of the source page, and opens the host page. */
  async function open(
    page: Page, request: APIRequestContext,
    sourceRows: string[], shown: number, hostRows: string[], root: number,
    collapsed = false,
  ): Promise<void> {
    server = new Server(request);
    const sourcePage = await server.newDocPage('enter-source');
    sourceId = sourcePage.id;
    source = await server.fill(sourceId, sourceRows);
    const hostPage = await server.newDocPage('enter-host');
    hostId = hostPage.id;
    host = await server.fill(hostId, hostRows);
    const base = `http://localhost:${serverPort}/api/items/${host[root].id}`;
    expect((await request.post(`${base}/transclude`, { data: { sourceItemId: source[shown].id } })).ok()).toBe(true);
    if (collapsed) expect((await request.put(base, { data: { collapsed: true } })).ok()).toBe(true);
    await openPage(page, hostPage.title);
  }

  /** Opens the editor of an item and puts the cursor after `count` characters. */
  async function editAt(page: Page, itemId: string, count: number): Promise<void> {
    await openEditor(page, itemId);
    await editorOf(page, itemId).evaluate((e: HTMLTextAreaElement, at: number) => e.setSelectionRange(at, at), count);
  }

  test('in the middle of a transclusion without children, the rest goes below the source item and leaves this page', async ({ page, request }) => {
    await open(page, request, ['top', '  alpha-beta', '  next'], 1, ['copy', 'below'], 0);
    const writes: string[] = [];
    page.on('request', r => {
      if (r.method() !== 'GET') writes.push(`${r.method()} ${new URL(r.url()).pathname} ${r.postData()}`);
    });

    await editAt(page, host[0].id, 6);
    await page.keyboard.press('Enter');

    expect(outlineOf(await server.itemsSettled(sourceId, i => i.length === 4)))
      .toEqual(['1:top', '2:alpha-', '2:beta', '2:next']);
    expect(outlineOf(await server.items(hostId))).toEqual(['1:alpha-', '1:below']);
    // The edit stays where it was: the new item is not on this page
    await expect(editorOf(page, host[0].id)).toBeFocused();
    await expect(editorOf(page, host[0].id)).toHaveValue('alpha-');
    expect(await shownBodies(page)).toEqual(['alpha-', 'below']);
    expect(writes).toEqual([`POST /api/items/${host[0].id}/split {"body":"alpha-beta","cursor":6}`]);
  });

  test('text typed into a transclusion right before Enter is cut with the rest of its text', async ({ page, request }) => {
    await open(page, request, ['alpha', 'next'], 0, ['copy'], 0);

    await editAt(page, host[0].id, 'alpha'.length);
    await page.keyboard.type('-beta');
    for (let i = 0; i < 4; i++) await page.keyboard.press('ArrowLeft');
    await page.keyboard.press('Enter');

    expect(outlineOf(await server.itemsSettled(sourceId, i => i.length === 3)))
      .toEqual(['1:alpha-', '1:beta', '1:next']);
    await expect(editorOf(page, host[0].id)).toHaveValue('alpha-');
    // Longer than the auto-save waits: the typed text must not be saved over the split
    await page.waitForTimeout(600);
    expect(outlineOf(await server.items(sourceId))).toEqual(['1:alpha-', '1:beta', '1:next']);
  });

  test('in the middle of a collapsed transclusion, the rest goes after the children of the source item', async ({ page, request }) => {
    await open(page, request, ['top', '  alpha-beta', '    child', '  next'], 1, ['copy'], 0, true);

    await editAt(page, host[0].id, 6);
    await page.keyboard.press('Enter');

    expect(outlineOf(await server.itemsSettled(sourceId, i => i.length === 5)))
      .toEqual(['1:top', '2:alpha-', '3:child', '2:beta', '2:next']);
    await expect(editorOf(page, host[0].id)).toBeFocused();
    await expect(editorOf(page, host[0].id)).toHaveValue('alpha-');
  });

  test('in the middle of a transclusion that shows children, the rest is shown as its first child and edited', async ({ page, request }) => {
    await open(page, request, ['alpha-beta', '  child'], 0, ['copy'], 0);

    await editAt(page, host[0].id, 6);
    await page.keyboard.press('Enter');
    await page.keyboard.type('gamma-');

    const items = await server.itemsSettled(sourceId, i => i.length === 3 && i[1].body === 'gamma-beta');
    expect(outlineOf(items)).toEqual(['1:alpha-', '2:gamma-beta', '2:child']);
    await expect(editorOf(page, items[1].id)).toBeFocused();
    expect(await shownBodies(page)).toEqual(['alpha-', 'gamma-beta', 'child']);
  });

  test('at the end of the last item a transclusion shows, an empty item is made below the transclusion on this page', async ({ page, request }) => {
    await open(page, request, ['alpha', '  child'], 0, ['copy', 'below'], 0);

    await editAt(page, source[1].id, 'child'.length);
    await page.keyboard.press('Enter');
    await page.keyboard.type('typed here');

    const items = await server.itemsSettled(hostId, i => i.length === 3 && i[1].body === 'typed here');
    expect(outlineOf(items)).toEqual(['1:alpha', '1:typed here', '1:below']);
    expect(outlineOf(await server.items(sourceId))).toEqual(['1:alpha', '2:child']);
    await expect(editorOf(page, items[1].id)).toBeFocused();
    expect(await shownBodies(page)).toEqual(['alpha', 'child', 'typed here', 'below']);
  });

  test('at the start of a transclusion, an empty item is made above it on this page', async ({ page, request }) => {
    await open(page, request, ['alpha'], 0, ['above', 'copy'], 1);

    await editAt(page, host[1].id, 0);
    await page.keyboard.press('Enter');

    const items = await server.itemsSettled(hostId, i => i.length === 3);
    expect(outlineOf(items)).toEqual(['1:above', '1:', '1:alpha']);
    expect(outlineOf(await server.items(sourceId))).toEqual(['1:alpha']);
    await expect(editorOf(page, items[1].id)).toBeFocused();
  });

  test('Ctrl+Z takes back a split whose new item is not on this page', async ({ page, request }) => {
    await open(page, request, ['alpha-beta', 'next'], 0, ['copy'], 0);

    await editAt(page, host[0].id, 6);
    await page.keyboard.press('Enter');
    await server.itemsSettled(sourceId, i => i.length === 3);
    await expect(editorOf(page, host[0].id)).toHaveValue('alpha-');
    await page.keyboard.press('Control+z');

    expect(outlineOf(await server.itemsSettled(sourceId, i => i.length === 2)))
      .toEqual(['1:alpha-beta', '1:next']);
    await expect.poll(() => shownBodies(page)).toEqual(['alpha-beta']);
  });

  // The requests go one at a time, so the page that is read again after the split can be slow
  // to come. On 2026-11-18 its answer, with the page as it was before the undo, put the split
  // back on the screen while the server had it taken back.
  test('Ctrl+Z pressed while the page is read again after the split is not undone by the answer', async ({ page, request }) => {
    await open(page, request, ['alpha-beta', 'next'], 0, ['copy'], 0);
    let hold = false;
    let release = () => {};
    const held = new Promise<void>(resolve => { release = resolve; });
    let sawReading = () => {};
    const readingOnItsWay = new Promise<void>(resolve => { sawReading = resolve; });
    await page.route(`**/api/pages/${hostId}`, async route => {
      if (hold && route.request().method() === 'GET') {
        hold = false;
        sawReading();
        await held;
      }
      await route.continue();
    });

    await editAt(page, host[0].id, 6);
    hold = true;
    await page.keyboard.press('Enter');
    await readingOnItsWay;
    await page.keyboard.press('Control+z');
    await expect.poll(() => shownBodies(page)).toEqual(['alpha-beta']);
    release();

    expect(outlineOf(await server.itemsSettled(sourceId, i => i.length === 2)))
      .toEqual(['1:alpha-beta', '1:next']);
    // Time enough for the answer of the reading to reach the screen
    await page.waitForTimeout(500);
    expect(await shownBodies(page)).toEqual(['alpha-beta']);
    await expect(editorOf(page, host[0].id)).toBeFocused();
    await expect(editorOf(page, host[0].id)).toHaveValue('alpha-beta');
    expect(await editorOf(page, host[0].id).evaluate((e: HTMLTextAreaElement) => e.selectionStart)).toBe(6);
  });
});
