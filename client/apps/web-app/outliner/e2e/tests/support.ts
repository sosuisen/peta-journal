import { APIRequestContext, Page, expect, test as base } from '@playwright/test';
import { randomUUID } from 'node:crypto';
import { serverPort } from '../playwright.config';

/**
 * The tests' own `test`: its `request` names itself to the server, which refuses a change
 * without a client ID (ADR 015). The page in the browser is left alone: the app names itself.
 */
export const test = base.extend({
  request: async ({ playwright }, use) => {
    const context = await playwright.request.newContext({ extraHTTPHeaders: { 'X-Client-Id': randomUUID() } });
    await use(context);
    await context.dispose();
  },
});

/** An item as the server stores it; only the fields the tests look at. */
export interface StoredItem {
  id: string;
  level: number;
  body: string;
  types: string[];
  properties: Record<string, unknown>;
  collapsed: boolean;
}

/** Talks to the journal server directly, past the app in the browser. */
export class Server {
  private readonly base = `http://localhost:${serverPort}`;

  constructor(private readonly request: APIRequestContext) {}

  /** Creates an empty doc page with a unique title and returns its id and title. */
  async newDocPage(prefix: string): Promise<{ id: string; title: string }> {
    const title = `${prefix}-${Date.now()}`;
    const resp = await this.request.post(`${this.base}/api/pages/doc`, { data: { title, dirPath: '' } });
    expect(resp.ok()).toBe(true);
    const { page } = await resp.json();
    return { id: page.id, title };
  }

  /**
   * Fills a page with items given as "body" or "  body" (two spaces per level below 1),
   * replacing the empty item a new page starts with, and returns them in page order.
   */
  async fill(pageId: string, lines: string[]): Promise<StoredItem[]> {
    const [first] = await this.items(pageId);
    let after = first.id;
    for (const line of lines) {
      const body = line.trimStart();
      const level = 1 + (line.length - body.length) / 2;
      const resp = await this.request.post(`${this.base}/api/items`, {
        data: { pageId, position: `after:${after}`, level, body, id: '' },
      });
      expect(resp.ok()).toBe(true);
      after = (await resp.json()).item.id;
    }
    await this.request.delete(`${this.base}/api/items/${first.id}`);
    return this.items(pageId);
  }

  async items(pageId: string): Promise<StoredItem[]> {
    const resp = await this.request.get(`${this.base}/api/pages/${pageId}`);
    expect(resp.ok()).toBe(true);
    const { items } = await resp.json();
    return items.map((i: StoredItem) => ({ id: i.id, level: i.level, body: i.body, types: i.types ?? [], properties: i.properties ?? {}, collapsed: !!i.collapsed }));
  }

  /** Makes the atlas page of a directory, which turns the page of that name into its index page. */
  async newAtlas(dirPath: string): Promise<{ id: string }> {
    const resp = await this.request.post(`${this.base}/api/pages/doc`, { data: { title: 'index_atlas', dirPath } });
    expect(resp.ok()).toBe(true);
    return { id: (await resp.json()).page.id };
  }

  /** Whether the server still has the page. */
  async hasPage(pageId: string): Promise<boolean> {
    return (await this.request.get(`${this.base}/api/pages/${pageId}`)).ok();
  }

  /** Deletes an item the way another client does. */
  async deleteItem(itemId: string): Promise<void> {
    const resp = await this.request.delete(`${this.base}/api/items/${itemId}`);
    expect(resp.ok()).toBe(true);
  }

  /**
   * The markdown folder of the test server. Not `markdownDir` of the config: each process of
   * the tests reads the config again, and makes a folder of its own.
   */
  async markdownDir(): Promise<string> {
    const resp = await this.request.get(`${this.base}/api/config`);
    expect(resp.ok()).toBe(true);
    return ((await resp.json()) as { markdownDir: string }).markdownDir;
  }

  /** Changes the body of an item the way another client does. */
  async updateBody(itemId: string, body: string): Promise<void> {
    const resp = await this.request.put(`${this.base}/api/items/${itemId}`, { data: { body } });
    expect(resp.ok()).toBe(true);
  }

  /** Waits until the stored items of the page satisfy the check; the auto-save needs a moment. */
  async itemsSettled(pageId: string, check: (items: StoredItem[]) => boolean): Promise<StoredItem[]> {
    await expect.poll(async () => check(await this.items(pageId)), { timeout: 5_000 }).toBe(true);
    return this.items(pageId);
  }

  /** Turns the logical outdent on or off for the whole app; the app reads it when a page loads. */
  async setLogicalOutdent(on: boolean): Promise<void> {
    const resp = await this.request.put(`${this.base}/api/config`, { data: { logicalOutdent: on } });
    expect(resp.ok()).toBe(true);
  }

  /** Gets or creates the journal page of a day (YYYY-MM-DD) and returns its id. */
  async journal(date: string): Promise<{ id: string; name: string }> {
    const resp = await this.request.post(`${this.base}/api/journals/${date}`);
    expect(resp.ok()).toBe(true);
    const { page } = await resp.json();
    return { id: page.id, name: page.name };
  }

  /** The most recently touched journal pages, newest first. */
  async recentJournals(): Promise<Array<{ id: string; name: string }>> {
    const resp = await this.request.get(`${this.base}/api/pages/recent?maxResults=10&type=journal`);
    expect(resp.ok()).toBe(true);
    const { pages } = await resp.json();
    return pages.map((p: { id: string; name: string }) => ({ id: p.id, name: p.name }));
  }
}

/** "level:body" of every stored item, the shape most assertions compare. */
export const outlineOf = (items: StoredItem[]) => items.map(i => `${i.level}:${i.body}`);

export const editorOf = (page: Page, itemId: string) => page.locator(`#item-editor-${itemId}`);

/** The items the outline shows, as text. */
export async function shownBodies(page: Page): Promise<string[]> {
  return page.locator('app-outline-item').evaluateAll(els =>
    els.map(e => (e.querySelector('textarea')?.value ?? e.querySelector('.item-body, app-item-view')?.textContent ?? '').trim()));
}

/** Opens a page of the app and waits for its outline. */
export async function openPage(page: Page, title: string): Promise<void> {
  await page.goto(`/pages/${title}`);
  await page.locator('app-outline-item').first().waitFor();
}

/** Clicks an item so its editor opens and has the focus. */
export async function openEditor(page: Page, itemId: string): Promise<void> {
  await page.locator(`[id="${itemId}"]`).click();
  await expect(editorOf(page, itemId)).toBeFocused();
}
