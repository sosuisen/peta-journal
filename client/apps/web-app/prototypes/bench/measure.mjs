// Measures both prototypes of #971 in the Chrome of this machine, without a window. The dev
// servers of the prototypes and a throw-away journal server with the page of seed.mjs must
// be running.
//
//   node measure.mjs [rounds]

import puppeteer from 'puppeteer-core';

const CHROME = process.env.CHROME ?? 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const PAGE = '/pages/proto-1000';
const ROUNDS = Number(process.argv[2] ?? '5');
const APPS = [
  { name: 'React', origin: 'http://localhost:5181', query: '' },
  { name: 'Solid (store)', origin: 'http://localhost:5182', query: '&apply=store' },
  { name: 'Solid (signal)', origin: 'http://localhost:5182', query: '&apply=signal' },
];

const median = values => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];
const round = value => Math.round(value * 10) / 10;

async function opened(page, app, carry) {
  await page.goto(`${app.origin}${PAGE}?carry=${carry}${app.query}`);
  await page.waitForFunction(() => window.proto?.report().open.count > 0, { timeout: 30000 });
  return page.evaluate(() => ({
    open: window.proto.report().open.median,
    rows: document.querySelectorAll('.row').length,
    heap: performance.memory?.usedJSHeapSize ?? 0,
  }));
}

async function measureApp(browser, app, carry) {
  const page = await browser.newPage();
  await page.setViewport({ width: 1200, height: 900 });
  const errors = [];
  page.on('pageerror', error => errors.push(String(error)));
  page.on('console', message => {
    if (message.type() === 'error') {
      errors.push(message.text());
    }
  });

  // The parent of 400 items is unfolded before anything is measured: a run before may have
  // left it folded on the server
  await opened(page, app, carry);
  const unfolded = await page.evaluate(() => {
    if (window.proto.rowCount() >= 1000) {
      return true;
    }
    window.proto.toggle(window.proto.idOf('0 大きな親'));
    return false;
  });
  if (!unfolded) {
    await page.waitForFunction(() => window.proto.rowCount() >= 1000, { timeout: 10000 });
    await new Promise(resolve => setTimeout(resolve, 500));
  }

  // Opening the page of 1000 items, from the load of the page to the first rows on screen
  const opens = [];
  let first;
  for (let i = 0; i < ROUNDS; i++) {
    first = await opened(page, app, carry);
    opens.push(first.open);
  }

  // A reset of every row, carried both ways
  const reset = await page.evaluate(rounds => window.proto.timeReset(rounds), 20);

  // Typing into the editor of the second row
  await page.evaluate(() => window.proto.clear());
  const body = await page.waitForSelector('.row:nth-of-type(1) .body > div', { timeout: 10000 });
  const rowsBefore = await page.$$eval('.row', rows => rows.length);
  const target = (await page.$$('.body > div'))[1] ?? body;
  await target.click();
  await page.waitForSelector('textarea', { timeout: 10000 });
  await page.keyboard.press('End');
  const typed = ' abcdefghij'.repeat(4);
  await page.keyboard.type(typed, { delay: 30 });
  await new Promise(resolve => setTimeout(resolve, 300));
  const afterTyping = await page.$eval('textarea', area => area.value);
  const keys = await page.evaluate(() => window.proto.report().key);

  // Enter makes a new item, and the cursor goes into it
  await page.keyboard.press('Enter');
  await new Promise(resolve => setTimeout(resolve, 300));
  const enter = await page.evaluate(() => ({
    focused: document.activeElement?.tagName === 'TEXTAREA',
    value: document.activeElement?.value ?? null,
  }));
  await page.keyboard.press('Escape');

  // Folding and unfolding the parent of 400 items
  await page.evaluate(() => window.proto.clear());
  const parentId = await page.evaluate(() => window.proto.idOf('0 大きな親'));
  const rowsUnfolded = await page.evaluate(() => window.proto.rowCount());
  for (let i = 0; i < ROUNDS * 2; i++) {
    const count = await page.evaluate(() => window.proto.report().expand.count);
    await page.evaluate(id => window.proto.toggle(id), parentId);
    await page.waitForFunction(
      c => window.proto.report().expand.count > c,
      { timeout: 10000 },
      count,
    );
  }
  const rowsAfter = await page.evaluate(() => window.proto.rowCount());
  const expand = await page.evaluate(() => window.proto.report().expand);

  // The search field gets the Search panel back after every letter
  await page.evaluate(() => window.proto.clear());
  await page.click('.search input');
  const query = 'sample text';
  await page.keyboard.type(query, { delay: 60 });
  const results = await page
    .waitForFunction(() => document.querySelectorAll('.search li').length > 0, { timeout: 10000 })
    .then(() => page.$$eval('.search li', items => items.length))
    .catch(() => 0);
  const search = await page.evaluate(() => ({
    value: document.querySelector('.search input').value,
    focused: document.activeElement === document.querySelector('.search input'),
    key: window.proto.report().key,
  }));
  await page.close();

  return {
    app: app.name,
    carry,
    openMs: round(median(opens)),
    rowsInDom: first.rows,
    resetRows: reset.rows,
    resetJsonMs: round(reset.json),
    resetValueMs: round(reset.value),
    keyMedianMs: round(keys.median),
    keyP95Ms: round(keys.p95),
    keyCount: keys.count,
    typedKept: afterTyping.endsWith(typed),
    rowsBefore,
    enterFocused: enter.focused && enter.value === '',
    expandMedianMs: round(expand.median),
    expandP95Ms: round(expand.p95),
    expandCount: expand.count,
    searchResults: results,
    searchKept: search.value === query && search.focused,
    searchKeyMedianMs: round(search.key.median),
    rowsUnfolded,
    rowsAfter,
    errors,
  };
}

const browser = await puppeteer.launch({
  executablePath: CHROME,
  headless: true,
  args: ['--enable-precise-memory-info'],
});
try {
  const results = [];
  for (const app of APPS) {
    for (const carry of ['json', 'value']) {
      results.push(await measureApp(browser, app, carry));
    }
  }
  console.log(JSON.stringify(results, null, 2));
} finally {
  await browser.close();
}
