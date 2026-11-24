import { expect, type Page } from '@playwright/test';
import { Server, openPage, test } from './support';

/** The requests that are not GET, as "METHOD path", in the order the app sends them. */
function writesOf(page: Page): string[] {
  const writes: string[] = [];
  page.on('request', r => {
    if (r.method() !== 'GET')
      writes.push(`${r.method()} ${new URL(r.url()).pathname}`);
  });
  return writes;
}

test.describe('deleting and demoting a page', () => {
  test('Delete page deletes the shown page with one request and opens today', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('delete-page');
    await openPage(page, p.title);
    const writes = writesOf(page);
    page.on('dialog', d => d.accept());

    await page.locator('.page-menu-btn').click();
    await page.getByRole('button', { name: 'Delete page' }).click();

    await expect(page).toHaveURL(/\/today$/);
    expect(await server.hasPage(p.id)).toBe(false);
    // The page is not looked up first: the deletion is the first request that is not a GET
    expect(writes[0]).toBe(`DELETE /api/pages/${p.id}`);
    expect(writes.filter(w => w.startsWith('DELETE'))).toHaveLength(1);
  });

  test('the demote button deletes the atlas and renames the index page with one request', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('demote-page');
    const atlas = await server.newAtlas(`${p.title}/`);
    await openPage(page, `${p.title}/index`);
    const writes = writesOf(page);
    page.on('dialog', d => d.accept());

    await page.locator('.demote-btn').click();

    await expect(page).toHaveURL(new RegExp(`/pages/${p.title}$`));
    expect(await server.hasPage(atlas.id)).toBe(false);
    expect(await server.hasPage(p.id)).toBe(true);
    expect(writes[0]).toBe(`POST /api/pages/${p.id}/demote`);
    expect(
      writes.filter(w => w.startsWith('DELETE') || w.startsWith('PUT')),
    ).toEqual([]);
  });
});
