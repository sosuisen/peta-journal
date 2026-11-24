import { expect } from '@playwright/test';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  Server,
  editorOf,
  openEditor,
  openPage,
  shownBodies,
  test,
} from './support';

// A change by another client reaches the page through the server's events (ADR 015).
test.describe('a change by another client', () => {
  // ほかのクライアントが消した項目は、編集中でも、編集を終えずに画面から消える（ADR 069）
  test('an item another client deletes goes away while this page is edited', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('other-client-delete');
    const [a, b] = await server.fill(p.id, ['a', 'b']);
    await openPage(page, p.title);
    await openEditor(page, a.id);

    await server.deleteItem(b.id);

    await expect.poll(() => shownBodies(page)).toEqual(['a']);
    await expect(editorOf(page, a.id)).toBeFocused();
  });

  // ほかのクライアントが別の項目の文を変えると、編集中でも、その文が画面に出る
  test('a text another client changes in another item shows while editing', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('other-client-change');
    const [a, b] = await server.fill(p.id, ['a', 'b']);
    await openPage(page, p.title);
    await openEditor(page, a.id);

    await server.updateBody(b.id, 'b changed');

    await expect.poll(() => shownBodies(page)).toEqual(['a', 'b changed']);
    await expect(editorOf(page, a.id)).toBeFocused();
  });

  // ほかの端末がファイルの別の項目の文を変えると、編集中でも、その文が画面に出る。Dropboxが書き換えたファイルは、この形で届く
  test('a text another device changes in the file shows while editing', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('other-device-change');
    const [a] = await server.fill(p.id, ['a', 'b']);
    const file = join(await server.markdownDir(), 'docs', `${p.title}.md`);
    await expect
      .poll(
        () => existsSync(file) && readFileSync(file, 'utf8').includes('\nb\n'),
        { timeout: 10_000 },
      )
      .toBe(true);
    await openPage(page, p.title);
    await openEditor(page, a.id);

    // Another device writes the time of its change with the item, as the merge takes the newer
    writeFileSync(
      file,
      readFileSync(file, 'utf8').replace(
        /modified="[^"]*" -->\nb\n/,
        `modified="${new Date().toISOString()}" -->\nb from the other device\n`,
      ),
    );

    await expect
      .poll(() => shownBodies(page))
      .toEqual(['a', 'b from the other device']);
    await expect(editorOf(page, a.id)).toBeFocused();
  });

  // ほかのクライアントが、編集中の項目の文を変えると、こちらで打っていなければ、エディタにその文が出て、続けて打てる
  test('a text another client changes in the edited item shows in its editor when nothing was typed', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('other-client-same');
    const [a] = await server.fill(p.id, ['a', 'b']);
    await openPage(page, p.title);
    await openEditor(page, a.id);

    await server.updateBody(a.id, 'a changed');

    await expect(editorOf(page, a.id)).toHaveValue('a changed');
    await expect(editorOf(page, a.id)).toBeFocused();
    await page.keyboard.press('End');
    await page.keyboard.type('!');
    await server.itemsSettled(p.id, i => i[0]?.body === 'a changed!');
  });

  // ほかのクライアントが、編集中の項目の文を変えても、こちらで打っていれば、打った文が残り、保存で上書きする
  test('text typed into the edited item stays when another client changes the item', async ({
    page,
    request,
  }) => {
    const server = new Server(request);
    const p = await server.newDocPage('other-client-typed');
    const [a, b] = await server.fill(p.id, ['a', 'b']);
    await openPage(page, p.title);
    await openEditor(page, a.id);
    await page.keyboard.press('End');
    await page.keyboard.type(' typed');
    await server.itemsSettled(p.id, i => i[0]?.body === 'a typed');

    await server.updateBody(a.id, 'a changed');
    await server.updateBody(b.id, 'b changed');

    await expect
      .poll(() => shownBodies(page))
      .toEqual(['a typed', 'b changed']);
    await page.keyboard.type('!');
    await server.itemsSettled(p.id, i => i[0]?.body === 'a typed!');
  });
});
