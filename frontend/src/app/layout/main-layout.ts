import { Component, HostListener, inject, signal, viewChild } from '@angular/core';
import { Router, RouterOutlet } from '@angular/router';
import { SearchBarComponent } from '../features/search/search-bar';
import { SidebarComponent } from '../features/sidebar/sidebar';
import { SseService } from '../core/services/sse.service';
import { ApiService } from '../core/services/api.service';
import { RecentPagesService } from '../core/services/recent-pages.service';
import { NavigationService } from '../core/services/navigation.service';
import { OutlineStateService } from '../core/services/outline-state.service';
import { ConfigService } from '../core/services/config.service';
import { generateUntitledTitle } from '../core/utils/title.utils';
import { classifyJournalFileName } from '../core/utils/journal-filename.utils';
import { validateFileName } from '../core/utils/filename-validation.utils';

@Component({
  selector: 'app-main-layout',
  imports: [RouterOutlet, SearchBarComponent, SidebarComponent],
  template: `
    <div class="layout" [class.sidebar-collapsed]="sidebarCollapsed()">
      <header class="header">
        <div class="header-left">
          <button class="menu-btn" (click)="sidebarCollapsed.set(!sidebarCollapsed())">
            &#9776;
          </button>
          <button class="journal-btn" title="Today" (click)="navigateToToday()">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#666" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="1" width="16" height="20" rx="1.5" ry="1.5"/>
              <line x1="3" y1="13" x2="19" y2="13" stroke="#666" stroke-width="1"/>
              <line x1="3" y1="17" x2="19" y2="17" stroke="#666" stroke-width="1"/>
              <line x1="14" y1="2" x2="14" y2="20" stroke="#666" stroke-width="3"/>
              <rect x="5" y="21" width="3" height="2" fill="#666" stroke="none"/>
            </svg>
          </button>
          <button class="root-btn" title="Root" (click)="navigateToRoot()">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#666" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M3 7v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-6l-2-2H5a2 2 0 0 0-2 2z"/>
            </svg>
          </button>
        </div>
        <div class="header-center">
          <button class="new-page-btn" title="New page" (click)="createNewPage()">+</button>
          <app-search-bar />
        </div>
        <div class="header-right">
          <button class="menu-btn home-btn" title="Home" (click)="navigateToToday()">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#666" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>
          </button>
          <div class="page-menu">
            <button class="page-menu-btn" (click)="pageMenuOpen.set(!pageMenuOpen())">&#8942;</button>
            @if (pageMenuOpen()) {
              <div class="page-menu-dropdown" (click)="pageMenuOpen.set(false)">
                <button class="page-menu-item" (click)="deletePage()">Delete page</button>
              </div>
            }
          </div>
        </div>
      </header>
      <div class="body">
        <aside class="sidebar" [class.collapsed]="sidebarCollapsed()" [style.width.px]="sidebarCollapsed() ? 0 : config.sidebarWidth()">
          <app-sidebar />
        </aside>
        @if (!sidebarCollapsed()) {
          <div class="resize-handle" (mousedown)="startSidebarResize($event)"></div>
        }
        <main class="content">
          <router-outlet />
        </main>
      </div>
      <footer class="footer">
        <span class="connection-status" [class]="sse.connectionState()">
          {{ sse.connectionState() }}
        </span>
        @if (outlineState.focusedItemId()) {
          <span class="item-id">Item: {{ outlineState.focusedItemId() }}</span>
        }
      </footer>
    </div>
  `,
  styles: [`
    .layout {
      display: flex;
      flex-direction: column;
      height: 100vh;
      width: 100vw;
    }
    .header {
      display: flex;
      align-items: center;
      gap: 16px;
      padding: 8px 16px;
      border-bottom: 1px solid #e0e0e0;
      background: white;
      z-index: 10;
      height: 48px;
      box-sizing: border-box;
    }
    .header-left {
      display: flex;
      align-items: center;
      gap: 2px;
    }
    .journal-btn {
      background: none;
      border: none;
      cursor: pointer;
      padding: 4px 8px;
      border-radius: 4px;
      display: flex;
      align-items: center;
      position: relative;
      top: 2px;
    }
    .journal-btn:hover { background: #f0f0f0; }
    .root-btn {
      background: none;
      border: none;
      cursor: pointer;
      padding: 4px 8px;
      border-radius: 4px;
      display: flex;
      align-items: center;
      position: relative;
      top: 2px;
    }
    .root-btn:hover { background: #f0f0f0; }
    .menu-btn {
      background: none;
      border: none;
      font-size: 18px;
      cursor: pointer;
      padding: 4px 8px;
      border-radius: 4px;
    }
    .menu-btn:hover { background: #f0f0f0; }
    .home-btn { display: flex; align-items: center; justify-content: center; position: relative; top: 1px; }
    .header-center {
      flex: 1;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
    }
    .new-page-btn {
      background: #007bff;
      color: white;
      border: none;
      border-radius: 6px;
      width: 32px;
      height: 32px;
      font-size: 18px;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
    }
    .new-page-btn:hover { background: #0056b3; }
    .header-right { display: flex; align-items: center; }
    .page-menu { position: relative; }
    .page-menu-btn {
      background: none;
      border: none;
      font-size: 20px;
      cursor: pointer;
      padding: 4px 8px;
      border-radius: 4px;
      line-height: 1;
    }
    .page-menu-btn:hover { background: #f0f0f0; }
    .page-menu-dropdown {
      position: absolute;
      right: 0;
      top: 100%;
      background: white;
      border: 1px solid #e0e0e0;
      border-radius: 6px;
      box-shadow: 0 4px 12px rgba(0,0,0,0.1);
      min-width: 180px;
      z-index: 100;
      padding: 4px 0;
    }
    .page-menu-item {
      display: block;
      width: 100%;
      text-align: left;
      padding: 8px 16px;
      border: none;
      background: none;
      cursor: pointer;
      font-size: 13px;
    }
    .page-menu-item:hover { background: #f0f0f0; }
    .body {
      display: flex;
      flex: 1;
      overflow: hidden;
    }
    .sidebar {
      flex-shrink: 0;
      overflow-y: auto;
      transition: width 0.2s;
      min-width: 200px;
      max-width: 600px;
    }
    .sidebar.collapsed {
      width: 0 !important;
      min-width: 0;
      overflow: hidden;
    }
    .resize-handle {
      width: 4px;
      cursor: col-resize;
      background: transparent;
      flex-shrink: 0;
      position: relative;
    }
    .resize-handle:hover {
      background: #007bff;
    }
    .resize-handle:active {
      background: #0056b3;
    }
    .content {
      flex: 1;
      overflow: hidden;
      padding: 0 0 0 8px;
      box-sizing: border-box;
    }
    @media (min-width: 768px) {
      .content { padding-left: 24px; }
    }
    .footer {
      display: flex;
      align-items: center;
      padding: 2px 16px;
      border-top: 1px solid #e0e0e0;
      background: #fafafa;
      height: 24px;
      box-sizing: border-box;
      font-size: 11px;
    }
    .connection-status.connected { color: #4caf50; }
    .connection-status.connecting { color: #ff9800; }
    .connection-status.disconnected { color: #999; }
    .connection-status.error { color: #f44336; }
    .item-id { margin-left: 16px; color: #888; }
  `]
})
export class MainLayoutComponent {
  readonly sse = inject(SseService);
  readonly outlineState = inject(OutlineStateService);
  readonly config = inject(ConfigService);
  private readonly router = inject(Router);
  private readonly api = inject(ApiService);
  private readonly nav = inject(NavigationService);
  private readonly recentPages = inject(RecentPagesService);
  private readonly searchBar = viewChild(SearchBarComponent);
  readonly sidebarCollapsed = signal(false);
  readonly pageMenuOpen = signal(false);
  private isResizing = false;
  private startX = 0;
  private startWidth = 0;

  constructor() {
    this.sse.connect();
  }

  /**
   * Initiates sidebar width resizing via mouse drag. Attaches mousemove/mouseup listeners
   * to handle the drag and cleans up on release.
   * @param event - The mousedown event on the resize handle.
   */
  startSidebarResize(event: MouseEvent): void {
    event.preventDefault();
    this.isResizing = true;
    this.startX = event.clientX;
    this.startWidth = this.config.sidebarWidth();

    const onMouseMove = (e: MouseEvent) => {
      if (!this.isResizing) return;
      const delta = e.clientX - this.startX;
      const newWidth = Math.max(200, Math.min(600, this.startWidth + delta));
      this.config.setSidebarWidth(newWidth);
    };

    const onMouseUp = () => {
      this.isResizing = false;
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
    };

    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);
  }

  /** Navigates to today's daily journal. */
  navigateToToday(): void {
    this.router.navigateByUrl('/today');
  }

  navigateToRoot(): void {
    this.nav.navigateToPage('index');
  }

  /**
   * Creates a new document page. Uses the current search query as the title if available,
   * otherwise prompts the user. Generates a timestamp-based title if left empty.
   */
  async createNewPage(): Promise<void> {
    const searchQuery = this.searchBar()?.query().trim();
    let title = searchQuery || prompt('Page title:');

    if (title !== null) {
      // Trim leading and trailing whitespace
      title = title.trim();

      // If empty or whitespace-only, generate timestamp-based title
      if (!title) {
        title = generateUntitledTitle();
      }

      if (searchQuery) {
        this.searchBar()?.clearSearch();
      }
      if (classifyJournalFileName(title) === 'none') {
        const error = validateFileName(title);
        if (error) { alert(error); return; }
      }
      this.nav.navigateToPage(title);
    }
  }

  /**
   * Deletes the currently viewed page after user confirmation, then navigates to today's journal.
   */
  async deletePage(): Promise<void> {
    const url = this.router.url;
    const match = url.match(/\/(?:pages|docs|journals)\/(.+)/);
    const fileName = match ? decodeURIComponent(match[1]) : null;
    if (!fileName || fileName === 'today') return;
    if (!confirm(`Delete "${fileName}"?`)) return;
    const isJournal = classifyJournalFileName(fileName) !== 'none';
    const result = isJournal
      ? await this.api.getOrCreateJournal(fileName)
      : await this.api.getOrCreateDoc(fileName);
    try {
      await this.api.deletePage(result.page.id);
    } catch (e: any) {
      alert(e.message || 'Failed to delete page');
      return;
    }
    this.recentPages.refresh();
    this.router.navigateByUrl('/today');
  }

  /**
   * Global keyboard shortcut handler. Focuses the search bar on Ctrl+/ or bare `/` key press.
   * @param event - The keyboard event.
   */
  @HostListener('document:keydown', ['$event'])
  onKeydown(event: KeyboardEvent): void {
    const isEditing = !!this.outlineState.focusedItemId();
    if (event.key === '/' && event.ctrlKey) {
      event.preventDefault();
      this.searchBar()?.focusInput();
    } else if (event.key === '/' && !isEditing && !event.ctrlKey && !event.altKey && !event.metaKey) {
      const tag = (event.target as HTMLElement)?.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA') return;
      event.preventDefault();
      this.searchBar()?.focusInput();
    }
  }
}
