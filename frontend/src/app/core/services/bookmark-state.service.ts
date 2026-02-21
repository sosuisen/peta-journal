import { Injectable, inject, signal, computed, effect, DestroyRef } from '@angular/core';
import { ApiService } from './api.service';
import { WebSocketService } from './websocket.service';
import { Bookmark, BookmarkGroup } from '../models/bookmark.model';

@Injectable({ providedIn: 'root' })
export class BookmarkStateService {
  private readonly api = inject(ApiService);
  private readonly ws = inject(WebSocketService);
  private readonly destroyRef = inject(DestroyRef);

  private readonly _groups = signal<BookmarkGroup[]>([]);
  private readonly _bookmarks = signal<Bookmark[]>([]);

  readonly groups = this._groups.asReadonly();
  readonly bookmarks = this._bookmarks.asReadonly();

  /** Default group first, then named groups by sortOrder */
  readonly sortedGroups = computed(() => {
    const gs = this._groups();
    const defaultGroup = gs.find(g => g.name === '');
    const named = gs.filter(g => g.name !== '').sort((a, b) => a.sortOrder.localeCompare(b.sortOrder));
    return defaultGroup ? [defaultGroup, ...named] : named;
  });

  constructor() {
    effect(() => {
      if (this.ws.isConnected()) {
        this.refresh();
      }
    });
  }

  bookmarksForGroup(groupId: string): Bookmark[] {
    return this._bookmarks()
      .filter(b => b.groupId === groupId)
      .sort((a, b) => a.sortOrder.localeCompare(b.sortOrder));
  }

  isBookmarked(pageId: string): boolean {
    return this._bookmarks().some(b => b.targetType === 'page' && b.targetId === pageId);
  }

  findBookmarkByPageId(pageId: string): Bookmark | undefined {
    return this._bookmarks().find(b => b.targetType === 'page' && b.targetId === pageId);
  }

  defaultGroupId(): string | null {
    const g = this._groups().find(g => g.name === '');
    return g ? g.id : null;
  }

  async refresh(): Promise<void> {
    try {
      const result = await this.api.getBookmarks();
      this._groups.set(result.groups);
      this._bookmarks.set(result.bookmarks);
    } catch {
      // WebSocket may not be ready yet
    }
  }

  async addBookmark(groupId: string, targetType: string, targetId: string): Promise<void> {
    const bookmark = await this.api.createBookmark(groupId, targetType, targetId);
    this._bookmarks.update(bs => [...bs, bookmark]);
  }

  async removeBookmark(bookmarkId: string): Promise<void> {
    await this.api.deleteBookmark(bookmarkId);
    this._bookmarks.update(bs => bs.filter(b => b.id !== bookmarkId));
  }

  async addGroup(name: string): Promise<BookmarkGroup> {
    const group = await this.api.createBookmarkGroup(name);
    this._groups.update(gs => [...gs, group]);
    return group;
  }

  async removeGroup(groupId: string): Promise<void> {
    await this.api.deleteBookmarkGroup(groupId);
    this._groups.update(gs => gs.filter(g => g.id !== groupId));
    this._bookmarks.update(bs => bs.filter(b => b.groupId !== groupId));
  }

  async renameGroup(groupId: string, name: string): Promise<void> {
    const updated = await this.api.updateBookmarkGroup(groupId, name);
    this._groups.update(gs => gs.map(g => g.id === groupId ? updated : g));
  }

  async moveBookmark(bookmarkId: string, targetGroupId: string, afterBookmarkId: string): Promise<void> {
    const updated = await this.api.moveBookmark(bookmarkId, targetGroupId, afterBookmarkId);
    this._bookmarks.update(bs => bs.map(b => b.id === bookmarkId ? updated : b));
  }

  async moveGroup(groupId: string, afterGroupId: string): Promise<void> {
    const updated = await this.api.moveBookmarkGroup(groupId, afterGroupId);
    this._groups.update(gs => gs.map(g => g.id === groupId ? updated : g));
  }
}
