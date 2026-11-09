import { test } from '@playwright/test';
import { Server, openEditor, openPage } from './support';

for (let n = 0; n < 12; n++) test('probe enter then type ' + n, async ({ page, request }) => {
  const server = new Server(request);
  const doc = await server.newDocPage('probe');
  const [first] = await server.items(doc.id);
  const log: string[] = [];
  const t0 = Date.now();
  page.on('request', r => { if (r.method() !== 'GET') log.push(`${Date.now() - t0} REQ ${r.method()} ${r.url().slice(21).replace(/2026-[0-9-]*/g, '')} ${(r.postData() ?? '').slice(0, 120)}`); });
  page.on('requestfinished', r => { if (r.method() !== 'GET') log.push(`${Date.now() - t0} FIN ${r.url().slice(21).replace(/2026-[0-9-]*/g, '')}`); });
  await openPage(page, doc.title);
  await openEditor(page, first.id);
  await page.keyboard.type('first line');
  await page.keyboard.press('Enter');
  await page.keyboard.type('second line');
  await page.waitForTimeout(1200);
  const items = await server.items(doc.id);
  const ok = items.length === 2 && items[1].body === 'second line' && items[0].body === 'first line';
  console.log(ok ? 'OK' : 'BAD', JSON.stringify(items.map(i => i.body)), 'shown:', JSON.stringify(await page.locator('textarea').evaluateAll(els => els.map(e => (e as HTMLTextAreaElement).value))));
  if (!ok) console.log(log.join('\n'));
});
