import { Page, expect } from '@playwright/test';
import { Server, test } from './support';
import { serverPort } from '../playwright.config';

const base = `http://localhost:${serverPort}`;

/** The point of the screen where a point of the canvas of the atlas lies, at its first view */
async function onScreen(
  page: Page,
  x: number,
  y: number,
): Promise<[number, number]> {
  const box = await page.locator('.atlas-canvas canvas').boundingBox();
  if (box === null) throw new Error('no canvas');
  return [box.x + box.width / 2 + x, box.y + box.height / 2 + y];
}

/** Right-clicks a point of the canvas and picks the first item of the menu drawn there */
async function firstOfMenu(page: Page, x: number, y: number): Promise<void> {
  await page.mouse.click(x, y, { button: 'right' });
  // The menu lies with its top left at the pointer; its first item is 30 high, after 4
  await page.mouse.click(x + 70, y + 19);
}

// カードのEditは、項目の本文をMarkdownのまま複数行の入力に出し、OKで書いたものを本文にする。キャンバスのAdd Cardは、書いたものを本文に、右クリックした点にカードを作る。Previewのタブは、書いたものをページの本文と同じに描く。
test.describe('Edit and Add Card of the canvas', () => {
  test('Edit writes the Markdown into the body of the item, and Add Card makes an item at the point', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const folder = await server.newDocPage('edit-card');
    const dir = `${folder.title}/`;
    // A page that a link of the new card names
    const target = await server.newDocPage('link-target');
    const atlas = await server.newAtlas(dir);
    const [plain] = await server.fill(atlas.id, ['plain note']);
    const placed = await request.put(
      `${base}/api/items/${plain.id}/properties/atlas_bounds`,
      { data: { valueType: 'bounds', value: '0,0,0,40' } },
    );
    expect(placed.ok()).toBe(true);
    await page.goto(`/pages/${dir}index_atlas`);
    await expect(page.locator('.atlas-canvas canvas')).toBeVisible();

    // Add Card where nothing is. The menu of the canvas needs no card to be drawn.
    const [ex, ey] = await onScreen(page, 0, 120);
    await firstOfMenu(page, ex, ey);
    const dialog = page.getByRole('dialog', { name: 'Add Card' });
    // Enter breaks the line; the Preview draws both lines as the page would
    await page.keyboard.type('a new card');
    await page.keyboard.press('Enter');
    await page.keyboard.type('**second** line');
    // A `[[` is closed as in the editor of a page, and the page names come to pick from
    await page.keyboard.press('Enter');
    await page.keyboard.type(`see [[${target.title.slice(0, 8)}`);
    await dialog
      .locator('.atlas-dialog-suggestion', { hasText: target.title })
      .click();
    await dialog.getByRole('tab', { name: 'Preview' }).click();
    const preview = dialog.locator('.atlas-dialog-preview .item-content');
    await expect(preview).toContainText('a new card');
    await expect(preview.locator('strong')).toHaveText('second');
    await expect(preview.locator('a[data-wikilink]')).toHaveText(target.title);
    await dialog.getByRole('button', { name: 'OK' }).click();
    const added = await server.itemsSettled(atlas.id, all => all.length === 2);

    // Edit of the plain card. The new card may still be drawing the cards anew; a right click
    // takes the card under it all the same. The cursor is at the end of its body.
    const [cx, cy] = await onScreen(page, 0, 0);
    await firstOfMenu(page, cx, cy);
    await expect(page.getByRole('dialog', { name: 'Edit' })).toBeVisible();
    await page.keyboard.type(' more');
    await page.keyboard.press('Control+Enter');
    const edited = await server.itemsSettled(
      atlas.id,
      all => all[0]?.body !== 'plain note',
    );

    // The middle of the canvas may fall between two pixels of the screen
    const place = (bounds: unknown) => {
      const [x, y, width, height] = String(bounds).split(',').map(Number);
      return [Math.abs(x) <= 1 ? 0 : x, y, width, height];
    };
    expect(
      added.map(i => [i.level, i.body, place(i.properties.atlas_bounds)]),
    ).toEqual([
      [1, 'plain note', [0, 0, 0, 40]],
      [
        1,
        `a new card\n**second** line\nsee [[${target.title}]]`,
        [0, 120, 0, 40],
      ],
    ]);
    expect(edited.map(i => i.body)).toEqual([
      'plain note more',
      `a new card\n**second** line\nsee [[${target.title}]]`,
    ]);
  });
});
