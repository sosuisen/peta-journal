import { Page, expect } from '@playwright/test';
import { Server, test } from './support';
import { serverPort } from '../playwright.config';

const base = `http://localhost:${serverPort}`;

/**
 * An atlas with a page card, a link card and a plain card in a row through the middle of the
 * canvas, each 150 apart, and the dir path of its folder.
 */
async function atlasOfThree(
  server: Server,
  request: Parameters<Parameters<typeof test>[2]>[0]['request'],
): Promise<string> {
  const folder = await server.newDocPage('show-text');
  const dir = `${folder.title}/`;
  const atlas = await server.newAtlas(dir);
  const made = await request.post(`${base}/api/pages/doc`, {
    data: { title: 'sample', dirPath: dir },
  });
  expect(made.ok()).toBe(true);
  const items = await server.fill(atlas.id, [
    `[[${dir}sample]]`,
    '[[elsewhere/far|Far]]',
    'plain note',
  ]);
  const places = ['-150,0,0,40', '0,0,0,40', '150,0,0,40'];
  for (const [i, item] of items.entries()) {
    const resp = await request.put(
      `${base}/api/items/${item.id}/properties/atlas_bounds`,
      { data: { valueType: 'bounds', value: places[i] } },
    );
    expect(resp.ok()).toBe(true);
  }
  return dir;
}

/** The point of the screen where a point of the canvas of the atlas lies, at its first view */
async function onScreen(page: Page, x: number): Promise<[number, number]> {
  const box = await page.locator('.atlas-canvas canvas').boundingBox();
  if (box === null) throw new Error('no canvas');
  return [box.x + box.width / 2 + x, box.y + box.height / 2];
}

const text = (page: Page) => page.locator('.atlas-text');

// 「Show Text」は、ページカード、リンクカード、ノーマルカードのどれでも、項目の全文を右クリックした場所に出す。選んだクリックで閉じない。外を押すと閉じる。
test.describe('Show Text of the card menu', () => {
  test('shows the whole text of a page card, a link card and a plain card, and stays', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const dir = await atlasOfThree(server, request);
    await page.goto(`/pages/${dir}index_atlas`);
    await expect(page.locator('.atlas-canvas canvas')).toBeVisible();

    // A page card: the menu of the shell. The cards are drawn a moment after the canvas.
    const [px, py] = await onScreen(page, -150);
    await expect(async () => {
      await page.mouse.click(px, py, { button: 'right' });
      await expect(page.locator('.atlas-menu')).toBeVisible({ timeout: 500 });
    }).toPass();
    await page.getByRole('button', { name: 'Show Text' }).click();
    await expect(text(page)).toContainText('sample');
    await page.keyboard.press('Escape');
    await expect(text(page)).toHaveCount(0);

    // A link card and a plain card: the menu that the canvas draws, Show Text first
    for (const [x, shown] of [
      [0, 'Far'],
      [150, 'plain note'],
    ] as const) {
      const [cx, cy] = await onScreen(page, x);
      await page.mouse.click(cx, cy, { button: 'right' });
      await page.mouse.click(cx + 70, cy + 19);
      await expect(text(page)).toContainText(shown);
      // At the place of the right click, not of the press that chose it
      const box = await text(page).boundingBox();
      expect([box?.x, box?.y]).toEqual([cx, cy]);
      // Not closed by the click that chose it, nor a moment after
      await page.waitForTimeout(300);
      await expect(text(page)).toContainText(shown);
      // A press outside closes it
      await page.mouse.click(cx, cy + 200);
      await expect(text(page)).toHaveCount(0);
    }
  });
});
