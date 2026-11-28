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

// カードのEditは、項目の本文をMarkdownのまま入力に出し、OKで書いたものを本文にする。キャンバスのAdd Cardは、書いたものを本文に、右クリックした点にカードを作る。
test.describe('Edit and Add Card of the canvas', () => {
  test('Edit writes the Markdown into the body of the item, and Add Card makes an item at the point', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const folder = await server.newDocPage('edit-card');
    const dir = `${folder.title}/`;
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
    await page.keyboard.type('a new card');
    await page.keyboard.press('Enter');
    const added = await server.itemsSettled(atlas.id, all => all.length === 2);

    // Edit of the plain card, drawn by now: the cursor is at the end of its body
    const [cx, cy] = await onScreen(page, 0, 0);
    await firstOfMenu(page, cx, cy);
    await page.keyboard.type(' more');
    await page.keyboard.press('Enter');
    const edited = await server.itemsSettled(
      atlas.id,
      all => all[0]?.body !== 'plain note',
    );

    expect(
      added.map(i => [i.level, i.body, i.properties.atlas_bounds]),
    ).toEqual([
      [1, 'plain note', '0,0,0,40'],
      [1, 'a new card', '0,120,0,40'],
    ]);
    expect(edited.map(i => i.body)).toEqual(['plain note more', 'a new card']);
  });
});
