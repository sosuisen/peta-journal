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

  // 表示中の項目の隠し文字は、文字の色が背景の色と同じで、読めない
  test('spoilered text in a shown item has the colour of its background', async ({ page }) => {
    await page.keyboard.type('the end: ||secret||');
    await server.itemsSettled(pageId, i => i[0].body.endsWith('||'));
    await page.reload();

    const spoiler = page.locator('app-outline-item .spoiler');
    await expect(spoiler).toHaveText('secret');
    const { color, background } = await spoiler.evaluate(e => {
      const style = getComputedStyle(e);
      return { color: style.color, background: style.backgroundColor };
    });
    expect(background).not.toBe('rgba(0, 0, 0, 0)');
    expect(color).toBe(background);
  });

  /**
   * Shows a wiki link to a new page whose name has a `#`, a `%`, a space and Japanese, and
   * returns the link with the name of the page and the URL it should lead to (ADR 061).
   */
  async function linkToAnOddName(page: import('@playwright/test').Page, request: import('@playwright/test').APIRequestContext) {
    const target = await new Server(request).newDocPage('C# 100% メモ');
    // Pasted, not typed: a typed # opens the list of tags
    await pasteText(page, `go to [[${target.title}]]`);
    await server.itemsSettled(pageId, i => i[0].body.endsWith(`${target.title}]]`));
    await page.reload();
    const url = `/pages/${encodeURI(target.title).replaceAll('#', '%23')}`;
    const link = page.locator('app-outline-item a[data-wikilink]');
    return { title: target.title, url, link };
  }

  // 名前に#、%、空白、日本語を含むページへのウィキリンクは、符号化したURLを指す。右クリックでコピーするリンクも、このURLである
  test('a wiki link to a name with odd characters points at its encoded url', async ({ page, request }) => {
    const { url, link } = await linkToAnOddName(page, request);

    await expect(link).toHaveAttribute('href', url);
  });

  // そのウィキリンクをクリックすると、そのページが開く
  test('a click on a wiki link to a name with odd characters opens that page', async ({ page, request }) => {
    const { title, url, link } = await linkToAnOddName(page, request);

    await link.click();

    await expect(page).toHaveURL(url);
    await expect(page).toHaveTitle(title);
  });

  // そのウィキリンクのCtrl+クリック（MacではCmd+クリック）と中ボタンのクリックは、そのページを新しいタブで開く。
  // MacのCtrl+クリックは右クリックである
  for (const [how, click] of [
    ['Ctrl+click (Cmd+click on a Mac)', { modifiers: ['ControlOrMeta'] }],
    ['middle click', { button: 'middle' }],
  ] as const) {
    test(`a ${how} on a wiki link to a name with odd characters opens that page in a new tab`, async ({ page, request }) => {
      const { title, url, link } = await linkToAnOddName(page, request);

      const [tab] = await Promise.all([page.context().waitForEvent('page'), link.click(click)]);

      await tab.waitForLoadState();
      expect(new URL(tab.url()).pathname).toBe(url);
      await expect(tab).toHaveTitle(title);
    });
  }

  // コピーしたHTMLでは、ウィキリンクとハッシュタグが、アプリのURLから始まるリンクになる。貼り付けた先から開ける
  test('the copied HTML links a wiki link and a hashtag to whole URLs of the app', async ({ page, request }) => {
    const target = await new Server(request).newDocPage('Copied');
    const tag = `tag${Date.now()}`;
    await page.keyboard.type(`#${tag} and [[${target.title}`);  // the editor closes the link itself
    const [item] = await server.itemsSettled(pageId, i => i[0].body.endsWith(`${target.title}]]`));
    await page.reload();  // closes the list of page names, which would take the arrow key
    await openEditor(page, item.id);
    // The test keeps what the app writes, and does not read the real clipboard: reading it
    // needs a permission and the focus of the window.
    await page.evaluate(() => {
      navigator.clipboard.write = async items => {
        (window as unknown as { copied: ClipboardItem }).copied = items[0];
      };
    });

    await page.keyboard.press('Shift+ArrowDown');  // leaves the editor: the item is selected
    await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(1);
    await page.keyboard.press('Control+c');

    const links = await page.evaluate(async () => {
      const { copied } = window as unknown as { copied: ClipboardItem };
      const html = await (await copied.getType('text/html')).text();
      const anchors = new DOMParser().parseFromString(html, 'text/html').querySelectorAll('a');
      return Array.from(anchors, a => ({ href: a.getAttribute('href'), text: a.textContent }));
    });
    const { origin } = new URL(page.url());
    expect(links).toHaveLength(2);
    expect(links[0].href).toMatch(new RegExp(`^${origin}/pages/(.*/)?${tag}$`));
    expect(links[0].text).toBe(`#${tag}`);
    expect(links[1]).toEqual({ href: `${origin}/pages/${encodeURI(target.title)}`, text: target.title });
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

  test('typing a block opening other than a code fence closes it too', async ({ page }) => {
    await page.keyboard.type('>>>');
    await page.keyboard.press('Enter');
    await page.keyboard.type('quoted');

    const items = await server.itemsSettled(pageId, i => i[0].body.includes('quoted'));
    expect(items.map(i => i.body)).toEqual(['>>>\nquoted\n>>>']);
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

// Text typed into B goes into A in the core at once, so the row of B shows it before the save,
// as the row of any other item does.
test.describe('typing into a transclusion', () => {
  test('an opening typed into a transclusion is closed, and the text typed into it is saved to the source', async ({ page, request }) => {
    const server = new Server(request);
    const source = await server.newDocPage('typing-source');
    const [a] = await server.fill(source.id, ['memo']);
    const host = await server.newDocPage('typing-host');
    const [b] = await server.fill(host.id, ['placeholder']);
    const resp = await request.post(
      `http://localhost:${serverPort}/api/items/${b.id}/transclude`,
      { data: { sourceItemId: a.id } },
    );
    expect(resp.ok()).toBe(true);
    await openPage(page, host.title);
    await openEditor(page, b.id);
    await page.keyboard.press('Control+End');

    await page.keyboard.type(' [[');
    await expect(editorOf(page, b.id)).toHaveValue('memo [[]]');
    await page.keyboard.type('Plan');

    await expect(editorOf(page, b.id)).toHaveValue('memo [[Plan]]');
    const [saved] = await server.itemsSettled(source.id, i => i[0].body === 'memo [[Plan]]');
    expect(saved.body).toBe('memo [[Plan]]');
  });
});
