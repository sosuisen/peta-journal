import { Page, expect, test } from '@playwright/test';
import { resolve } from 'node:path';

// atlas.js alone in Chromium, on an empty page: what a press does before PixiJS has drawn the
// next frame. The page of the app is not needed, so the canvas is made here as a site makes it.

const web = (name: string) =>
  resolve(__dirname, '../../../../../../atlas/web', name);

/** A plain card of the text `label`, its center at the middle of the canvas */
const plainCard = (id: string, label: string) => ({
  id,
  label,
  path: null,
  isDir: false,
  isLink: true,
  isPlain: true,
  tone: 'plain',
  runs: [{ text: label, link: null }],
  x: 0,
  y: 0,
  width: 0,
  height: 40,
  scale: 1,
  group: null,
  groups: [],
  icon: null,
  blob: null,
  iconSize: 0,
  iconDrawSize: 0,
  masked: false,
});

/** An editable canvas over the whole page, with one card drawn, that keeps what it asks */
async function canvasWithCard(page: Page): Promise<void> {
  await page.route('**/atlas-alone', route =>
    route.fulfill({
      contentType: 'text/html',
      body: '<!doctype html><body style="margin:0"><div id="host" style="width:600px;height:400px"></div></body>',
    }),
  );
  await page.goto('/atlas-alone');
  await page.addScriptTag({ path: web('pixi.min.js') });
  await page.addScriptTag({ path: web('atlas.js') });
  await page.evaluate(
    async card => {
      const w = window as unknown as Record<string, any>;
      const host = document.getElementById('host') as HTMLElement;
      const canvas = new w.AtlasCanvas(
        host,
        () => {},
        () => {},
        () => {},
        () => {},
        0,
        0,
        1,
        {
          // No font to wait for: the titles take the font of the page
          titleFont: { family: 'none', url: 'data:,' },
        },
      );
      w.asked = [];
      canvas.setOnText((asked: { name: string }) => w.asked.push(asked.name));
      canvas.setOnCard(
        () => {},
        () => {},
        () => {},
        () => 'note',
      );
      await canvas.ready;
      canvas.setCards(JSON.stringify([card]));
      w.canvas = canvas;
      w.testCard = card;
    },
    plainCard('c', 'note'),
  );
  // The first frames are drawn, and the font of the titles has failed
  await page.waitForTimeout(300);
}

// メニューは、開いたすぐ後の、まだ描いていない間の押下も受ける
test('a menu takes a click at once, before the next frame is drawn', async ({
  page,
}) => {
  await canvasWithCard(page);

  const asked = await page.evaluate(() => {
    const w = window as unknown as Record<string, any>;
    const element = w.canvas.app.canvas as HTMLCanvasElement;
    const box = element.getBoundingClientRect();
    const [x, y] = [box.left + box.width / 2, box.top + box.height / 2];
    const press = (type: string, px: number, py: number, button: number) =>
      element.dispatchEvent(
        new PointerEvent(type, {
          clientX: px,
          clientY: py,
          button,
          buttons: type === 'pointerdown' ? (button === 2 ? 2 : 1) : 0,
          pointerId: 1,
          pointerType: 'mouse',
          isPrimary: true,
          bubbles: true,
          cancelable: true,
        }),
      );
    // The right button on the card opens its menu, whose first item is Edit; in the same task,
    // the left button picks it
    press('pointerdown', x, y, 2);
    press('pointerup', x, y, 2);
    press('pointerdown', x + 70, y + 19, 0);
    press('pointerup', x + 70, y + 19, 0);
    return w.asked;
  });

  expect(asked).toEqual(['Edit']);
});

// カードを描き直したすぐ後の、まだ描いていない間の右クリックも、そのカードのメニューを開く
test('a card drawn again takes a right click at once, before the next frame is drawn', async ({
  page,
}) => {
  await canvasWithCard(page);

  const asked = await page.evaluate(() => {
    const w = window as unknown as Record<string, any>;
    const element = w.canvas.app.canvas as HTMLCanvasElement;
    const box = element.getBoundingClientRect();
    const [x, y] = [box.left + box.width / 2, box.top + box.height / 2];
    const press = (type: string, px: number, py: number, button: number) =>
      element.dispatchEvent(
        new PointerEvent(type, {
          clientX: px,
          clientY: py,
          button,
          buttons: type === 'pointerdown' ? (button === 2 ? 2 : 1) : 0,
          pointerId: 1,
          pointerType: 'mouse',
          isPrimary: true,
          bubbles: true,
          cancelable: true,
        }),
      );
    // A new list of cards comes, as after Add Card, and draws every card anew
    w.canvas.setCards(JSON.stringify([w.testCard]));
    press('pointerdown', x, y, 2);
    press('pointerup', x, y, 2);
    // The menu is drawn now; its first item is Edit for a card, Add Card for the canvas
    return new Promise(resolve =>
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          press('pointerdown', x + 70, y + 19, 0);
          press('pointerup', x + 70, y + 19, 0);
          resolve(w.asked);
        }),
      ),
    );
  });

  expect(asked).toEqual(['Edit']);
});
