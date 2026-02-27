import { Injectable, inject } from '@angular/core';
import { ProtobufService } from './protobuf.service';
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

function toPage(p: any): Page {
  return {
    id: p.id ?? '',
    parentId: p.parentId ?? '',
    dirPath: p.dirPath ?? '',
    name: p.name ?? '',
    type: p.type ?? 'doc',
    createdDate: p.createdDate?.millis?.toNumber?.() ?? Number(p.createdDate?.millis ?? 0),
    modifiedDate: p.modifiedDate?.millis?.toNumber?.() ?? Number(p.modifiedDate?.millis ?? 0),
    tags: p.tags ?? [],
  };
}

function toBlock(b: any): Block {
  return {
    id: b.id ?? '',
    pageId: b.pageId ?? '',
    dirPath: b.dirPath ?? '',
    name: b.name ?? '',
    type: b.type ?? 'doc',
    createdDate: b.createdDate?.millis?.toNumber?.() ?? Number(b.createdDate?.millis ?? 0),
    modifiedDate: b.modifiedDate?.millis?.toNumber?.() ?? Number(b.modifiedDate?.millis ?? 0),
    idx: b.idx ?? '',
    level: b.level ?? 1,
    body: b.body ?? '',
    html: b.html ?? '',
    collapsed: b.collapsed ?? false,
    status: b.status ?? 'normal',
    types: b.types ?? [],
  };
}

function toBookmarkGroup(g: any): BookmarkGroup {
  return {
    id: g.id ?? '',
    name: g.name ?? '',
    sortOrder: g.sortOrder ?? '',
    createdAt: g.createdAt?.millis?.toNumber?.() ?? Number(g.createdAt?.millis ?? 0),
  };
}

function toBookmark(b: any): Bookmark {
  return {
    id: b.id ?? '',
    groupId: b.groupId ?? '',
    targetType: b.targetType ?? 'page',
    targetId: b.targetId ?? '',
    sortOrder: b.sortOrder ?? '',
    createdAt: b.createdAt?.millis?.toNumber?.() ?? Number(b.createdAt?.millis ?? 0),
    targetPage: b.targetPage ? toPage(b.targetPage) : null,
  };
}

@Injectable({ providedIn: 'root' })
export class ApiService {
  private readonly proto = inject(ProtobufService);

  /**
   * Retrieves a page and its blocks by page ID.
   * @param pageId - The UUID of the page to retrieve.
   * @returns The page and its associated blocks.
   */
  async getPageById(pageId: string): Promise<GetPageResult> {
    const resp: any = await this.proto.send({ getPageById: { pageId } });
    const r = resp.getPageById;
    return { page: toPage(r.page), blocks: (r.blocks ?? []).map(toBlock) };
  }

  /**
   * Gets the most recent daily journal before the specified date.
   * @param beforeDate - Date in YYYY-MM-DD format (exclusive upper bound).
   * @returns The journal page and blocks if found, or a not-found result.
   */
  async getMostRecentJournalBefore(beforeDate: string): Promise<GetMostRecentJournalBeforeResult> {
    const resp: any = await this.proto.send({ getMostRecentJournalBefore: { beforeDate } });
    const r = resp.getMostRecentJournalBefore;
    if (!r.found) {
      return { found: false, page: null, blocks: [] };
    }
    return { found: true, page: toPage(r.page), blocks: (r.blocks ?? []).map(toBlock) };
  }

  /**
   * Gets or creates a journal page by file name (e.g., '2026-02-18', '2026-W08').
   * @param fileName - The journal file name in daily, weekly, monthly, or yearly format.
   * @returns The journal page, its blocks, and whether it was newly created.
   */
  async getOrCreateJournal(fileName: string): Promise<GetJournalResult> {
    const resp: any = await this.proto.send({ getOrCreateJournal: { name: fileName } });
    const r = resp.getOrCreateJournal;
    return { page: toPage(r.page), blocks: (r.blocks ?? []).map(toBlock), created: r.created ?? false };
  }

  /**
   * Gets or creates a document page by title. Parses path from title if dirPath is not provided.
   * @param title - The document title, optionally including a path (e.g., 'projects/myDoc').
   * @param dirPath - Optional directory path (e.g., 'projects/'). Parsed from title if omitted.
   * @param tags - Optional array of tags to assign to the document.
   * @returns The document page, its blocks, and whether it was newly created.
   */
  async getOrCreateDoc(title: string, dirPath?: string, tags?: string[]): Promise<GetDocResult> {
    // Parse path: "cat/01.md" → dirPath="cat/", title="01"
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
    const resp: any = await this.proto.send({ getOrCreateDoc: { title, dirPath, tags } });
    const r = resp.getOrCreateDoc;
    return { page: toPage(r.page), blocks: (r.blocks ?? []).map(toBlock), created: r.created ?? false };
  }

  /**
   * Creates a new block in a page, inserted after the specified block.
   * @param pageId - The UUID of the page to add the block to.
   * @param afterBlockId - The block ID to insert after, or empty string to append at end.
   * @param level - The indentation level (0-based).
   * @param body - The markdown body text of the block.
   * @returns The newly created block.
   */
  async createBlock(pageId: string, afterBlockId: string, level: number, body: string, id?: string): Promise<Block> {
    const resp: any = await this.proto.send({ createBlock: { pageId, afterBlockId, level, body, id: id ?? '' } });
    return toBlock(resp.createBlock.block);
  }

  /**
   * Updates an existing block's body, level, collapsed state, or properties.
   * @param blockId - The 7-char hex block ID.
   * @param updates - An object with optional fields to update.
   * @returns The updated block.
   */
  async updateBlock(blockId: string, updates: { body?: string; level?: number; collapsed?: boolean }): Promise<Block> {
    const resp: any = await this.proto.send({ updateBlock: { blockId, ...updates } });
    return toBlock(resp.updateBlock.block);
  }

  /**
   * Moves blocks to a target page and position, returning blocks with fresh indices.
   * @param blockIds - Array of block IDs to move.
   * @param targetPageId - The UUID of the destination page.
   * @param afterBlockId - The block ID to insert after, or empty string to insert at beginning.
   * @param levelDelta - Optional indentation level adjustment.
   * @returns The moved blocks with updated indices.
   */
  async moveBlocks(blockIds: string[], targetPageId: string, afterBlockId: string, levelDelta?: number): Promise<Block[]> {
    const resp: any = await this.proto.send({ moveBlocks: { blockIds, targetPageId, afterBlockId, levelDelta } });
    return (resp.moveBlocks.blocks ?? []).map(toBlock);
  }

  /**
   * Deletes a page and all its blocks.
   * @param pageId - The UUID of the page to delete.
   */
  async deletePage(pageId: string): Promise<void> {
    await this.proto.send({ deletePage: { pageId } });
  }

  /**
   * Deletes a block by its ID.
   * @param blockId - The 7-char hex block ID to delete.
   */
  async deleteBlock(blockId: string): Promise<void> {
    await this.proto.send({ deleteBlock: { blockId } });
  }

  /**
   * Updates a block's status (e.g., 'normal', 'todo', 'running', 'done').
   * @param blockId - The 7-char hex block ID.
   * @param status - The new status value.
   * @returns The updated block.
   */
  async updateBlockStatus(blockId: string, status: string): Promise<Block> {
    const resp: any = await this.proto.send({ updateBlockStatus: { blockId, status } });
    return toBlock(resp.updateBlockStatus.block);
  }

  /**
   * Searches page names using prefix, fuzzy/token, and substring matching.
   * @param query - The search query string.
   * @param maxResults - Maximum number of results to return (default 20).
   * @returns Matching search results.
   */
  async searchPageNames(query: string, maxResults = 20): Promise<SearchPageNamesResult> {
    const resp: any = await this.proto.send({ searchPageNames: { query, maxResults } });
    return {
      results: (resp.searchPageNames.results ?? []).map((r: any) => ({
        pageId: r.pageId ?? '',
        pageTitle: r.pageTitle ?? '',
        blockId: r.blockId ?? '',
        snippet: r.snippet ?? '',
        score: r.score ?? 0,
      })),
    };
  }

  /**
   * Performs a full-text search across all block content.
   * @param query - The search query string.
   * @param maxResults - Maximum number of results to return (default 20).
   * @returns Matching search results and total hit count.
   */
  async searchFullText(query: string, maxResults = 20): Promise<SearchFullTextResult> {
    const resp: any = await this.proto.send({ searchFullText: { query, maxResults } });
    return {
      results: (resp.searchFullText.results ?? []).map((r: any) => ({
        pageId: r.pageId ?? '',
        pageTitle: r.pageTitle ?? '',
        blockId: r.blockId ?? '',
        snippet: r.snippet ?? '',
        score: r.score ?? 0,
      })),
      totalHits: resp.searchFullText.totalHits ?? 0,
    };
  }

  /**
   * Gets recently modified pages, ordered by modification date (newest first).
   * @param maxResults - Maximum number of pages to return (default 10).
   * @param type - Optional filter by page type ('journal', 'doc', or omit for all).
   * @returns The list of recently modified pages.
   */
  async getRecentPages(maxResults = 10, type?: string): Promise<GetRecentPagesResult> {
    const resp: any = await this.proto.send({ getRecentPages: { maxResults, type: type ?? '' } });
    return { pages: (resp.getRecentPages.pages ?? []).map(toPage) };
  }

  /**
   * Updates a page's title or directory path.
   * @param pageId - The UUID of the page to update.
   * @param updates - An object with optional title and dirPath fields.
   * @returns The updated page.
   */
  async updatePage(pageId: string, updates: { title?: string; dirPath?: string }): Promise<Page> {
    const resp: any = await this.proto.send({ updatePage: { pageId, ...updates } });
    return toPage(resp.updatePage.page);
  }

  /**
   * Gets all pages and blocks that link to the specified page (backlinks).
   * @param pageId - The UUID of the target page.
   * @returns The backlink entries, each containing a source page and source block.
   */
  async getBacklinks(pageId: string): Promise<GetBacklinksResult> {
    const resp: any = await this.proto.send({ getBacklinks: { pageId } });
    return {
      entries: (resp.getBacklinks.entries ?? []).map((e: any) => ({
        sourcePage: toPage(e.sourcePage),
        sourceBlock: toBlock(e.sourceBlock),
      })),
    };
  }

  /**
   * Gets the server configuration (e.g., week numbering setting).
   * @returns The current configuration.
   */
  async getConfig(): Promise<{ weekNumbering: string }> {
    const resp: any = await this.proto.send({ getConfig: {} });
    const r = resp.getConfig;
    return { weekNumbering: r.weekNumbering ?? 'us' };
  }

  /**
   * Updates the server configuration.
   * @param weekNumbering - The week numbering system ('us' or 'iso').
   * @returns The updated configuration.
   */
  async updateConfig(weekNumbering: string): Promise<{ weekNumbering: string }> {
    const resp: any = await this.proto.send({ updateConfig: { weekNumbering } });
    const r = resp.updateConfig;
    return { weekNumbering: r.weekNumbering ?? 'us' };
  }

  /**
   * Exports blocks as hierarchical tree markdown, flat markdown, and HTML.
   * @param blockIds - Array of block IDs to export.
   * @returns The tree markdown, flat markdown, and HTML representations.
   */
  async exportTreeMarkdown(blockIds: string[]): Promise<{ treeMarkdown: string; html: string; flatMarkdown: string }> {
    const resp: any = await this.proto.send({ exportTreeMarkdown: { blockIds } });
    const r = resp.exportTreeMarkdown;
    return { treeMarkdown: r.treeMarkdown ?? '', html: r.html ?? '', flatMarkdown: r.flatMarkdown ?? '' };
  }

  /**
   * Imports tree markdown and converts it to flat markdown.
   * @param treeMarkdown - The hierarchical tree markdown string to import.
   * @returns The converted flat markdown representation.
   */
  async importTreeMarkdown(treeMarkdown: string): Promise<{ flatMarkdown: string }> {
    const resp: any = await this.proto.send({ importTreeMarkdown: { treeMarkdown } });
    const r = resp.importTreeMarkdown;
    return { flatMarkdown: r.flatMarkdown ?? '' };
  }

  async getBookmarks(): Promise<GetBookmarksResult> {
    const resp: any = await this.proto.send({ getBookmarks: {} });
    const r = resp.getBookmarks;
    return {
      groups: (r.groups ?? []).map(toBookmarkGroup),
      bookmarks: (r.bookmarks ?? []).map(toBookmark),
    };
  }

  async createBookmarkGroup(name: string): Promise<BookmarkGroup> {
    const resp: any = await this.proto.send({ createBookmarkGroup: { name } });
    return toBookmarkGroup(resp.createBookmarkGroup.group);
  }

  async createBookmark(groupId: string, targetType: string, targetId: string): Promise<Bookmark> {
    const resp: any = await this.proto.send({ createBookmark: { groupId, targetType, targetId } });
    return toBookmark(resp.createBookmark.bookmark);
  }

  async deleteBookmark(bookmarkId: string): Promise<void> {
    await this.proto.send({ deleteBookmark: { bookmarkId } });
  }

  async deleteBookmarkGroup(groupId: string): Promise<void> {
    await this.proto.send({ deleteBookmarkGroup: { groupId } });
  }

  async moveBookmark(bookmarkId: string, targetGroupId: string, afterBookmarkId: string): Promise<Bookmark> {
    const resp: any = await this.proto.send({ moveBookmark: { bookmarkId, targetGroupId, afterBookmarkId } });
    return toBookmark(resp.moveBookmark.bookmark);
  }

  async moveBookmarkGroup(groupId: string, afterGroupId: string): Promise<BookmarkGroup> {
    const resp: any = await this.proto.send({ moveBookmarkGroup: { groupId, afterGroupId } });
    return toBookmarkGroup(resp.moveBookmarkGroup.group);
  }

  async updateBookmarkGroup(groupId: string, name: string): Promise<BookmarkGroup> {
    const resp: any = await this.proto.send({ updateBookmarkGroup: { groupId, name } });
    return toBookmarkGroup(resp.updateBookmarkGroup.group);
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
