import { expect, type Page } from '@playwright/test';
import { Server, editorOf, openEditor, openPage, test } from './support';

const PNG_1X1 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==';

/** Pastes a small PNG into the focused editor, the way the clipboard hands over an image. */
async function pasteImage(page: Page, itemId: string): Promise<void> {
  await editorOf(page, itemId).evaluate((el, base64) => {
    const bytes = Uint8Array.from(atob(base64), c => c.charCodeAt(0));
    const data = new DataTransfer();
    data.items.add(new File([bytes], 'pic.png', { type: 'image/png' }));
    el.dispatchEvent(
      new ClipboardEvent('paste', {
        clipboardData: data,
        bubbles: true,
        cancelable: true,
      }),
    );
  }, PNG_1X1);
}

test.describe('pasting an image', () => {
  test('the image is uploaded and its link goes in at the cursor with one request, and Ctrl+Z takes it back', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('image-paste');
    const [a] = await server.fill(p.id, ['abcd']);
    await openPage(page, p.title);
    await openEditor(page, a.id);
    await page.keyboard.press('Home');
    await page.keyboard.press('ArrowRight');
    await page.keyboard.press('ArrowRight');
    const writes: string[] = [];
    page.on('request', r => {
      if (r.method() !== 'GET')
        writes.push(`${r.method()} ${new URL(r.url()).pathname}`);
    });

    await pasteImage(page, a.id);

    // The text is the name the file came with. The file is saved under an HMTID (ADR 022)
    const link = /^ab!\[pic\.png\]\(\.\.\/assets\/\d{14}_[0-9A-Z]{7}\.png\)cd$/;
    const [pasted] = await server.itemsSettled(p.id, i => link.test(i[0].body));
    await expect(editorOf(page, a.id)).toHaveValue(pasted.body);
    expect(writes).toEqual([
      'POST /api/assets/upload',
      `POST /api/items/${a.id}/paste`,
    ]);

    await page.keyboard.press('Control+z');

    await server.itemsSettled(p.id, i => i[0].body === 'abcd');
  });
});
