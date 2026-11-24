import { expect } from '@playwright/test';
import {
  Server,
  editorOf,
  openEditor,
  openPage,
  outlineOf,
  test,
} from './support';

// A selected item always comes with its descendants, so deleting it deletes them too.
test.describe('deleting a selected parent', () => {
  test('a selection started in the editor of a parent takes its children, and Delete removes them', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('parent-from-editor');
    const items = await server.fill(p.id, ['a', '  a1', 'b']);
    await openPage(page, p.title);
    await openEditor(page, items[0].id);

    await page.keyboard.press('Shift+ArrowDown'); // leaves the editor: a and a1 are selected
    await expect(
      page.locator('app-outline-item .outline-item.selected'),
    ).toHaveCount(2);
    await page.keyboard.press('Delete');

    expect(
      outlineOf(await server.itemsSettled(p.id, i => i.length === 1)),
    ).toEqual(['1:b']);
  });

  test('undo of the deletion of a parent and its child brings both back on the server, with their ids', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('undo-parent-delete');
    const items = await server.fill(p.id, ['a', '  a1', 'b']);
    await openPage(page, p.title);
    await openEditor(page, items[0].id);
    await page.keyboard.press('Shift+ArrowDown'); // leaves the editor: a and a1 are selected
    await expect(
      page.locator('app-outline-item .outline-item.selected'),
    ).toHaveCount(2);
    await page.keyboard.press('Delete');
    await server.itemsSettled(p.id, i => i.length === 1);

    await openEditor(page, items[2].id);
    await page.keyboard.press('Control+z');

    const back = await server.itemsSettled(p.id, i => i.length === 3);
    expect(outlineOf(back)).toEqual(['1:a', '2:a1', '1:b']);
    expect(back.map(i => i.id)).toEqual(items.map(i => i.id));
  });

  test('Delete of every item writes with one request, and the page shows the empty item the server made', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('delete-everything');
    const items = await server.fill(p.id, ['a', '  a1']);
    await openPage(page, p.title);
    await openEditor(page, items[0].id);
    await page.keyboard.press('Shift+ArrowDown'); // leaves the editor: a and a1 are selected
    await expect(
      page.locator('app-outline-item .outline-item.selected'),
    ).toHaveCount(2);
    const posts: unknown[] = [];
    page.on('request', r => {
      if (r.method() !== 'GET')
        posts.push([
          `${r.method()} ${new URL(r.url()).pathname}`,
          r.postDataJSON(),
        ]);
    });

    await page.keyboard.press('Delete');

    const left = await server.itemsSettled(
      p.id,
      i => i.length === 1 && i[0].body === '',
    );
    await expect(page.locator(`[id="${left[0].id}"]`)).toHaveCount(1);
    await expect(page.locator('app-outline-item')).toHaveCount(1);
    // The first request is the question whether another item transcludes these; it writes nothing
    const itemIds = items.map(i => i.id);
    expect(posts).toEqual([
      ['POST /api/items/transcluded', { itemIds }],
      ['POST /api/items/delete', { itemIds }],
    ]);
  });

  test('text typed right after Delete of every item goes into the empty item the server made', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('type-after-delete-everything');
    const items = await server.fill(p.id, ['a', '  a1']);
    await openPage(page, p.title);
    await openEditor(page, items[0].id);
    await page.keyboard.press('Shift+ArrowDown'); // leaves the editor: a and a1 are selected
    await expect(
      page.locator('app-outline-item .outline-item.selected'),
    ).toHaveCount(2);
    await page.keyboard.press('Delete');
    const [left] = await server.itemsSettled(
      p.id,
      i => i.length === 1 && i[0].body === '',
    );
    await expect(editorOf(page, left.id)).toBeFocused();

    await page.keyboard.type('new');

    expect(
      outlineOf(await server.itemsSettled(p.id, i => i[0]?.body === 'new')),
    ).toEqual(['1:new']);
  });

  test('Ctrl+Z right after Delete, with no editor and no selection, brings the items back, and Ctrl+Shift+Z takes them again', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('undo-right-after-delete');
    const items = await server.fill(p.id, ['a', '  a1', 'b']);
    await openPage(page, p.title);
    await openEditor(page, items[0].id);
    await page.keyboard.press('Shift+ArrowDown'); // leaves the editor: a and a1 are selected
    await expect(
      page.locator('app-outline-item .outline-item.selected'),
    ).toHaveCount(2);
    await page.keyboard.press('Delete');
    await server.itemsSettled(p.id, i => i.length === 1);
    await expect(page.locator('app-outline-item')).toHaveCount(1);

    await page.keyboard.press('Control+z');

    const back = await server.itemsSettled(p.id, i => i.length === 3);
    expect(outlineOf(back)).toEqual(['1:a', '2:a1', '1:b']);
    await expect(page.locator('app-outline-item')).toHaveCount(3);

    await page.keyboard.press('Control+Shift+z');

    expect(
      outlineOf(await server.itemsSettled(p.id, i => i.length === 1)),
    ).toEqual(['1:b']);
  });

  test('Delete removes a collapsed parent with its hidden children', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('collapsed-parent');
    const items = await server.fill(p.id, ['x', 'a', '  a1', 'b']);
    await openPage(page, p.title);
    await openEditor(page, items[1].id);
    await page.keyboard.press('Control+ArrowUp');
    await expect(page.locator('app-outline-item')).toHaveCount(3);

    await openEditor(page, items[0].id);
    await page.keyboard.press('Shift+ArrowDown'); // leaves the editor: x is selected
    await page.keyboard.press('Shift+ArrowDown'); // a joins, with its hidden child
    await expect(
      page.locator('app-outline-item .outline-item.selected'),
    ).toHaveCount(2);
    await page.keyboard.press('Delete');

    expect(
      outlineOf(await server.itemsSettled(p.id, i => i.length === 1)),
    ).toEqual(['1:b']);
  });

  test('undo of the deletion of a collapsed task brings it back collapsed and a task, on the server and on the screen', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('undo-collapsed-task');
    const items = await server.fill(p.id, ['a', '  a1', 'b']);
    await openPage(page, p.title);
    await openEditor(page, items[0].id);
    await page.keyboard.press('Control+Enter'); // a becomes a task
    await page.keyboard.press('Control+ArrowUp');
    await server.itemsSettled(
      p.id,
      i => i[0].collapsed && i[0].types.includes('task'),
    );
    await expect(page.locator('app-outline-item')).toHaveCount(2);
    await page.keyboard.press('Shift+ArrowDown'); // leaves the editor: a is selected, with its hidden child
    await expect(
      page.locator('app-outline-item .outline-item.selected'),
    ).toHaveCount(1);
    await page.keyboard.press('Delete');
    await server.itemsSettled(p.id, i => i.length === 1);

    await openEditor(page, items[2].id);
    await page.keyboard.press('Control+z');

    const back = await server.itemsSettled(p.id, i => i.length === 3);
    expect(outlineOf(back)).toEqual(['1:a', '2:a1', '1:b']);
    expect(back[0].collapsed).toBe(true);
    expect(back[0].types).toContain('task');
    await expect(page.locator('app-outline-item')).toHaveCount(2);
  });
});
