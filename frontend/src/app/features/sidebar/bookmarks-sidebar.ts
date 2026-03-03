import { Component, inject, signal, computed, ElementRef } from '@angular/core';
import { BookmarkStateService } from '../../core/services/bookmark-state.service';
import { CurrentPageService } from '../../core/services/current-page.service';
import { NavigationService } from '../../core/services/navigation.service';
import { Bookmark, BookmarkGroup } from '../../core/models/bookmark.model';
import { formatJournalDate } from '../../core/utils/date.utils';

@Component({
  selector: 'app-bookmarks-sidebar',
  template: `
    <div class="bookmarks-section">
      @for (group of bookmarkState.sortedGroups(); track group.id) {
        <div class="bookmark-group">
          <div
            class="group-header"
            [class.default-group-header]="group.name === ''"
            [draggable]="group.name !== ''"
            (dragstart)="onGroupDragStart($event, group)"
            (dragover)="onGroupDragOver($event, group)"
            (drop)="onGroupDrop($event, group)"
            (contextmenu)="onGroupContextMenu($event, group)"
          >
            <h3 class="group-name">{{ group.name === '' ? 'Bookmarks' : group.name }}</h3>
            <button
              class="add-btn"
              title="Add current page to this group"
              [disabled]="!currentPage.currentPage() || isPageInGroup(group.id)"
              (click)="addCurrentPage(group.id)"
            >+</button>
          </div>

          @for (bm of bookmarkState.bookmarksForGroup(group.id); track bm.id) {
            <div
              class="bookmark-item"
              [class.drop-above]="dropTarget()?.bookmarkId === bm.id && dropTarget()?.position === 'above'"
              [class.drop-below]="dropTarget()?.bookmarkId === bm.id && dropTarget()?.position === 'below'"
              draggable="true"
              (dragstart)="onBookmarkDragStart($event, bm)"
              (dragover)="onBookmarkDragOver($event, bm, group.id)"
              (drop)="onBookmarkDrop($event, group.id)"
              (dragleave)="onBookmarkDragLeave()"
              (click)="goToBookmark(bm, $event)"
              (contextmenu)="onBookmarkContextMenu($event, bm)"
            >
              <svg class="page-icon" width="11" height="11"><use href="#icon-page"/></svg>
              {{ formatBookmarkTitle(bm) }}
            </div>
          } @empty {
            <div
              class="empty-drop-zone"
              [class.drop-active]="emptyDropGroupId() === group.id"
              (dragover)="onEmptyGroupDragOver($event, group.id)"
              (dragleave)="onEmptyGroupDragLeave()"
              (drop)="onEmptyGroupDrop($event, group.id)"
            ></div>
          }
        </div>
      }

      @if (contextMenu()) {
        <div
          class="context-menu"
          [style.left.px]="contextMenu()!.x"
          [style.top.px]="contextMenu()!.y"
        >
          @if (contextMenu()!.type === 'bookmark') {
            <div class="context-menu-item" (mousedown)="removeBookmark()">Remove</div>
          }
          @if (contextMenu()!.type === 'group') {
            <div class="context-menu-item" (mousedown)="renameGroup()">Rename</div>
            <div class="context-menu-item" (mousedown)="deleteGroup()">Delete group</div>
          }
        </div>
      }

      <button class="new-group-btn" (click)="createNewGroup()">+ New group</button>
    </div>
  `,
  styles: [`
    .bookmarks-section { position: relative; }
    .bookmark-group { margin-bottom: 4px; }
    .group-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin: 0 0 8px 0;
      padding: 4px 12px;
      border-bottom: 1px dashed #999;
    }
    .default-group-header {
      background: #edeef0;
      border-radius: 4px;
      border-bottom: none;
    }
    .group-name {
      font-size: 11px;
      text-transform: uppercase;
      color: #555;
      margin: 0;
      letter-spacing: 0.5px;
      flex: 1;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      font-weight: bold;
    }
    .add-btn {
      background: none;
      border: none;
      color: #888;
      cursor: pointer;
      font-size: 14px;
      padding: 0 4px;
      line-height: 1;
      visibility: hidden;
    }
    .group-header:hover .add-btn { visibility: visible; }
    .add-btn:hover { color: #333; }
    .add-btn:disabled { color: #ccc; cursor: default; }
    .bookmark-item {
      padding: 6px 12px;
      cursor: pointer;
      border-radius: 4px;
      font-size: 13px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      display: flex;
      align-items: center;
      gap: 6px;
      border-top: 2px solid transparent;
      border-bottom: 2px solid transparent;
    }
    .bookmark-item:hover { background: #e9ecef; }
    .page-icon { flex-shrink: 0; color: #666; }
    .bookmark-item.drop-above { border-top-color: #4a9eff; }
    .bookmark-item.drop-below { border-bottom-color: #4a9eff; }
    .empty-drop-zone {
      min-height: 8px;
      border-top: 2px solid transparent;
      border-radius: 4px;
      margin: 0 4px;
    }
    .empty-drop-zone.drop-active {
      min-height: 24px;
      border-top-color: #4a9eff;
      background: rgba(74, 158, 255, 0.05);
    }
    .context-menu {
      position: fixed;
      background: white;
      border: 1px solid #ddd;
      border-radius: 4px;
      box-shadow: 0 2px 8px rgba(0,0,0,0.15);
      z-index: 1000;
      min-width: 120px;
    }
    .context-menu-item {
      padding: 6px 12px;
      cursor: pointer;
      font-size: 13px;
    }
    .context-menu-item:hover { background: #e9ecef; }
    .new-group-btn {
      background: none;
      border: none;
      color: #888;
      cursor: pointer;
      font-size: 11px;
      padding: 4px 12px;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }
    .new-group-btn:hover { color: #333; }
  `],
  host: {
    '(document:click)': 'closeContextMenu()',
  },
})
export class BookmarksSidebarComponent {
  readonly bookmarkState = inject(BookmarkStateService);
  readonly currentPage = inject(CurrentPageService);
  private readonly nav = inject(NavigationService);

  readonly contextMenu = signal<{
    type: 'bookmark' | 'group';
    x: number;
    y: number;
    bookmarkId?: string;
    groupId?: string;
  } | null>(null);

  readonly dropTarget = signal<{
    bookmarkId: string;
    position: 'above' | 'below';
  } | null>(null);

  readonly emptyDropGroupId = signal<string | null>(null);

  private draggedBookmark: Bookmark | null = null;
  private draggedGroup: BookmarkGroup | null = null;

  formatBookmarkTitle(bm: Bookmark): string {
    if (bm.targetPage) {
      if (bm.targetPage.type === 'journal') {
        return formatJournalDate(bm.targetPage.name);
      }
      if (bm.targetPage.name === 'index' && bm.targetPage.dirPath) {
        return bm.targetPage.dirPath;
      }
      return bm.targetPage.dirPath
        ? bm.targetPage.dirPath + bm.targetPage.name
        : bm.targetPage.name;
    }
    return bm.targetId;
  }

  isPageInGroup(groupId: string): boolean {
    const page = this.currentPage.currentPage();
    if (!page) return false;
    return this.bookmarkState.bookmarksForGroup(groupId)
      .some(b => b.targetType === 'page' && b.targetId === page.id);
  }

  async addCurrentPage(groupId: string): Promise<void> {
    const page = this.currentPage.currentPage();
    if (!page) return;
    await this.bookmarkState.addBookmark(groupId, 'page', page.id);
  }

  goToBookmark(bm: Bookmark, event: MouseEvent): void {
    if (!bm.targetPage) return;
    const path = bm.targetPage.dirPath + bm.targetPage.name;
    this.nav.navigateToPage(path, event.ctrlKey || event.metaKey, event.shiftKey);
  }

  // --- Context menu ---

  onBookmarkContextMenu(event: MouseEvent, bm: Bookmark): void {
    event.preventDefault();
    this.contextMenu.set({ type: 'bookmark', x: event.clientX, y: event.clientY, bookmarkId: bm.id });
  }

  onGroupContextMenu(event: MouseEvent, group: BookmarkGroup): void {
    if (group.name === '') return; // No context menu for default group
    event.preventDefault();
    this.contextMenu.set({ type: 'group', x: event.clientX, y: event.clientY, groupId: group.id });
  }

  closeContextMenu(): void {
    this.contextMenu.set(null);
  }

  async removeBookmark(): Promise<void> {
    const menu = this.contextMenu();
    if (!menu?.bookmarkId) return;
    this.contextMenu.set(null);
    await this.bookmarkState.removeBookmark(menu.bookmarkId);
  }

  async renameGroup(): Promise<void> {
    const menu = this.contextMenu();
    if (!menu?.groupId) return;
    this.contextMenu.set(null);
    const group = this.bookmarkState.sortedGroups().find(g => g.id === menu.groupId);
    const name = prompt('Group name:', group?.name ?? '');
    if (name === null) return;
    await this.bookmarkState.renameGroup(menu.groupId, name);
  }

  async deleteGroup(): Promise<void> {
    const menu = this.contextMenu();
    if (!menu?.groupId) return;
    this.contextMenu.set(null);
    if (!confirm('Delete this group and all its bookmarks?')) return;
    await this.bookmarkState.removeGroup(menu.groupId);
  }

  async createNewGroup(): Promise<void> {
    const name = prompt('Group name:');
    if (!name) return;
    await this.bookmarkState.addGroup(name);
  }

  // --- Drag and drop: bookmarks ---

  onBookmarkDragStart(event: DragEvent, bm: Bookmark): void {
    this.draggedBookmark = bm;
    this.draggedGroup = null;
    event.dataTransfer!.effectAllowed = 'move';
    event.dataTransfer!.setData('text/plain', bm.id);
  }

  onBookmarkDragOver(event: DragEvent, targetBm: Bookmark, groupId: string): void {
    if (!this.draggedBookmark || this.draggedBookmark.id === targetBm.id) return;
    event.preventDefault();
    event.dataTransfer!.dropEffect = 'move';

    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const midY = rect.top + rect.height / 2;
    const position = event.clientY < midY ? 'above' : 'below';
    this.dropTarget.set({ bookmarkId: targetBm.id, position });
  }

  onBookmarkDragLeave(): void {
    this.dropTarget.set(null);
  }

  async onBookmarkDrop(event: DragEvent, groupId: string): Promise<void> {
    event.preventDefault();
    const target = this.dropTarget();
    if (!this.draggedBookmark || !target) {
      this.cleanupDrag();
      return;
    }

    const groupBookmarks = this.bookmarkState.bookmarksForGroup(groupId);
    const targetIndex = groupBookmarks.findIndex(b => b.id === target.bookmarkId);
    if (targetIndex < 0) {
      this.cleanupDrag();
      return;
    }

    let afterBookmarkId = '';
    if (target.position === 'above') {
      // Place before target → afterBookmarkId = the one before target
      if (targetIndex > 0) {
        afterBookmarkId = groupBookmarks[targetIndex - 1].id;
      }
    } else {
      // Place after target
      afterBookmarkId = groupBookmarks[targetIndex].id;
    }

    const bookmarkId = this.draggedBookmark.id;
    this.cleanupDrag();
    await this.bookmarkState.moveBookmark(bookmarkId, groupId, afterBookmarkId);
  }

  // --- Drag and drop: empty group drop zone ---

  onEmptyGroupDragOver(event: DragEvent, groupId: string): void {
    if (!this.draggedBookmark) return;
    event.preventDefault();
    event.dataTransfer!.dropEffect = 'move';
    this.emptyDropGroupId.set(groupId);
  }

  onEmptyGroupDragLeave(): void {
    this.emptyDropGroupId.set(null);
  }

  async onEmptyGroupDrop(event: DragEvent, groupId: string): Promise<void> {
    event.preventDefault();
    if (!this.draggedBookmark) {
      this.cleanupDrag();
      return;
    }
    const bookmarkId = this.draggedBookmark.id;
    this.cleanupDrag();
    await this.bookmarkState.moveBookmark(bookmarkId, groupId, '');
  }

  // --- Drag and drop: groups ---

  onGroupDragStart(event: DragEvent, group: BookmarkGroup): void {
    if (group.name === '') {
      event.preventDefault();
      return;
    }
    this.draggedGroup = group;
    this.draggedBookmark = null;
    event.dataTransfer!.effectAllowed = 'move';
    event.dataTransfer!.setData('text/plain', group.id);
  }

  onGroupDragOver(event: DragEvent, targetGroup: BookmarkGroup): void {
    if (!this.draggedGroup || this.draggedGroup.id === targetGroup.id) return;
    event.preventDefault();
    event.dataTransfer!.dropEffect = 'move';
  }

  async onGroupDrop(event: DragEvent, targetGroup: BookmarkGroup): Promise<void> {
    event.preventDefault();
    if (!this.draggedGroup || this.draggedGroup.id === targetGroup.id) {
      this.cleanupDrag();
      return;
    }

    const groupId = this.draggedGroup.id;
    const afterGroupId = targetGroup.id;
    this.cleanupDrag();
    await this.bookmarkState.moveGroup(groupId, afterGroupId);
  }

  private cleanupDrag(): void {
    this.draggedBookmark = null;
    this.draggedGroup = null;
    this.dropTarget.set(null);
    this.emptyDropGroupId.set(null);
  }
}
