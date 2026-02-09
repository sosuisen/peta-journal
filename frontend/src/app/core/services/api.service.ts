import { Injectable, inject } from '@angular/core';
import { ProtobufService } from './protobuf.service';
import { Block } from '../models/block.model';
import { Page } from '../models/page.model';
import { SearchResult } from '../models/search.model';

interface GetPageResult { page: Page; blocks: Block[] }
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

  async getJournal(date: string): Promise<GetJournalResult> {
    const resp: any = await this.proto.send({ getJournal: { date } });
    const r = resp.getJournal;
    return { page: toPage(r.page), blocks: (r.blocks ?? []).map(toBlock), created: r.created ?? false };
  }

  async getMostRecentJournalBefore(beforeDate: string): Promise<GetMostRecentJournalBeforeResult> {
    const resp: any = await this.proto.send({ getMostRecentJournalBefore: { beforeDate } });
    const r = resp.getMostRecentJournalBefore;
    if (!r.found) {
      return { found: false, page: null, blocks: [] };
    }
    return { found: true, page: toPage(r.page), blocks: (r.blocks ?? []).map(toBlock) };
  }

  async createPage(title: string, dirPath?: string, tags?: string[]): Promise<Page> {
    const resp: any = await this.proto.send({ createPage: { title, dirPath, tags } });
    return toPage(resp.createPage.page);
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

  async getRecentPages(maxResults = 20): Promise<GetRecentPagesResult> {
    const resp: any = await this.proto.send({ getRecentPages: { maxResults } });
    return { pages: (resp.getRecentPages.pages ?? []).map(toPage) };
  }

  async updatePage(pageId: string, updates: { title?: string }): Promise<Page> {
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
}
