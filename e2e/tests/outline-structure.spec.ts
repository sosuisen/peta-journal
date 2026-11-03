import { expect, test } from '@playwright/test';
import { Server, StoredItem, editorOf, openEditor, openPage, outlineOf } from './support';

test.describe('outline structure', () => {
  let server: Server;
  let pageId: string;
  let items: StoredItem[];
  const byBody = (body: string) => items.find(i => i.body === body)!;

  // * a
  //     * a1
  //     * a2
  // * b
  //     * b1
  test.beforeEach(async ({ page, request }) => {
    server = new Server(request);
    const doc = await server.newDocPage('structure');
    pageId = doc.id;
    items = await server.fill(pageId, ['a', '  a1', '  a2', 'b', '  b1']);
    await openPage(page, doc.title);
  });

  test('Tab indents the item with its children, Shift+Tab outdents it again', async ({ page }) => {
    await openEditor(page, byBody('b').id);

    await page.keyboard.press('Tab');
    expect(outlineOf(await server.itemsSettled(pageId, i => i[3].level === 2)))
      .toEqual(['1:a', '2:a1', '2:a2', '2:b', '3:b1']);

    await page.keyboard.press('Shift+Tab');
    expect(outlineOf(await server.itemsSettled(pageId, i => i[3].level === 1)))
      .toEqual(['1:a', '2:a1', '2:a2', '1:b', '2:b1']);
  });

  test('the first item cannot be indented and a top-level item cannot be outdented', async ({ page }) => {
    await openEditor(page, byBody('a').id);
    await page.keyboard.press('Tab');
    await page.keyboard.press('Shift+Tab');
    await page.waitForTimeout(500);

    expect(outlineOf(await server.items(pageId))).toEqual(['1:a', '2:a1', '2:a2', '1:b', '2:b1']);
  });

  test('Alt+Shift+Down moves the item with its children below the next sibling', async ({ page }) => {
    await openEditor(page, byBody('a').id);

    await page.keyboard.press('Alt+Shift+ArrowDown');

    expect(outlineOf(await server.itemsSettled(pageId, i => i[0].body === 'b')))
      .toEqual(['1:b', '2:b1', '1:a', '2:a1', '2:a2']);
    await expect(editorOf(page, byBody('a').id)).toBeFocused();
  });

  test('Alt+Shift+Up moves a child above its sibling', async ({ page }) => {
    await openEditor(page, byBody('a2').id);

    await page.keyboard.press('Alt+Shift+ArrowUp');

    expect(outlineOf(await server.itemsSettled(pageId, i => i[1].body === 'a2')))
      .toEqual(['1:a', '2:a2', '2:a1', '1:b', '2:b1']);
  });

  test('Ctrl+ArrowUp hides the children and Ctrl+ArrowDown shows them again', async ({ page }) => {
    await openEditor(page, byBody('a').id);
    const shown = page.locator('app-outline-item');

    await page.keyboard.press('Control+ArrowUp');
    await expect(shown).toHaveCount(3);
    expect((await server.itemsSettled(pageId, i => i[0].collapsed))[0].collapsed).toBe(true);

    await page.keyboard.press('Control+ArrowDown');
    await expect(shown).toHaveCount(5);
    expect((await server.itemsSettled(pageId, i => !i[0].collapsed))[0].collapsed).toBe(false);
  });

  test('Shift+Down selects items and Delete removes them', async ({ page }) => {
    await openEditor(page, byBody('a1').id);

    await page.keyboard.press('Shift+ArrowDown');  // leaves the editor: a1 is selected
    await page.keyboard.press('Shift+ArrowDown');  // a2
    await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(2);

    await page.keyboard.press('Delete');

    expect(outlineOf(await server.itemsSettled(pageId, i => i.length === 3)))
      .toEqual(['1:a', '1:b', '2:b1']);
  });

  test.describe('copying the selection', () => {
    // The clipboard is written through the asynchronous clipboard API, which asks for both
    test.use({ permissions: ['clipboard-read', 'clipboard-write'] });

    test('Ctrl+C puts the selection on the clipboard and a paste brings it back with its levels', async ({ page }) => {
      await openEditor(page, byBody('a1').id);
      await page.keyboard.press('Shift+ArrowDown');  // a1
      await page.keyboard.press('Shift+ArrowDown');  // a2
      await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(2);

      await page.keyboard.press('Control+c');

      // The write finishes only once the server has made the tree markdown
      await expect.poll(() => page.evaluate(() => navigator.clipboard.readText())).toBe('- a1\n- a2');

      await page.keyboard.press('Escape');
      await openEditor(page, byBody('b1').id);
      await page.keyboard.press('End');
      await page.keyboard.press('Enter');  // an empty item below b1
      await page.evaluate(async () => {
        const data = new DataTransfer();
        data.setData('text/plain', await navigator.clipboard.readText());
        document.activeElement!.dispatchEvent(new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true }));
      });

      expect(outlineOf(await server.itemsSettled(pageId, i => i.length === 7)))
        .toEqual(['1:a', '2:a1', '2:a2', '1:b', '2:b1', '2:a1', '2:a2']);
    });
  });

  test('a selection started at a parent takes its whole subtree, and grows past it', async ({ page }) => {
    await openEditor(page, byBody('a').id);

    await page.keyboard.press('Shift+ArrowDown');  // a, with a1 and a2
    await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(3);

    await page.keyboard.press('Shift+ArrowDown');  // b, with b1
    await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(5);
  });

  test('Escape ends a selection without changing anything', async ({ page }) => {
    await openEditor(page, byBody('b').id);
    await page.keyboard.press('Shift+ArrowDown');  // b, with b1
    await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(2);

    await page.keyboard.press('Escape');

    await expect(page.locator('app-outline-item .outline-item.selected')).toHaveCount(0);
    expect(outlineOf(await server.items(pageId))).toEqual(['1:a', '2:a1', '2:a2', '1:b', '2:b1']);
  });

  test('Ctrl+Enter turns the item into a task and shows its checkbox', async ({ page }) => {
    await openEditor(page, byBody('b1').id);

    await page.keyboard.press('Control+Enter');

    const [, , , , b1] = await server.itemsSettled(pageId, i => i[4].types.includes('task'));
    expect(b1.types).toContain('task');
    await expect(page.locator(`[id="${b1.id}"] .task-checkbox`)).toBeVisible();
  });

  test('Ctrl+Z takes back an indent and Ctrl+Shift+Z does it again', async ({ page }) => {
    await openEditor(page, byBody('b').id);
    await page.keyboard.press('Tab');
    await server.itemsSettled(pageId, i => i[3].level === 2);

    await page.keyboard.press('Control+z');
    expect(outlineOf(await server.itemsSettled(pageId, i => i[3].level === 1 && i[4].level === 2)))
      .toEqual(['1:a', '2:a1', '2:a2', '1:b', '2:b1']);

    await page.keyboard.press('Control+Shift+z');
    expect(outlineOf(await server.itemsSettled(pageId, i => i[3].level === 2 && i[4].level === 3)))
      .toEqual(['1:a', '2:a1', '2:a2', '2:b', '3:b1']);
  });
});
