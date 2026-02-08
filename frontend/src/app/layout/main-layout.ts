import { Component, HostListener, inject, signal, viewChild } from '@angular/core';
import { Router, RouterOutlet } from '@angular/router';
import { SearchBarComponent } from '../features/search/search-bar';
import { SidebarComponent } from '../features/sidebar/sidebar';
import { WebSocketService } from '../core/services/websocket.service';
import { ApiService } from '../core/services/api.service';
import { RecentPagesService } from '../core/services/recent-pages.service';
import { OutlineStateService } from '../core/services/outline-state.service';

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
        </div>
        <div class="header-center">
          <button class="new-page-btn" title="New page" (click)="createNewPage()">+</button>
          <app-search-bar />
        </div>
        <div class="header-right">
          <div class="page-menu">
            <button class="page-menu-btn" (click)="pageMenuOpen.set(!pageMenuOpen())">&#8942;</button>
            @if (pageMenuOpen()) {
              <div class="page-menu-dropdown" (click)="pageMenuOpen.set(false)">
                <button class="page-menu-item" (click)="deletePage()">Delete page</button>
                <button class="page-menu-item" (click)="exportPage()">Export as Markdown</button>
              </div>
            }
          </div>
        </div>
      </header>
      <div class="body">
        <aside class="sidebar" [class.collapsed]="sidebarCollapsed()">
          <app-sidebar />
        </aside>
        <main class="content">
          <router-outlet />
        </main>
      </div>
      <footer class="footer">
        <span class="connection-status" [class]="ws.connectionState()">
          {{ ws.connectionState() }}
        </span>
        @if (outlineState.editingBlockId()) {
          <span class="block-id">Block: {{ outlineState.editingBlockId() }}</span>
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
    }
    .menu-btn {
      background: none;
      border: none;
      font-size: 18px;
      cursor: pointer;
      padding: 4px 8px;
      border-radius: 4px;
    }
    .menu-btn:hover { background: #f0f0f0; }
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
    .header-right { }
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
      width: 260px;
      flex-shrink: 0;
      overflow-y: auto;
      transition: width 0.2s, margin-left 0.2s;
    }
    .sidebar.collapsed {
      width: 0;
      overflow: hidden;
    }
    .content {
      flex: 1;
      overflow-y: auto;
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
    .block-id { margin-left: 16px; color: #888; }
  `]
})
export class MainLayoutComponent {
  readonly ws = inject(WebSocketService);
  readonly outlineState = inject(OutlineStateService);
  private readonly router = inject(Router);
  private readonly api = inject(ApiService);
  private readonly recentPages = inject(RecentPagesService);
  private readonly searchBar = viewChild(SearchBarComponent);
  readonly sidebarCollapsed = signal(false);
  readonly pageMenuOpen = signal(false);

  constructor() {
    this.ws.connect();
  }

  goHome(): void {
    this.router.navigate(['/journals/today']);
  }

  async createNewPage(): Promise<void> {
    const searchQuery = this.searchBar()?.query().trim();
    const title = searchQuery || prompt('Page title:');
    if (title) {
      const page = await this.api.createPage(title);
      this.recentPages.refresh();
      if (searchQuery) {
        this.searchBar()?.query.set('');
        this.searchBar()?.results.set([]);
        this.searchBar()?.showDropdown.set(false);
        this.searchBar()?.showCreateButton.set(false);
      }
      this.router.navigate(['/pages', page.id]);
    }
  }

  async deletePage(): Promise<void> {
    const url = this.router.url;
    const match = url.match(/\/pages\/(.+)/);
    if (!match) return;
    if (!confirm('Delete this page?')) return;
    await this.api.deletePage(match[1]);
    this.recentPages.refresh();
    this.router.navigate(['/journals/today']);
  }

  exportPage(): void {
    // TODO: trigger server-side export for current page
  }

  @HostListener('document:keydown', ['$event'])
  onKeydown(event: KeyboardEvent): void {
    const isEditing = !!this.outlineState.editingBlockId();
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
