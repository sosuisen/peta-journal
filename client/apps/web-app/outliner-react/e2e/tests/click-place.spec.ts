import { expect, Page } from '@playwright/test';
import { Server, editorOf, openPage, test } from './support';

// A click on the shown text of an item puts the cursor at the place in its Markdown that stands
// there. The page sends the letters before the click, and the core lines them up with the
// Markdown (ADR 044).
test.describe('a click on the shown text', () => {
  let server: Server;

  test.beforeEach(async ({ request }) => {
    server = new Server(request);
  });

  /** Clicks on the left half of the letter `offset` letters into `word` of the shown item. */
  async function clickBefore(
    page: Page,
    itemId: string,
    word: string,
    offset: number,
  ) {
    const point = await page.locator(`[id="${itemId}"]`).evaluate(
      (el, [word, offset]) => {
        const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
        for (let node = walker.nextNode(); node; node = walker.nextNode()) {
          const at = (node.textContent ?? '').indexOf(word as string);
          if (at < 0) continue;
          const range = document.createRange();
          range.setStart(node, at + (offset as number));
          range.setEnd(node, at + (offset as number) + 1);
          const box = range.getBoundingClientRect();
          return { x: box.left + 1, y: box.top + box.height / 2 };
        }
        return null;
      },
      [word, offset] as const,
    );
    expect(point).not.toBeNull();
    await page.mouse.click(point!.x, point!.y);
  }

  async function cursorOf(page: Page, itemId: string): Promise<number> {
    const editor = editorOf(page, itemId);
    await expect(editor).toBeFocused();
    return editor.evaluate((el: HTMLTextAreaElement) => el.selectionStart);
  }

  // 太字やリンクの記号は飛ばされ、カーソルは、クリックした文字のMarkdownでの位置に置かれる。
  test('the marks of bold letters and of a link are left out', async ({
    page,
  }) => {
    const doc = await server.newDocPage('click-place-marks');
    const [item] = await server.fill(doc.id, [
      '**bold** and [link](https://x.example/a) end',
    ]);
    await openPage(page, doc.title);

    // A click on the link itself follows it: the click is after it
    await clickBefore(page, item.id, 'end', 1);

    expect(await cursorOf(page, item.id)).toBe(
      '**bold** and [link](https://x.example/a) e'.length,
    );
  });

  // リストの項目では、ページの上のタグの間の改行や印は飛ばされる。
  test('a click in an entry of a list leaves out what the page adds', async ({
    page,
  }) => {
    const doc = await server.newDocPage('click-place-list');
    const [item] = await server.fill(doc.id, ['- first\n- second']);
    await openPage(page, doc.title);

    await clickBefore(page, item.id, 'second', 2);

    expect(await cursorOf(page, item.id)).toBe('- first\n- se'.length);
  });
});
