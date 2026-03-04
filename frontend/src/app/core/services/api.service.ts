import { Injectable } from '@angular/core';
import { Item } from '../models/item.model';
import { Page } from '../models/page.model';
import { SearchResult } from '../models/search.model';

interface GetPageResult { page: Page; items: Item[] }
interface GetDocResult { page: Page; items: Item[]; created: boolean }
interface GetJournalResult { page: Page; items: Item[]; created: boolean }
interface GetMostRecentJournalBeforeResult { found: boolean; page: Page | null; items: Item[] }
interface SearchPageNamesResult { results: SearchResult[] }
interface SearchFullTextResult { results: SearchResult[]; totalHits: number }
interface GetRecentPagesResult { pages: Page[] }
interface BacklinkEntry { sourcePage: Page; sourceItem: Item }
interface GetBacklinksResult { entries: BacklinkEntry[] }
async function json<T>(resp: Response): Promise<T> {
  if (!resp.ok) {
    const text = await resp.text().catch(() => '');
    throw new Error(text || `HTTP ${resp.status}`);
  }
  return await resp.json();
}

@Injectable({ providedIn: 'root' })
export class ApiService {

  async getPageById(pageId: string): Promise<GetPageResult> {
    const resp = await fetch(`/api/pages/${pageId}`);
    return json(resp);
  }

  async getMostRecentJournalBefore(beforeDate: string): Promise<GetMostRecentJournalBeforeResult> {
    const resp = await fetch(`/api/journals/before?date=${encodeURIComponent(beforeDate)}`);
    return json(resp);
  }

  async getMostRecentWeeklyJournalBefore(beforeWeek: string): Promise<GetMostRecentJournalBeforeResult> {
    const resp = await fetch(`/api/journals/weekly/before?week=${encodeURIComponent(beforeWeek)}`);
    return json(resp);
  }

  async getOrCreateJournal(fileName: string): Promise<GetJournalResult> {
    const resp = await fetch(`/api/journals/${encodeURIComponent(fileName)}`, { method: 'POST' });
    return json(resp);
  }

  async getOrCreateDoc(title: string, dirPath?: string, tags?: string[]): Promise<GetDocResult> {
    // Parse path: "cat/01.md" -> dirPath="cat/", title="01"
    if (!dirPath) {
      const stripped = title.replace(/\.md$/, '');
      const lastSlash = stripped.lastIndexOf('/');
      if (lastSlash >= 0) {
        dirPath = stripped.substring(0, lastSlash + 1);
        title = stripped.substring(lastSlash + 1);
      } else {
        dirPath = '';
        title = stripped;
      }
    }
    const resp = await fetch('/api/pages/doc', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ title, dirPath, tags }),
    });
    return json(resp);
  }

  async createItem(pageId: string, afterItemId: string, level: number, body: string, id?: string): Promise<Item> {
    const resp = await fetch('/api/items', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pageId, afterItemId, level, body, id: id ?? '' }),
    });
    const data = await json<{ item: Item }>(resp);
    return data.item;
  }

  async updateItem(itemId: string, updates: { body?: string; level?: number; collapsed?: boolean }): Promise<Item> {
    const resp = await fetch(`/api/items/${itemId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(updates),
    });
    const data = await json<{ item: Item }>(resp);
    return data.item;
  }

  async moveItems(itemIds: string[], targetPageId: string, afterItemId: string, levelDelta?: number): Promise<Item[]> {
    const resp = await fetch('/api/items/move', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ itemIds, targetPageId, afterItemId, levelDelta }),
    });
    const data = await json<{ items: Item[] }>(resp);
    return data.items;
  }

  async deletePage(pageId: string): Promise<void> {
    const resp = await fetch(`/api/pages/${pageId}`, { method: 'DELETE' });
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
  }

  async deleteItem(itemId: string): Promise<void> {
    const resp = await fetch(`/api/items/${itemId}`, { method: 'DELETE' });
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
  }

  async cycleTask(itemId: string): Promise<Item> {
    const resp = await fetch(`/api/items/${itemId}/cycle-task`, { method: 'POST' });
    const data = await json<{ item: Item }>(resp);
    return data.item;
  }

  async upsertItemProperty(itemId: string, key: string, valueType: string, value: string): Promise<Item> {
    const resp = await fetch(`/api/items/${itemId}/properties/${encodeURIComponent(key)}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ valueType, value }),
    });
    const data = await json<{ item: Item }>(resp);
    return data.item;
  }

  async searchPageNames(query: string, maxResults = 20): Promise<SearchPageNamesResult> {
    const resp = await fetch(`/api/search/pages?q=${encodeURIComponent(query)}&maxResults=${maxResults}`);
    return json(resp);
  }

  async searchFullText(query: string, maxResults = 20): Promise<SearchFullTextResult> {
    const resp = await fetch(`/api/search/fulltext?q=${encodeURIComponent(query)}&maxResults=${maxResults}`);
    return json(resp);
  }

  async getRecentPages(maxResults = 10, type?: string): Promise<GetRecentPagesResult> {
    let url = `/api/pages/recent?maxResults=${maxResults}`;
    if (type) url += `&type=${encodeURIComponent(type)}`;
    const resp = await fetch(url);
    return json(resp);
  }

  async updatePage(pageId: string, updates: { title?: string; dirPath?: string }): Promise<Page> {
    const resp = await fetch(`/api/pages/${pageId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(updates),
    });
    const data = await json<{ page: Page }>(resp);
    return data.page;
  }

  async getBacklinks(pageId: string): Promise<GetBacklinksResult> {
    const resp = await fetch(`/api/pages/${pageId}/backlinks`);
    return json(resp);
  }

  async getConfig(): Promise<{ weekNumbering: string }> {
    const resp = await fetch('/api/config');
    return json(resp);
  }

  async getPropertyDefinitions(): Promise<{ propertyDefinitions: Record<string, string> }> {
    const resp = await fetch('/api/config/property-definitions');
    return json(resp);
  }

  async updateConfig(weekNumbering: string): Promise<{ weekNumbering: string }> {
    const resp = await fetch('/api/config', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ weekNumbering }),
    });
    return json(resp);
  }

  async exportTreeMarkdown(itemIds: string[]): Promise<{ treeMarkdown: string; ast: string; flatMarkdown: string }> {
    const resp = await fetch('/api/items/export', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ itemIds }),
    });
    return json(resp);
  }

  async importTreeMarkdown(treeMarkdown: string): Promise<{ flatMarkdown: string }> {
    const resp = await fetch('/api/items/import', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ treeMarkdown }),
    });
    return json(resp);
  }

  async getChildPages(parentId: string): Promise<Page[]> {
    const resp = await fetch(`/api/pages/${parentId}/children`);
    const data = await json<{ pages: Page[] }>(resp);
    return data.pages;
  }

  async uploadAsset(file: File): Promise<{ fileName: string }> {
    const formData = new FormData();
    formData.append('file', file);
    const resp = await fetch('/api/assets/upload', {
      method: 'POST',
      body: formData,
    });
    if (!resp.ok) {
      throw new Error(`Upload failed: ${resp.status}`);
    }
    return await resp.json();
  }
}
