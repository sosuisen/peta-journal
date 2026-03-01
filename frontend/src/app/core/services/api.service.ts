import { Injectable } from '@angular/core';
import { Block } from '../models/block.model';
import { Bookmark, BookmarkGroup } from '../models/bookmark.model';
import { Page } from '../models/page.model';
import { SearchResult } from '../models/search.model';

interface GetPageResult { page: Page; blocks: Block[] }
interface GetDocResult { page: Page; blocks: Block[]; created: boolean }
interface GetJournalResult { page: Page; blocks: Block[]; created: boolean }
interface GetMostRecentJournalBeforeResult { found: boolean; page: Page | null; blocks: Block[] }
interface SearchPageNamesResult { results: SearchResult[] }
interface SearchFullTextResult { results: SearchResult[]; totalHits: number }
interface GetRecentPagesResult { pages: Page[] }
interface BacklinkEntry { sourcePage: Page; sourceBlock: Block }
interface GetBacklinksResult { entries: BacklinkEntry[] }
interface GetBookmarksResult { groups: BookmarkGroup[]; bookmarks: Bookmark[] }

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

  async createBlock(pageId: string, afterBlockId: string, level: number, body: string, id?: string): Promise<Block> {
    const resp = await fetch('/api/blocks', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pageId, afterBlockId, level, body, id: id ?? '' }),
    });
    const data = await json<{ block: Block }>(resp);
    return data.block;
  }

  async updateBlock(blockId: string, updates: { body?: string; level?: number; collapsed?: boolean }): Promise<Block> {
    const resp = await fetch(`/api/blocks/${blockId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(updates),
    });
    const data = await json<{ block: Block }>(resp);
    return data.block;
  }

  async moveBlocks(blockIds: string[], targetPageId: string, afterBlockId: string, levelDelta?: number): Promise<Block[]> {
    const resp = await fetch('/api/blocks/move', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ blockIds, targetPageId, afterBlockId, levelDelta }),
    });
    const data = await json<{ blocks: Block[] }>(resp);
    return data.blocks;
  }

  async deletePage(pageId: string): Promise<void> {
    const resp = await fetch(`/api/pages/${pageId}`, { method: 'DELETE' });
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
  }

  async deleteBlock(blockId: string): Promise<void> {
    const resp = await fetch(`/api/blocks/${blockId}`, { method: 'DELETE' });
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
  }

  async updateBlockStatus(blockId: string, status: string): Promise<Block> {
    const resp = await fetch(`/api/blocks/${blockId}/status`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status }),
    });
    const data = await json<{ block: Block }>(resp);
    return data.block;
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

  async updateConfig(weekNumbering: string): Promise<{ weekNumbering: string }> {
    const resp = await fetch('/api/config', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ weekNumbering }),
    });
    return json(resp);
  }

  async exportTreeMarkdown(blockIds: string[]): Promise<{ treeMarkdown: string; html: string; flatMarkdown: string }> {
    const resp = await fetch('/api/blocks/export', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ blockIds }),
    });
    return json(resp);
  }

  async importTreeMarkdown(treeMarkdown: string): Promise<{ flatMarkdown: string }> {
    const resp = await fetch('/api/blocks/import', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ treeMarkdown }),
    });
    return json(resp);
  }

  async getBookmarks(): Promise<GetBookmarksResult> {
    const resp = await fetch('/api/bookmarks');
    return json(resp);
  }

  async createBookmarkGroup(name: string): Promise<BookmarkGroup> {
    const resp = await fetch('/api/bookmarks/groups', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name }),
    });
    const data = await json<{ group: BookmarkGroup }>(resp);
    return data.group;
  }

  async createBookmark(groupId: string, targetType: string, targetId: string): Promise<Bookmark> {
    const resp = await fetch('/api/bookmarks', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ groupId, targetType, targetId }),
    });
    const data = await json<{ bookmark: Bookmark }>(resp);
    return data.bookmark;
  }

  async deleteBookmark(bookmarkId: string): Promise<void> {
    const resp = await fetch(`/api/bookmarks/${bookmarkId}`, { method: 'DELETE' });
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
  }

  async deleteBookmarkGroup(groupId: string): Promise<void> {
    const resp = await fetch(`/api/bookmarks/groups/${groupId}`, { method: 'DELETE' });
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
  }

  async moveBookmark(bookmarkId: string, targetGroupId: string, afterBookmarkId: string): Promise<Bookmark> {
    const resp = await fetch('/api/bookmarks/move', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ bookmarkId, targetGroupId, afterBookmarkId }),
    });
    const data = await json<{ bookmark: Bookmark }>(resp);
    return data.bookmark;
  }

  async moveBookmarkGroup(groupId: string, afterGroupId: string): Promise<BookmarkGroup> {
    const resp = await fetch('/api/bookmarks/groups/move', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ groupId, afterGroupId }),
    });
    const data = await json<{ group: BookmarkGroup }>(resp);
    return data.group;
  }

  async updateBookmarkGroup(groupId: string, name: string): Promise<BookmarkGroup> {
    const resp = await fetch(`/api/bookmarks/groups/${groupId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name }),
    });
    const data = await json<{ group: BookmarkGroup }>(resp);
    return data.group;
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
