import { expect, test } from '@playwright/test';
import { Server, editorOf, openEditor, openPage, outlineOf } from './support';

/** Pastes text into the focused editor the way the browser would. */
async function pasteText(page: import('@playwright/test').Page, text: string): Promise<void> {
  await page.evaluate(text => {
    const data = new DataTransfer();
    data.setData('text/plain', text);
    document.activeElement!.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
  }, text);
}

test.describe('outline content', () => {
  let server: Server;
  let pageId: string;

  test.beforeEach(async ({ page, request }) => {
    server = new Server(request);
    const doc = await server.newDocPage('content');
    pageId = doc.id;
    await openPage(page, doc.title);
    const [first] = await server.items(pageId);
    await openEditor(page, first.id);
  });

  test('pasting several paragraphs makes one item per paragraph', async ({ page }) => {
    await page.keyboard.type('intro ');

    await pasteText(page, 'one\n\ntwo\n\nthree');

    expect(outlineOf(await server.itemsSettled(pageId, i => i.length === 3)))
      .toEqual(['1:intro one', '1:two', '1:three']);
  });

  test('pasting a markdown list keeps its nesting', async ({ page }) => {
    await pasteText(page, '- parent\n  - child\n- sibling');

    expect(outlineOf(await server.itemsSettled(pageId, i => i.length === 3)))
      .toEqual(['1:parent', '2:child', '1:sibling']);
  });

  test('typing [[ offers matching page names and Enter completes the link', async ({ page, request }) => {
    const target = await new Server(request).newDocPage('Target');

    await page.keyboard.type('see [[Targ');

    const suggestion = page.locator('.wiki-suggestion-item', { hasText: target.title });
    await expect(suggestion).toBeVisible();
    await page.keyboard.press('Enter');

    const [item] = await server.itemsSettled(pageId, i => i[0].body.includes(target.title));
    expect(item.body).toBe(`see [[${target.title}]]`);
    await expect(page.locator('.wiki-suggestions')).toHaveCount(0);
  });

  test('typing the closing brackets of a wiki link goes over them', async ({ page }) => {
    await page.keyboard.type('see [[Plan]] now');

    const [item] = await server.itemsSettled(pageId, i => i[0].body.endsWith('now'));
    expect(item.body).toBe('see [[Plan]] now');
  });

  test('a wiki link in a shown item opens the page it names', async ({ page, request }) => {
    const target = await new Server(request).newDocPage('Opened');
    await page.keyboard.type(`go to [[${target.title}`);  // the editor closes the link itself
    await server.itemsSettled(pageId, i => i[0].body.endsWith(`${target.title}]]`));
    await page.reload();  // shows the item as rendered text instead of the editor

    await page.locator('app-outline-item a.wiki-link', { hasText: target.title }).click();

    await expect(page).toHaveURL(new RegExp(`/pages/${target.title}$`));
    await expect(page.locator('app-outline-item').first()).toBeVisible();
  });

  test('Enter inside a code fence adds a line and the caret follows it', async ({ page }) => {
    await page.keyboard.type('```');  // the editor closes the fence itself
    await page.keyboard.press('Enter');
    await page.keyboard.type('one');
    await page.keyboard.press('Enter');
    await page.keyboard.type('two');

    const items = await server.itemsSettled(pageId, i => i[0].body.includes('two'));
    expect(items.map(i => i.body)).toEqual(['```\none\ntwo\n```']);
  });

  test('Ctrl+O opens the wiki link at the caret', async ({ page, request }) => {
    const target = await new Server(request).newDocPage('CtrlO');
    await page.keyboard.type(`go to [[${target.title}`);  // the editor closes the link itself

    await page.keyboard.press('Control+o');

    await expect(page).toHaveURL(new RegExp(`/pages/${target.title}$`));
  });

  test('Ctrl+O opens the page of the tag at the caret', async ({ page }) => {
    const tag = `tag${Date.now()}`;
    await page.keyboard.type(`about #${tag}`);

    await page.keyboard.press('Control+o');

    await expect(page).toHaveURL(new RegExp(`/pages/(.*/)?${tag}$`));
  });

  test('Shift+Enter puts a line break inside the item instead of splitting it', async ({ page }) => {
    const [first] = await server.items(pageId);
    await page.keyboard.type('line one');
    await page.keyboard.press('Shift+Enter');
    await page.keyboard.type('line two');

    const items = await server.itemsSettled(pageId, i => i[0].body.includes('line two'));
    expect(items.map(i => i.body)).toEqual(['line one\nline two']);
    await expect(editorOf(page, first.id)).toBeFocused();
  });
});
