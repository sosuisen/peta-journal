import { expect, type APIRequestContext, type Page } from '@playwright/test';
import { serverPort } from '../playwright.config';
import { Server, openPage, test } from './support';

const PNG_1X1 = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==';
const base = `http://localhost:${serverPort}`;

/** Puts a small PNG into the assets folder and returns the name the server gave it. */
async function uploadImage(request: APIRequestContext): Promise<string> {
  const resp = await request.post(`${base}/api/assets/upload`, {
    multipart: { file: { name: 'pic.png', mimeType: 'image/png', buffer: Buffer.from(PNG_1X1, 'base64') } },
  });
  expect(resp.ok()).toBe(true);
  const { fileName } = await resp.json();
  return fileName;
}

/** Creates a doc page in `dirPath` and returns its id and the name its URL holds. */
async function newPageIn(request: APIRequestContext, dirPath: string): Promise<{ id: string; name: string }> {
  const title = `asset-image-${Date.now()}`;
  const resp = await request.post(`${base}/api/pages/doc`, { data: { title, dirPath } });
  expect(resp.ok()).toBe(true);
  const { page } = await resp.json();
  return { id: page.id, name: `${dirPath}${title}` };
}

/** The width the browser loaded for the first image of the page: 0 when it did not load. */
async function loadedWidth(page: Page): Promise<number> {
  const image = page.locator('app-outline-item img').first();
  await image.waitFor();
  return image.evaluate(el => (el as HTMLImageElement).complete ? (el as HTMLImageElement).naturalWidth : -1);
}

test.describe('an image of the assets folder', () => {
  for (const dirPath of ['', 'folder/']) {
    test(`shows on a page in "${dirPath}"`, async ({ page, request }) => {
      const server = new Server(request);
      const file = await uploadImage(request);
      const p = await newPageIn(request, dirPath);
      await server.fill(p.id, [`![pic](../assets/${file})`]);

      await openPage(page, p.name);

      await expect.poll(() => loadedWidth(page)).toBe(1);
    });
  }
});
