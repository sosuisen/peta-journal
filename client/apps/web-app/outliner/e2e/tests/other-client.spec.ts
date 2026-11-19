import { expect } from '@playwright/test';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { Server, openEditor, openPage, shownBodies, test } from './support';

// A change by another client reaches the page through the server's events (ADR 015).
test.describe('a change by another client', () => {
  // ほかのクライアントが消した項目は、編集中は画面に残り、編集を終えると消える
  test('an item another client deletes while this page is edited goes away when the edit ends', async ({ page, request }) => {
    const server = new Server(request);
    const p = await server.newDocPage('other-client-delete');
    const [a, b] = await server.fill(p.id, ['a', 'b']);
    await openPage(page, p.title);
    await openEditor(page, a.id);

    await server.deleteItem(b.id);
    await server.itemsSettled(p.id, i => i.length === 1);
    // The page waits for the end of the edit: typing must not be overwritten
    await page.waitForTimeout(500);
    expect(await shownBodies(page)).toEqual(['a', 'b']);

    await page.evaluate(() => (document.activeElement as HTMLElement).blur());

    await expect.poll(() => shownBodies(page)).toEqual(['a']);
  });

  // ほかのクライアントが別の項目の文を変えると、編集を終えたあとに、その文が画面に出る
  test('a text another client changes in another item shows when the edit ends', async ({ page, request }) => {
    const server = new Server(request);
    const p = await server.newDocPage('other-client-change');
    const [a, b] = await server.fill(p.id, ['a', 'b']);
    await openPage(page, p.title);
    await openEditor(page, a.id);

    await server.updateBody(b.id, 'b changed');
    await page.waitForTimeout(500);
    await page.evaluate(() => (document.activeElement as HTMLElement).blur());

    await expect.poll(() => shownBodies(page)).toEqual(['a', 'b changed']);
  });

  // ほかの端末がファイルの別の項目の文を変えると、編集を終えたあとに、その文が画面に出る。Dropboxが書き換えたファイルは、この形で届く
  test('a text another device changes in the file shows when the edit ends', async ({ page, request }) => {
    const server = new Server(request);
    const p = await server.newDocPage('other-device-change');
    const [a] = await server.fill(p.id, ['a', 'b']);
    const file = join(await server.markdownDir(), 'docs', `${p.title}.md`);
    await expect.poll(() => existsSync(file) && readFileSync(file, 'utf8').includes('\nb\n'), { timeout: 10_000 })
      .toBe(true);
    await openPage(page, p.title);
    await openEditor(page, a.id);

    // Another device writes the time of its change with the item, as the merge takes the newer
    writeFileSync(file, readFileSync(file, 'utf8').replace(
      /modified="[^"]*" -->\nb\n/,
      `modified="${new Date().toISOString()}" -->\nb from the other device\n`,
    ));
    await server.itemsSettled(p.id, i => i[1]?.body === 'b from the other device');
    await page.evaluate(() => (document.activeElement as HTMLElement).blur());

    await expect.poll(() => shownBodies(page)).toEqual(['a', 'b from the other device']);
  });

  // ほかのクライアントが、編集中の項目の文を変えると、こちらで打っていなければ、編集を終えたあとに、その文が画面に出る
  test('a text another client changes in the edited item shows when the edit ends without typing', async ({ page, request }) => {
    const server = new Server(request);
    const p = await server.newDocPage('other-client-same');
    const [a] = await server.fill(p.id, ['a', 'b']);
    await openPage(page, p.title);
    await openEditor(page, a.id);

    await server.updateBody(a.id, 'a changed');
    await page.waitForTimeout(500);
    await page.evaluate(() => (document.activeElement as HTMLElement).blur());

    await expect.poll(() => shownBodies(page)).toEqual(['a changed', 'b']);
    // The edit that ended saves nothing over the change
    await page.waitForTimeout(1_000);
    expect((await server.items(p.id)).map(i => i.body)).toEqual(['a changed', 'b']);
  });
});
