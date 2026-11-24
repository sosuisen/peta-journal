import { expect } from '@playwright/test';
import {
  Server,
  StoredItem,
  editorOf,
  openEditor,
  openPage,
  outlineOf,
  shownBodies,
  test,
} from './support';

test.describe('outline structure', () => {
  let server: Server;
  let pageId: string;
  let items: StoredItem[];
  const byBody = (body: string) => items.find(i => i.body === body)!;

  // * a
  //     * a1
  //     * a2
  // * b
  //     * b1
  test.beforeEach(async ({ page, request }) => {
    server = new Server(request);
    const doc = await server.newDocPage('structure');
    pageId = doc.id;
    items = await server.fill(pageId, ['a', '  a1', '  a2', 'b', '  b1']);
    await openPage(page, doc.title);
  });

  test('Tab indents the item with its children, Shift+Tab outdents it again', async ({
    page,
  }) => {
    await openEditor(page, byBody('b').id);

    await page.keyboard.press('Tab');
    expect(
      outlineOf(await server.itemsSettled(pageId, i => i[3].level === 2)),
    ).toEqual(['1:a', '2:a1', '2:a2', '2:b', '3:b1']);

    await page.keyboard.press('Shift+Tab');
    expect(
      outlineOf(await server.itemsSettled(pageId, i => i[3].level === 1)),
    ).toEqual(['1:a', '2:a1', '2:a2', '1:b', '2:b1']);
  });

  test('the first item cannot be indented and a top-level item cannot be outdented', async ({
    page,
  }) => {
    await openEditor(page, byBody('a').id);
    await page.keyboard.press('Tab');
    await page.keyboard.press('Shift+Tab');
    await page.waitForTimeout(500);

    expect(outlineOf(await server.items(pageId))).toEqual([
      '1:a',
      '2:a1',
      '2:a2',
      '1:b',
      '2:b1',
    ]);
  });

  test('Alt+Shift+Down moves the item with its children below the next sibling', async ({
    page,
  }) => {
    await openEditor(page, byBody('a').id);

    await page.keyboard.press('Alt+Shift+ArrowDown');

    expect(
      outlineOf(await server.itemsSettled(pageId, i => i[0].body === 'b')),
    ).toEqual(['1:b', '2:b1', '1:a', '2:a1', '2:a2']);
    await expect(editorOf(page, byBody('a').id)).toBeFocused();
  });

  test('Ctrl+Z takes back a move and Ctrl+Shift+Z does it again', async ({
    page,
  }) => {
    await openEditor(page, byBody('a').id);
    await page.keyboard.press('Alt+Shift+ArrowDown');
    await server.itemsSettled(pageId, i => i[0].body === 'b');
    // The server knows the move before the screen does, and only then is there a move to take back
    await expect
      .poll(() => shownBodies(page))
      .toEqual(['b', 'b1', 'a', 'a1', 'a2']);

    await page.keyboard.press('Control+z');

    expect(
      outlineOf(await server.itemsSettled(pageId, i => i[0].body === 'a')),
    ).toEqual(['1:a', '2:a1', '2:a2', '1:b', '2:b1']);
    await expect
      .poll(() => shownBodies(page))
      .toEqual(['a', 'a1', 'a2', 'b', 'b1']);
    await expect(editorOf(page, byBody('a').id)).toBeFocused();

    await page.keyboard.press('Control+Shift+z');

    expect(
      outlineOf(await server.itemsSettled(pageId, i => i[0].body === 'b')),
    ).toEqual(['1:b', '2:b1', '1:a', '2:a1', '2:a2']);
    await expect
      .poll(() => shownBodies(page))
      .toEqual(['b', 'b1', 'a', 'a1', 'a2']);
  });

  test('Alt+Shift+Up moves a child above its sibling', async ({ page }) => {
    await openEditor(page, byBody('a2').id);

    await page.keyboard.press('Alt+Shift+ArrowUp');

    expect(
      outlineOf(await server.itemsSettled(pageId, i => i[1].body === 'a2')),
    ).toEqual(['1:a', '2:a2', '2:a1', '1:b', '2:b1']);
  });

  test('a move writes with one request, which carries the text typed and not saved yet', async ({
    page,
  }) => {
    await openEditor(page, byBody('a').id);
    await page.keyboard.press('End');
    const writes: unknown[] = [];
    page.on('request', r => {
      if (r.method() !== 'GET')
        writes.push([
          `${r.method()} ${new URL(r.url()).pathname}`,
          r.postDataJSON(),
        ]);
    });

    // No pause between the keys: the auto-save has not sent the text yet when the item moves
    await page.keyboard.type('X');
    await page.keyboard.press('Alt+Shift+ArrowDown');

    expect(
      outlineOf(await server.itemsSettled(pageId, i => i[0].body === 'b')),
    ).toEqual(['1:b', '2:b1', '1:aX', '2:a1', '2:a2']);
    await expect
      .poll(() => shownBodies(page))
      .toEqual(['b', 'b1', 'aX', 'a1', 'a2']);
    await expect(editorOf(page, byBody('a').id)).toBeFocused();
    // Longer than the auto-save waits: the typed text must not be saved a second time
    await page.waitForTimeout(600);
    expect(writes).toEqual([
      [
        'POST /api/items/move',
        {
          itemIds: [byBody('a').id, byBody('a1').id, byBody('a2').id],
          targetPageId: pageId,
          position: `after:${byBody('b1').id}`,
          body: 'aX',
        },
      ],
    ]);
  });

  test('text typed right after a move goes into the moved item', async ({
    page,
  }) => {
    await openEditor(page, byBody('a').id);
    await page.keyboard.press('End');

    // No pause between the keys: the answer of the server is not there yet when the text arrives
    await page.keyboard.press('Alt+Shift+ArrowDown');
    await page.keyboard.type('XY');

    expect(
      outlineOf(
        await server.itemsSettled(
          pageId,
          i => i[0].body === 'b' && i[2].body === 'aXY',
        ),
      ),
    ).toEqual(['1:b', '2:b1', '1:aXY', '2:a1', '2:a2']);
    await expect
      .poll(() => shownBodies(page))
      .toEqual(['b', 'b1', 'aXY', 'a1', 'a2']);
  });

  test('two moves pressed without a pause both happen, in their order', async ({
    page,
  }) => {
    await openEditor(page, byBody('a2').id);
    let moved = 0;
    page.on('response', r => {
      if (
        r.request().method() === 'POST' &&
        new URL(r.url()).pathname === '/api/items/move'
      )
        moved++;
    });

    await page.keyboard.press('Alt+Shift+ArrowUp');
    await page.keyboard.press('Alt+Shift+ArrowDown');
    await page.keyboard.press('Alt+Shift+ArrowUp');

    // The outline after the first move is the same as after the third: waiting for that
    // outline alone compared the outline while the second move was being written
    await expect.poll(() => moved).toBe(3);
    expect(outlineOf(await server.items(pageId))).toEqual([
      '1:a',
      '2:a2',
      '2:a1',
      '1:b',
      '2:b1',
    ]);
    await expect
      .poll(() => shownBodies(page))
      .toEqual(['a', 'a2', 'a1', 'b', 'b1']);
    await expect(editorOf(page, byBody('a2').id)).toBeFocused();
    // Nothing more is on its way
    await page.waitForTimeout(600);
    expect(outlineOf(await server.items(pageId))).toEqual([
      '1:a',
      '2:a2',
      '2:a1',
      '1:b',
      '2:b1',
    ]);
  });

  // Command is the key on the Mac, Control is the key elsewhere: both are read everywhere
  for (const modifier of ['Control', 'Meta']) {
    test(`${modifier}+ArrowUp only hides the children and ${modifier}+ArrowDown only shows them`, async ({
      page,
    }) => {
      await openEditor(page, byBody('a').id);
      const shown = page.locator('app-outline-item');

      await page.keyboard.press(`${modifier}+ArrowDown`); // already shown: nothing happens
      await page.keyboard.press(`${modifier}+ArrowUp`);
      await expect(shown).toHaveCount(3);
      expect(
        (await server.itemsSettled(pageId, i => i[0].collapsed))[0].collapsed,
      ).toBe(true);

      await page.keyboard.press(`${modifier}+ArrowUp`); // already hidden: nothing happens
      await page.waitForTimeout(300);
      await expect(shown).toHaveCount(3);
      expect((await server.items(pageId))[0].collapsed).toBe(true);

      await page.keyboard.press(`${modifier}+ArrowDown`);
      await expect(shown).toHaveCount(5);
      expect(
        (await server.itemsSettled(pageId, i => !i[0].collapsed))[0].collapsed,
      ).toBe(false);
    });
  }

  test('Shift+Down selects items and Delete removes them', async ({ page }) => {
    await openEditor(page, byBody('a1').id);

    await page.keyboard.press('Shift+ArrowDown'); // leaves the editor: a1 is selected
    await page.keyboard.press('Shift+ArrowDown'); // a2
    await expect(
      page.locator('app-outline-item .outline-item.selected'),
    ).toHaveCount(2);

    await page.keyboard.press('Delete');

    expect(
      outlineOf(await server.itemsSettled(pageId, i => i.length === 3)),
    ).toEqual(['1:a', '1:b', '2:b1']);
  });

  test.describe('copying the selection', () => {
    // The clipboard is written through the asynchronous clipboard API, which asks for both
    test.use({ permissions: ['clipboard-read', 'clipboard-write'] });

    test('Ctrl+C puts the selection on the clipboard and a paste brings it back with its levels', async ({
      page,
    }) => {
      await openEditor(page, byBody('a1').id);
      await page.keyboard.press('Shift+ArrowDown'); // a1
      await page.keyboard.press('Shift+ArrowDown'); // a2
      await expect(
        page.locator('app-outline-item .outline-item.selected'),
      ).toHaveCount(2);

      await page.keyboard.press('Control+c');

      // The write finishes only once the server has made the tree markdown. Windows hands the
      // text back with \r\n, though the app writes \n
      await expect
        .poll(() =>
          page.evaluate(async () =>
            (await navigator.clipboard.readText()).replace(/\r\n/g, '\n'),
          ),
        )
        .toBe('- a1\n- a2');

      await page.keyboard.press('Escape');
      await openEditor(page, byBody('b1').id);
      await page.keyboard.press('End');
      await page.keyboard.press('Enter'); // an empty item below b1
      // The new item is shown once the server has answered: a paste sent before that goes into b1
      await expect(page.locator('textarea:focus')).toHaveValue('');
      await page.evaluate(async () => {
        const data = new DataTransfer();
        data.setData('text/plain', await navigator.clipboard.readText());
        document.activeElement!.dispatchEvent(
          new ClipboardEvent('paste', {
            clipboardData: data,
            bubbles: true,
            cancelable: true,
          }),
        );
      });

      expect(
        outlineOf(await server.itemsSettled(pageId, i => i.length === 7)),
      ).toEqual(['1:a', '2:a1', '2:a2', '1:b', '2:b1', '2:a1', '2:a2']);
    });
  });

  test('a selection started at a parent takes its whole subtree, and grows past it', async ({
    page,
  }) => {
    await openEditor(page, byBody('a').id);

    await page.keyboard.press('Shift+ArrowDown'); // a, with a1 and a2
    await expect(
      page.locator('app-outline-item .outline-item.selected'),
    ).toHaveCount(3);

    await page.keyboard.press('Shift+ArrowDown'); // b, with b1
    await expect(
      page.locator('app-outline-item .outline-item.selected'),
    ).toHaveCount(5);
  });

  test('Escape ends a selection without changing anything', async ({
    page,
  }) => {
    await openEditor(page, byBody('b').id);
    await page.keyboard.press('Shift+ArrowDown'); // b, with b1
    await expect(
      page.locator('app-outline-item .outline-item.selected'),
    ).toHaveCount(2);

    await page.keyboard.press('Escape');

    await expect(
      page.locator('app-outline-item .outline-item.selected'),
    ).toHaveCount(0);
    expect(outlineOf(await server.items(pageId))).toEqual([
      '1:a',
      '2:a1',
      '2:a2',
      '1:b',
      '2:b1',
    ]);
  });

  test('undo of Ctrl+Enter reaches the server, so a reload keeps the undone state', async ({
    page,
  }) => {
    const b1 = byBody('b1').id;
    await openEditor(page, b1);
    await page.keyboard.press('Control+Enter');
    await server.itemsSettled(pageId, i => i[4].types.includes('task'));
    // The app records the cycle for undo when the answer is there: wait for the screen
    await expect(page.locator(`[id="${b1}"] .task-checkbox`)).toBeVisible();

    await page.keyboard.press('Control+z');

    await server.itemsSettled(pageId, i => !i[4].types.includes('task'));
    await page.reload();
    await page.locator('app-outline-item').first().waitFor();
    await expect(page.locator(`[id="${b1}"] .task-checkbox`)).toHaveCount(0);
  });

  test('undo of the cycle from done to normal puts the task back with its times', async ({
    page,
  }) => {
    const b1 = byBody('b1').id;
    const checkbox = page.locator(`[id="${b1}"] .task-checkbox`);
    await openEditor(page, b1);
    // Each key waits for the screen, not only for the server: the app records a cycle for undo
    // when the answer is there, so a key pressed before that would act on the state before it
    await page.keyboard.press('Control+Enter');
    await server.itemsSettled(pageId, i => i[4].types.includes('task'));
    await expect(checkbox).toHaveJSProperty('indeterminate', false);
    await page.keyboard.press('Control+Enter');
    await server.itemsSettled(pageId, i => 'begin' in i[4].properties);
    await expect(checkbox).toHaveJSProperty('indeterminate', true);
    await page.keyboard.press('Control+Enter');
    const [, , , , done] = await server.itemsSettled(
      pageId,
      i => 'end' in i[4].properties,
    );
    await expect(checkbox).toBeChecked();
    await page.keyboard.press('Control+Enter');
    await server.itemsSettled(pageId, i => !i[4].types.includes('task'));
    await expect(checkbox).toHaveCount(0);

    await page.keyboard.press('Control+z');

    const [, , , , undone] = await server.itemsSettled(pageId, i =>
      i[4].types.includes('task'),
    );
    expect(undone.properties).toMatchObject({
      begin: done.properties.begin,
      end: done.properties.end,
    });
  });

  test('Ctrl+Enter turns the item into a task and shows its checkbox', async ({
    page,
  }) => {
    await openEditor(page, byBody('b1').id);

    await page.keyboard.press('Control+Enter');

    const [, , , , b1] = await server.itemsSettled(pageId, i =>
      i[4].types.includes('task'),
    );
    expect(b1.types).toContain('task');
    await expect(page.locator(`[id="${b1.id}"] .task-checkbox`)).toBeVisible();
  });

  test('Ctrl+Z takes back an indent and Ctrl+Shift+Z does it again', async ({
    page,
  }) => {
    await openEditor(page, byBody('b').id);
    await page.keyboard.press('Tab');
    await server.itemsSettled(pageId, i => i[3].level === 2);

    await page.keyboard.press('Control+z');
    expect(
      outlineOf(
        await server.itemsSettled(
          pageId,
          i => i[3].level === 1 && i[4].level === 2,
        ),
      ),
    ).toEqual(['1:a', '2:a1', '2:a2', '1:b', '2:b1']);

    await page.keyboard.press('Control+Shift+z');
    expect(
      outlineOf(
        await server.itemsSettled(
          pageId,
          i => i[3].level === 2 && i[4].level === 3,
        ),
      ),
    ).toEqual(['1:a', '2:a1', '2:a2', '2:b', '3:b1']);
  });

  test('Send to Stash writes with one request and puts the item with its children under a new # Stash heading', async ({
    page,
  }) => {
    const writes: unknown[] = [];
    page.on('request', r => {
      if (r.method() !== 'GET')
        writes.push([
          `${r.method()} ${new URL(r.url()).pathname}`,
          r.postDataJSON(),
        ]);
    });

    await page
      .locator(`[id="${byBody('a').id}"] .item-menu`)
      .first()
      .click({ force: true });
    await page.getByRole('button', { name: 'Send to Stash' }).click();

    expect(
      outlineOf(
        await server.itemsSettled(pageId, i =>
          i.some(x => x.body === '# Stash'),
        ),
      ),
    ).toEqual(['1:b', '2:b1', '1:# Stash', '2:a', '3:a1', '3:a2']);
    // The heading is shown as text, without its mark
    await expect
      .poll(() => shownBodies(page))
      .toEqual(['b', 'b1', 'Stash', 'a', 'a1', 'a2']);
    expect(writes).toEqual([[`POST /api/items/${byBody('a').id}/stash`, {}]]);
  });
});
