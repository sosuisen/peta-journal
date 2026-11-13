#!/usr/bin/env node
// Measures the round trip of the requests the frontend makes while editing: the time from
// sending a request until its response arrives (the point where the frontend takes the saved
// state), and until the matching SSE `pagesUpdated` event arrives.
//
//   node bench/roundtrip.mjs http://localhost:8087 [iterations] [--json]
//
// The server must be running. Everything the benchmark creates goes to pages named
// `bench-roundtrip-*`; the page is deleted at the end.

const base = process.argv[2] ?? 'http://localhost:8087';
const iterations = Number(process.argv[3] ?? 200);
const asJson = process.argv.includes('--json');
const PAGE_ITEMS = 30; // a page of typical size, so that recalculations and reloads are realistic

// The server refuses a change of a client that does not name itself (ADR 015)
const clientId = { 'X-Client-Id': crypto.randomUUID() };

const json = (resp) => resp.json();

async function call(method, path, body) {
  const resp = await fetch(base + path, {
    method,
    headers: body === undefined ? clientId : { ...clientId, 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!resp.ok && resp.status !== 204) throw new Error(`${method} ${path} -> ${resp.status} ${await resp.text()}`);
  return resp.status === 204 ? null : json(resp);
}

// ---- SSE: remember when each pagesUpdated event arrived ----

const sseArrivals = []; // {pageId, at}
async function listenSse() {
  const resp = await fetch(base + '/api/events', { headers: { Accept: 'text/event-stream' } });
  const reader = resp.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  (async () => {
    for (;;) {
      const { value, done } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      let idx;
      while ((idx = buffer.indexOf('\n\n')) >= 0) {
        const chunk = buffer.slice(0, idx);
        buffer = buffer.slice(idx + 2);
        for (const line of chunk.split('\n')) {
          if (line.startsWith('data:')) {
            try {
              const ev = JSON.parse(line.slice(5).trim());
              if (ev.type === 'pagesUpdated') sseArrivals.push({ pageId: ev.pageId, at: performance.now() });
            } catch { /* keep-alive or non-JSON */ }
          }
        }
      }
    }
  })();
  // Wait for the "connected" event so the stream is really open
  await new Promise((r) => setTimeout(r, 200));
}

/** Time until an SSE pagesUpdated for the page arrives after `since`; waits up to 2 s. */
async function sseLatency(pageId, since) {
  const deadline = performance.now() + 2000;
  for (;;) {
    const hit = sseArrivals.find((a) => a.pageId === pageId && a.at >= since);
    if (hit) return hit.at - since;
    if (performance.now() > deadline) return NaN;
    await new Promise((r) => setTimeout(r, 1));
  }
}

// ---- stats ----

function stats(samples) {
  const s = samples.filter((x) => !Number.isNaN(x)).sort((a, b) => a - b);
  const q = (p) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  const mean = s.reduce((a, b) => a + b, 0) / s.length;
  return { n: s.length, min: s[0], p50: q(0.5), p95: q(0.95), p99: q(0.99), max: s[s.length - 1], mean };
}

const results = {};
async function measure(name, fn, { warmup = 20, withSse = false } = {}) {
  const http = [];
  const sse = [];
  for (let i = 0; i < warmup + iterations; i++) {
    const t0 = performance.now();
    const pageId = await fn(i);
    const t1 = performance.now();
    if (i < warmup) continue;
    http.push(t1 - t0);
    if (withSse) sse.push(await sseLatency(pageId, t0));
  }
  results[name] = { http: stats(http), sse: withSse ? stats(sse) : null };
}

// ---- the scenario ----

async function main() {
  await listenSse();
  const title = `bench-roundtrip-${Date.now()}`;
  const { page, items: initial } = await call('POST', '/api/pages/doc', { title, dirPath: '' });
  const pageId = page.id;
  const other = (await call('POST', '/api/pages/doc', { title: `${title}-other`, dirPath: '' })).page;

  // Fill the page like the frontend does: one item after another, with some nesting
  let after = initial[0].id;
  const ids = [];
  for (let i = 0; i < PAGE_ITEMS; i++) {
    const level = i % 3 === 2 ? 2 : 1;
    const item = (await call('POST', '/api/items', {
      pageId, position: `after:${after}`, level, body: `item ${i} with some text #tag${i % 5}`, id: '',
    })).item;
    ids.push(item.id);
    after = item.id;
  }
  await call('DELETE', `/api/items/${initial[0].id}`);
  const target = ids[Math.floor(PAGE_ITEMS / 2)];

  // 1. Auto-save of a body while typing (PUT /api/items/{id})
  await measure('update body (auto-save)', async (i) => {
    await call('PUT', `/api/items/${target}`, { body: `typed text ${i} with a [[link]] and #tag${i % 7}` });
    return pageId;
  }, { withSse: true });

  // 2. Enter: a new item after the current one, then Delete to keep the page the same size
  await measure('create item (Enter)', async () => {
    const item = (await call('POST', '/api/items', { pageId, position: `after:${target}`, level: 1, body: 'new line', id: '' })).item;
    await call('DELETE', `/api/items/${item.id}`);
    return pageId;
  });

  // 3. Tab / Shift+Tab: indent and outdent one item
  await measure('shift level (Tab)', async (i) => {
    await call('POST', '/api/items/shift-level', { itemIds: [target], delta: i % 2 === 0 ? 1 : -1 });
    return pageId;
  }, { withSse: true });

  // 4. Alt+Shift+Down / Up: move the item past its neighbour on the same page
  await measure('move item (Alt+Shift+Down)', async (i) => {
    const pos = i % 2 === 0 ? `after:${ids[ids.indexOf(target) + 1]}` : `after:${ids[ids.indexOf(target) - 1]}`;
    await call('POST', '/api/items/move', { itemIds: [target], targetPageId: pageId, position: pos });
    return pageId;
  }, { withSse: true });

  // 5. Ctrl+Enter: cycle the task state
  await measure('cycle task (Ctrl+Enter)', async () => {
    await call('POST', `/api/items/${target}/cycle-task`);
    return pageId;
  }, { withSse: true });

  // 6. Reload of the page (what the frontend does on an SSE event from another device)
  await measure(`get page (${PAGE_ITEMS} items)`, async () => {
    await call('GET', `/api/pages/${pageId}`);
    return pageId;
  });

  // 7. Search as you type
  await measure('search combined', async (i) => {
    await call('GET', `/api/search/combined?q=${encodeURIComponent('text ' + (i % 10))}&maxResults=20`);
    return pageId;
  });

  // 8. Page name suggestions for [[
  await measure('search pages ([[ popup)', async () => {
    await call('GET', `/api/search/pages?q=bench&maxResults=20`);
    return pageId;
  });

  await call('DELETE', `/api/pages/${pageId}`);
  await call('DELETE', `/api/pages/${other.id}`);

  if (asJson) {
    console.log(JSON.stringify({ base, iterations, results }, null, 2));
    return;
  }
  const f = (x) => (x === undefined || Number.isNaN(x) ? '   -  ' : x.toFixed(2).padStart(7));
  console.log(`${base}  (${iterations} iterations per operation, milliseconds)\n`);
  console.log('operation'.padEnd(30) + '   p50 ms   p95 ms   p99 ms   max ms  mean ms | SSE p50  SSE p95');
  for (const [name, r] of Object.entries(results)) {
    const h = r.http;
    const s = r.sse;
    console.log(
      name.padEnd(30) + ' ' + [h.p50, h.p95, h.p99, h.max, h.mean].map(f).join('  ')
      + ' | ' + (s ? [s.p50, s.p95].map(f).join('  ') : '   -        -'),
    );
  }
}

main().then(() => process.exit(0)).catch((e) => { console.error(e); process.exit(1); });
