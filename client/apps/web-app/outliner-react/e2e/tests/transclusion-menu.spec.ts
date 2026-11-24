import { Page, expect } from '@playwright/test';
import { serverPort } from '../playwright.config';
import { Server, openPage, outlineOf, shownBodies, test } from './support';

/** Opens the menu of an item and clicks one of its buttons. */
async function clickMenu(
  page: Page,
  itemId: string,
  name: string,
): Promise<void> {
  // The menu button shows only while the pointer is over the item
  await page
    .locator(`[id="${itemId}"] .item-menu`)
    .first()
    .click({ force: true });
  await page.getByRole('button', { name }).click();
}

// 項目のメニューから、トランスクルージョンを作り、外す
test.describe('the item menu and transclusions', () => {
  // Copy Referenceは項目のIDを書き、Paste as Transclusionはそれを読む
  test.use({ permissions: ['clipboard-read', 'clipboard-write'] });

  // Copy Referenceのあとに、別のページの項目でPaste as Transclusionを選ぶと、その項目が元の項目と子を表示する
  test('Paste as Transclusion after Copy Reference makes the item show the source item and its children', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const sourcePage = await server.newDocPage('menu-source');
    const source = await server.fill(sourcePage.id, ['top', '  child']);
    const hostPage = await server.newDocPage('menu-host');
    const host = await server.fill(hostPage.id, ['copy', 'below']);
    await openPage(page, sourcePage.title);
    await clickMenu(page, source[0].id, 'Copy Reference');
    await expect
      .poll(() => page.evaluate(() => navigator.clipboard.readText()))
      .toBe(source[0].id);
    await openPage(page, hostPage.title);

    await clickMenu(page, host[0].id, 'Paste as Transclusion');

    await expect
      .poll(() => shownBodies(page))
      .toEqual(['top', 'child', 'below']);
    // The server copies the source text into the item
    expect(
      outlineOf(
        await server.itemsSettled(hostPage.id, i => i[0].body === 'top'),
      ),
    ).toEqual(['1:top', '1:below']);
    expect(outlineOf(await server.items(sourcePage.id))).toEqual([
      '1:top',
      '2:child',
    ]);
  });

  // Remove Transclusionを選ぶと、項目は空になって元の項目を表示しなくなり、元のページは変わらない
  test('Remove Transclusion empties the item so it stops showing the source item, and leaves the source page as it was', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const sourcePage = await server.newDocPage('menu-remove-source');
    const source = await server.fill(sourcePage.id, ['top', '  child']);
    const hostPage = await server.newDocPage('menu-remove-host');
    const host = await server.fill(hostPage.id, ['copy', 'below']);
    const transclude = `http://localhost:${serverPort}/api/items/${host[0].id}/transclude`;
    expect(
      (
        await request.post(transclude, { data: { sourceItemId: source[0].id } })
      ).ok(),
    ).toBe(true);
    await openPage(page, hostPage.title);
    await expect
      .poll(() => shownBodies(page))
      .toEqual(['top', 'child', 'below']);

    await clickMenu(page, host[0].id, 'Remove Transclusion');

    expect(
      outlineOf(await server.itemsSettled(hostPage.id, i => i[0].body === '')),
    ).toEqual(['1:', '1:below']);
    await expect.poll(() => shownBodies(page)).toEqual(['', 'below']);
    await expect(
      page.getByRole('button', { name: 'Remove Transclusion' }),
    ).toHaveCount(0);
    expect(outlineOf(await server.items(sourcePage.id))).toEqual([
      '1:top',
      '2:child',
    ]);
  });

  // トランスクルージョンでない項目のメニューには、Remove Transclusionがない
  test('the menu of an item that is not a transclusion has no Remove Transclusion', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('menu-plain');
    const [plain] = await server.fill(p.id, ['plain']);
    await openPage(page, p.title);

    await page
      .locator(`[id="${plain.id}"] .item-menu`)
      .first()
      .click({ force: true });

    await expect(
      page.getByRole('button', { name: 'Paste as Transclusion' }),
    ).toBeVisible();
    await expect(
      page.getByRole('button', { name: 'Remove Transclusion' }),
    ).toHaveCount(0);
  });
});
