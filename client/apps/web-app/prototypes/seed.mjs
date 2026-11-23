// Makes the page of the measurements of #971 on a throw-away journal server: 1000 items of
// generated text. Run it against a server started with --throwaway-dir, never against the
// server with your notes.
//
//   node seed.mjs http://127.0.0.1:18971

import { randomUUID } from 'node:crypto';

const server = process.argv[2];
if (server === undefined) {
  throw new Error('give the URL of a throw-away server');
}
const clientId = randomUUID();

async function call(method, path, body) {
  const response = await fetch(`${server}${path}`, {
    method,
    headers: { 'content-type': 'application/json', 'x-client-id': clientId },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!response.ok) {
    throw new Error(`${method} ${path}: ${response.status} ${await response.text()}`);
  }
  return response.json();
}

const WORDS = ['テスト', 'サンプル', '項目', 'sample', 'text', '計測', '行', 'outline', '本文'];

/** A body of the item `n`, with some Markdown so that the HTML is not plain text. */
function body(n) {
  const words = Array.from({ length: 4 + (n % 9) }, (_, i) => WORDS[(n * 7 + i) % WORDS.length]);
  if (n % 10 === 3) {
    words.push(`**強調${n}**`);
  }
  if (n % 25 === 7) {
    words.push('`code`');
  }
  return `${n} ${words.join(' ')}`;
}

/** The levels: a parent with 400 children first, then groups of a parent and 9 children. */
function level(n) {
  if (n === 0) {
    return 1;
  }
  if (n <= 400) {
    return 2;
  }
  return (n - 401) % 10 === 0 ? 1 : 2;
}

const page = await call('POST', '/api/pages/doc', { title: 'proto-1000', dirPath: '' });
const pageId = page.id ?? page.page?.id;
if (pageId === undefined) {
  throw new Error(`no page id in ${JSON.stringify(page).slice(0, 200)}`);
}
let after = 'last';
for (let n = 0; n < 1000; n++) {
  const created = await call('POST', '/api/items', {
    pageId,
    position: after,
    level: level(n),
    body: n === 0 ? '0 大きな親（子が400）' : body(n),
  });
  const id = created.id ?? created.item?.id;
  after = `after:${id}`;
}
console.log(`made ${pageId} with 1000 items`);
