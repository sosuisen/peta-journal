import { expect } from '@playwright/test';
import { templatePagePrefix, templateTag } from '../playwright.config';
import { Server, openPage, outlineOf, shownBodies, test } from './support';

// サーバーで動く拡張関数（auto-template）が作った項目が、画面に出る（ADR 009）
test.describe('items an extension function makes', () => {
  // 新しいページを開くと、テンプレートの項目が最初から出て、空の項目は残らない
  test('a new page shows the items of its template from the start, with no empty item left', async ({
    page,
  }) => {
    const title = `${templatePagePrefix}${Date.now()}`;

    await openPage(page, title);

    await expect.poll(() => shownBodies(page)).toEqual(['from page template']);
  });

  // ページにタグを足して確認にOKと答えると、タグのテンプレートの項目が空の項目の代わりに出る
  test('a tag added to a page shows the items of its template in place of the empty item once the user agrees', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('tagged');
    await openPage(page, p.title);
    await page.locator('.page-menu-btn').click();
    await page.getByRole('button', { name: 'Edit properties' }).click();
    await page.locator('.property-add').click();
    await page.locator('.property-key-select').selectOption('tags');

    // The question of an extension is the browser's own confirm, not the app's dialog
    const asked: string[] = [];
    page.on('dialog', async dialog => {
      asked.push(dialog.message());
      await dialog.accept();
    });

    await page.locator('.property-tag-input').fill(templateTag);
    await page.locator('.property-tag-input').press('Enter');

    await expect.poll(() => shownBodies(page)).toEqual(['from tag template']);
    expect(outlineOf(await server.items(p.id))).toEqual([
      '1:from tag template',
    ]);
    expect(asked).toEqual([
      `Insert the template of the tag "#${templateTag}" at the end of this page?`,
    ]);
  });
});
