import { Injectable, inject } from '@angular/core';
import { ProtobufService } from './protobuf.service';
import { Block } from '../models/block.model';
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

function toPage(p: any): Page {
  return {
    id: p.id ?? '',
    dirPath: p.dirPath ?? '',
    fileName: p.fileName ?? '',
    type: p.type ?? 'doc',
    createdDate: p.createdDate?.millis?.toNumber?.() ?? Number(p.createdDate?.millis ?? 0),
    modifiedDate: p.modifiedDate?.millis?.toNumber?.() ?? Number(p.modifiedDate?.millis ?? 0),
    tags: p.tags ?? [],
    propertiesJson: p.propertiesJson ?? '',
  };
}

function toBlock(b: any): Block {
  return {
    id: b.id ?? '',
    pageId: b.pageId ?? '',
    dirPath: b.dirPath ?? '',
    fileName: b.fileName ?? '',
    type: b.type ?? 'doc',
    createdDate: b.createdDate?.millis?.toNumber?.() ?? Number(b.createdDate?.millis ?? 0),
    modifiedDate: b.modifiedDate?.millis?.toNumber?.() ?? Number(b.modifiedDate?.millis ?? 0),
    idx: b.idx ?? '',
    level: b.level ?? 0,
    body: b.body ?? '',
    html: b.html ?? '',
    collapsed: b.collapsed ?? false,
    status: b.status ?? 'normal',
    types: b.types ?? [],
    propertiesJson: b.propertiesJson ?? '',
  };
}

@Injectable({ providedIn: 'root' })
export class ApiService {
  private readonly proto = inject(ProtobufService);

  async getPage(pageId: string): Promise<GetPageResult> {
    const resp: any = await this.proto.send({ getPage: { pageId } });
    const r = resp.getPage;
    return { page: toPage(r.page), blocks: (r.blocks ?? []).map(toBlock) };
  }

  async getMostRecentJournalBefore(beforeDate: string): Promise<GetMostRecentJournalBeforeResult> {
    const resp: any = await this.proto.send({ getMostRecentJournalBefore: { beforeDate } });
    const r = resp.getMostRecentJournalBefore;
    if (!r.found) {
      return { found: false, page: null, blocks: [] };
    }
    return { found: true, page: toPage(r.page), blocks: (r.blocks ?? []).map(toBlock) };
  }

  async getJournalByName(fileName: string): Promise<GetJournalResult> {
    const resp: any = await this.proto.send({ getJournalByName: { fileName } });
    const r = resp.getJournalByName;
    return { page: toPage(r.page), blocks: (r.blocks ?? []).map(toBlock), created: r.created ?? false };
  }

  async getDocByName(title: string, dirPath?: string, tags?: string[]): Promise<GetDocResult> {
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
    const resp: any = await this.proto.send({ getDocByName: { title, dirPath, tags } });
    const r = resp.getDocByName;
    return { page: toPage(r.page), blocks: (r.blocks ?? []).map(toBlock), created: r.created ?? false };
  }

  async createBlock(pageId: string, afterBlockId: string, level: number, body: string): Promise<Block> {
    const resp: any = await this.proto.send({ createBlock: { pageId, afterBlockId, level, body } });
    return toBlock(resp.createBlock.block);
  }

  async updateBlock(blockId: string, updates: { body?: string; level?: number; collapsed?: boolean; propertiesJson?: string }): Promise<Block> {
    const resp: any = await this.proto.send({ updateBlock: { blockId, ...updates } });
    return toBlock(resp.updateBlock.block);
  }

  async moveBlocks(blockIds: string[], targetPageId: string, afterBlockId: string, levelDelta?: number): Promise<Block[]> {
    const resp: any = await this.proto.send({ moveBlocks: { blockIds, targetPageId, afterBlockId, levelDelta } });
    return (resp.moveBlocks.blocks ?? []).map(toBlock);
  }

  async deletePage(pageId: string): Promise<void> {
    await this.proto.send({ deletePage: { pageId } });
  }

  async deleteBlock(blockId: string): Promise<void> {
    await this.proto.send({ deleteBlock: { blockId } });
  }

  async updateBlockStatus(blockId: string, status: string): Promise<Block> {
    const resp: any = await this.proto.send({ updateBlockStatus: { blockId, status } });
    return toBlock(resp.updateBlockStatus.block);
  }

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

  async getRecentPages(maxResults = 10, type?: string): Promise<GetRecentPagesResult> {
    const resp: any = await this.proto.send({ getRecentPages: { maxResults, type: type ?? '' } });
    return { pages: (resp.getRecentPages.pages ?? []).map(toPage) };
  }

  async updatePage(pageId: string, updates: { title?: string; dirPath?: string }): Promise<Page> {
    const resp: any = await this.proto.send({ updatePage: { pageId, ...updates } });
    return toPage(resp.updatePage.page);
  }

  async getBacklinks(pageId: string): Promise<GetBacklinksResult> {
    const resp: any = await this.proto.send({ getBacklinks: { pageId } });
    return {
      entries: (resp.getBacklinks.entries ?? []).map((e: any) => ({
        sourcePage: toPage(e.sourcePage),
        sourceBlock: toBlock(e.sourceBlock),
      })),
    };
  }

  async getConfig(): Promise<{ weekNumbering: string }> {
    const resp: any = await this.proto.send({ getConfig: {} });
    const r = resp.getConfig;
    return { weekNumbering: r.weekNumbering ?? 'us' };
  }

  async updateConfig(weekNumbering: string): Promise<{ weekNumbering: string }> {
    const resp: any = await this.proto.send({ updateConfig: { weekNumbering } });
    const r = resp.updateConfig;
    return { weekNumbering: r.weekNumbering ?? 'us' };
  }

  async exportTreeMarkdown(blockIds: string[]): Promise<{ treeMarkdown: string; html: string; flatMarkdown: string }> {
    const resp: any = await this.proto.send({ exportTreeMarkdown: { blockIds } });
    const r = resp.exportTreeMarkdown;
    return { treeMarkdown: r.treeMarkdown ?? '', html: r.html ?? '', flatMarkdown: r.flatMarkdown ?? '' };
  }

  async importTreeMarkdown(treeMarkdown: string): Promise<{ flatMarkdown: string }> {
    const resp: any = await this.proto.send({ importTreeMarkdown: { treeMarkdown } });
    const r = resp.importTreeMarkdown;
    return { flatMarkdown: r.flatMarkdown ?? '' };
  }
}
