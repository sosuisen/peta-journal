import { expect } from '@playwright/test';
import { serverPort } from '../playwright.config';
import { Server, editorOf, openEditor, openPage, outlineOf, test } from './support';

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

    await page.locator('app-outline-item a[data-wikilink]', { hasText: target.title }).click();

    await expect(page).toHaveURL(new RegExp(`/pages/${target.title}$`));
    await expect(page.locator('app-outline-item').first()).toBeVisible();
  });

  // 表示中の項目のウィキリンクは、ブラウザがそのまま開けるように、/pages/ の下を指す
  test('a wiki link in a shown item points under /pages/ for the browser', async ({ page, request }) => {
    const target = await new Server(request).newDocPage('Pointed');
    await page.keyboard.type(`go to [[${target.title}`);  // the editor closes the link itself
    await server.itemsSettled(pageId, i => i[0].body.endsWith(`${target.title}]]`));
    await page.reload();

    await expect(page.locator('app-outline-item a[data-wikilink]'))
      .toHaveAttribute('href', `/pages/${target.title}`);
  });

  // 表示中の項目のハッシュタグは、タグのページを指す
  test('a hashtag in a shown item points at the page of the tag', async ({ page }) => {
    const tag = `tag${Date.now()}`;
    await page.keyboard.type(`about #${tag}`);
    await server.itemsSettled(pageId, i => i[0].body.endsWith(tag));
    await page.reload();

    await expect(page.locator('app-outline-item a.hashtag'))
      .toHaveAttribute('href', new RegExp(`^/pages/(.*/)?${tag}$`));
  });

  // 画像のURLの w と h は、表示中の画像の幅と高さになる
  test('the w and h of an image URL size the shown image', async ({ page }) => {
    await pasteText(page, '![pic](pic.png#w=120&h=80)');
    await server.itemsSettled(pageId, i => i[0].body.includes('pic.png'));
    await page.reload();

    const image = page.locator('app-outline-item img');
    await expect(image).toHaveAttribute('width', '120');
    await expect(image).toHaveAttribute('height', '80');
  });

  // 言語を書いたコードブロックは、色が付き、言語の札が出る
  test('a code block that names its language is coloured and shows the name', async ({ page }) => {
    await page.keyboard.type('```rust');  // the editor closes the fence itself
    await page.keyboard.press('Enter');
    await page.keyboard.type('fn main() {}');
    await server.itemsSettled(pageId, i => i[0].body.includes('fn main'));
    await page.reload();

    await expect(page.locator('app-outline-item pre code.hljs')).toHaveCount(1);
    await expect(page.locator('app-outline-item .code-lang-label')).toHaveText('rust');
  });

  // 言語を書かないコードブロックは、色を付けず、札も出さない
  test('a code block without a language is neither coloured nor named', async ({ page }) => {
    await page.keyboard.type('```');  // the editor closes the fence itself
    await page.keyboard.press('Enter');
    await page.keyboard.type('plain words');
    await server.itemsSettled(pageId, i => i[0].body.includes('plain words'));
    await page.reload();

    await expect(page.locator('app-outline-item pre code')).toHaveText(/plain words/);
    await expect(page.locator('app-outline-item pre code.hljs')).toHaveCount(0);
    await expect(page.locator('app-outline-item .code-lang-label')).toHaveCount(0);
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

// Item B on the host page transcludes item A. A paste into B is a paste into A: the text and
// every item it makes go to the page of A, like the text typed into B does.
test.describe('pasting into a transclusion', () => {
  test('the pasted text and the items it makes go to the source page, with the text typed just before', async ({ page, request }) => {
    const server = new Server(request);
    const source = await server.newDocPage('paste-source');
    const [a] = await server.fill(source.id, ['source', 'next']);
    const host = await server.newDocPage('paste-host');
    const [b] = await server.fill(host.id, ['placeholder']);
    const resp = await request.post(
      `http://localhost:${serverPort}/api/items/${b.id}/transclude`,
      { data: { sourceItemId: a.id } },
    );
    expect(resp.ok()).toBe(true);
    await openPage(page, host.title);
    await openEditor(page, b.id);
    await page.keyboard.press('Control+End');
    await page.keyboard.type(' typed ');

    await pasteText(page, 'one\n\ntwo');

    expect(outlineOf(await server.itemsSettled(source.id, i => i.length === 3)))
      .toEqual(['1:source typed one', '1:two', '1:next']);
    // The host page gets no item: it keeps only its copy of the source
    expect(outlineOf(await server.items(host.id))).toEqual(['1:source typed one']);
    await expect(editorOf(page, b.id)).toBeFocused();
    await expect(editorOf(page, b.id)).toHaveValue('source typed one');
  });
});
