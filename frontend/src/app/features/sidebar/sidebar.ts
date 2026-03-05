import { Component, computed, inject, OnInit, effect, signal } from '@angular/core';
import { RecentPagesService } from '../../core/services/recent-pages.service';
import { CurrentPageService } from '../../core/services/current-page.service';
import { ApiService } from '../../core/services/api.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { Page } from '../../core/models/page.model';
import { CalendarComponent } from '../../shared/calendar';
import { formatJournalDate, formatDateToIso } from '../../core/utils/date.utils';
import { NavigationService } from '../../core/services/navigation.service';

@Component({
  selector: 'app-sidebar',
  imports: [CalendarComponent],
  template: `
    <nav class="sidebar">
      <div class="sidebar-content">
        <div class="sidebar-section">
          <app-calendar
            [selectedDate]="today"
            (dateSelected)="goToDate($event)"
            (weekClicked)="goToWeek($event)"
            (monthClicked)="goToMonth($event)"
            (yearClicked)="goToYear($event)"
          />
        </div>

        @if (indexPages().length > 0) {
          <div class="sidebar-section">
            <div class="section-header">
              <h3 class="section-title section-title-link" (click)="goToIndex($event)">{{ indexHeader() }}</h3>
              <button class="collapse-btn" (click)="toggleSection('index')" [attr.aria-label]="collapsed('index') ? 'Expand' : 'Collapse'">
                <svg width="10" height="12" viewBox="0 0 10 12">
                  @if (collapsed('index')) {
                    <polygon points="2,4.5 5,1.5 8,4.5" fill="#888"/>
                    <polygon points="2,7.5 5,10.5 8,7.5" fill="#888"/>
                  } @else {
                    <polygon points="2,2.5 5,5.5 8,2.5" fill="#888"/>
                    <polygon points="2,9.5 5,6.5 8,9.5" fill="#888"/>
                  }
                </svg>
              </button>
            </div>
            @if (!collapsed('index')) {
              <div class="index-page-list">
                @for (page of indexPages(); track page.id) {
                  <div class="sidebar-item recent-page-item" (click)="goToPage(page, $event)"
                       [class.active]="page.id === currentPageService.currentPage()?.id">
                    <svg class="page-icon" width="11" height="11"><use href="#icon-page"/></svg>
                    {{ formatIndexPageName(page) }}
                  </div>
                }
              </div>
            }
          </div>
        }

        @if (headings().length > 0) {
          <div class="sidebar-section">
            <div class="section-header">
              <h3 class="section-title">Contents</h3>
              <button class="collapse-btn" (click)="toggleSection('contents')" [attr.aria-label]="collapsed('contents') ? 'Expand' : 'Collapse'">
                <svg width="10" height="12" viewBox="0 0 10 12">
                  @if (collapsed('contents')) {
                    <polygon points="2,4.5 5,1.5 8,4.5" fill="#888"/>
                    <polygon points="2,7.5 5,10.5 8,7.5" fill="#888"/>
                  } @else {
                    <polygon points="2,2.5 5,5.5 8,2.5" fill="#888"/>
                    <polygon points="2,9.5 5,6.5 8,9.5" fill="#888"/>
                  }
                </svg>
              </button>
            </div>
            @if (!collapsed('contents')) {
              <div class="contents-list">
                @for (h of headings(); track h.id) {
                  <div class="contents-item"
                       [class]="'contents-h' + h.level"
                       (click)="scrollToItem(h.id)">
                    {{ h.text }}
                  </div>
                }
              </div>
            }
          </div>
        }

        <div class="sidebar-section">
          <div class="section-header">
            <h3 class="section-title">Recent Pages</h3>
            <button class="collapse-btn" (click)="toggleSection('recent')" [attr.aria-label]="collapsed('recent') ? 'Expand' : 'Collapse'">
              <svg width="10" height="12" viewBox="0 0 10 12">
                @if (collapsed('recent')) {
                  <polygon points="2,4.5 5,1.5 8,4.5" fill="#888"/>
                  <polygon points="2,7.5 5,10.5 8,7.5" fill="#888"/>
                } @else {
                  <polygon points="2,2.5 5,5.5 8,2.5" fill="#888"/>
                  <polygon points="2,9.5 5,6.5 8,9.5" fill="#888"/>
                }
              </svg>
            </button>
          </div>
          @if (!collapsed('recent')) {
            @for (page of recentPages.recentPages(); track page.id) {
              <div class="sidebar-item recent-page-item" (click)="goToPage(page, $event)">
                <svg class="page-icon" width="11" height="11"><use href="#icon-page"/></svg>
                {{ formatPageTitle(page) }}
              </div>
            }
          }
        </div>

      </div>

      <div class="sidebar-footer">
        <img src="/peta-journal-gray.png" alt="PetaJournal" class="footer-logo" />
        <span class="footer-text">PetaJournal</span>
      </div>
    </nav>
  `,
  styles: [`
    .sidebar {
      width: 100%;
      height: 100%;
      background: #f8f9fa;
      border-right: 1px solid #e0e0e0;
      box-sizing: border-box;
      display: flex;
      flex-direction: column;
    }
    .sidebar-content {
      flex: 1;
      overflow-y: auto;
      padding: 4px 12px 12px;
    }
    .sidebar-section { margin-bottom: 8px; }
    .section-header {
      display: flex;
      align-items: center;
    }
    .section-header .section-title {
      flex: 1;
      margin-bottom: 0;
    }
    .collapse-btn {
      background: none;
      border: none;
      cursor: pointer;
      padding: 4px 6px;
      border-radius: 4px;
      display: flex;
      align-items: center;
      opacity: 0.6;
    }
    .collapse-btn:hover { opacity: 1; background: #e0e2e5; }
    .index-page-list {}
    .contents-list {}
    .contents-item {
      padding: 3px 12px;
      cursor: pointer;
      border-radius: 4px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .contents-item:hover { background: #e9ecef; }
    .contents-h1 { font-size: 13px; font-weight: 600; }
    .contents-h2 { font-size: 13px; padding-left: 20px; }
    .contents-h3 { font-size: 12px; padding-left: 28px; }
    .contents-h4 { font-size: 12px; padding-left: 36px; color: #555; }
    .contents-h5 { font-size: 11px; padding-left: 44px; color: #555; }
    .contents-h6 { font-size: 11px; padding-left: 52px; color: #555; }
    app-calendar { display: block; }
    .section-title {
      font-size: 11px;
      text-transform: uppercase;
      color: #555;
      margin: 0 0 8px 0;
      padding: 4px 12px;
      letter-spacing: 0.5px;
      background: #edeef0;
      border-radius: 4px;
    }
    .section-title-link {
      cursor: pointer;
      text-transform: none;
    }
    .section-title-link:hover {
      background: #e0e2e5;
      color: #007bff;
    }
    .sidebar-item {
      padding: 6px 12px;
      cursor: pointer;
      border-radius: 4px;
      font-size: 13px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .sidebar-item:hover { background: #e9ecef; }
    .sidebar-item.active { background: #e2e6ea; font-weight: 600; }
    .recent-page-item { display: flex; align-items: center; gap: 6px; justify-content: flex-start; }
    .page-icon { flex-shrink: 0; color: #666; }
    .sidebar-placeholder {
      padding: 6px 12px;
      color: #aaa;
      font-size: 13px;
    }
    .sidebar-footer {
      padding: 12px;
      border-top: 1px solid #e0e0e0;
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .footer-logo {
      width: 24px;
      height: 24px;
      object-fit: contain;
    }
    .footer-text {
      font-size: 13px;
      color: #666;
      font-weight: 500;
    }
  `]
})
export class SidebarComponent implements OnInit {
  readonly recentPages = inject(RecentPagesService);
  readonly currentPageService = inject(CurrentPageService);
  private readonly api = inject(ApiService);
  private readonly outlineState = inject(OutlineStateService);
  private readonly nav = inject(NavigationService);
  readonly today = formatDateToIso(new Date());

  readonly indexHeader = signal('');
  readonly indexPages = signal<Page[]>([]);
  private readonly collapsedSections = signal<Set<string>>(this.loadCollapsedSections());

  readonly headings = computed(() => {
    const page = this.currentPageService.currentPage();
    if (!page || page.type !== 'doc') return [];
    return this.outlineState.headings();
  });

  constructor() {
    effect(() => {
      const page = this.currentPageService.currentPage();
      if (!page || page.type !== 'doc') {
        this.indexHeader.set('');
        this.indexPages.set([]);
        return;
      }
      this.loadIndexPages(page);
    });
  }

  private async loadIndexPages(page: Page): Promise<void> {
    if (page.name === 'index') {
      // Index page: show its children
      const isRoot = !page.dirPath;
      this.indexHeader.set(page.dirPath || '(root)/');
      const children = await this.api.getChildPages(page.id);
      this.indexPages.set(isRoot ? children.filter(c => c.type === 'doc') : children);
    } else if (page.parentId) {
      // Non-index page: show parent's children as siblings
      const parent = await this.api.getPageById(page.parentId);
      const isRoot = !parent.page.dirPath;
      this.indexHeader.set(parent.page.dirPath || '(root)/');
      const siblings = await this.api.getChildPages(page.parentId);
      this.indexPages.set(isRoot ? siblings.filter(c => c.type === 'doc') : siblings);
    } else {
      this.indexHeader.set('');
      this.indexPages.set([]);
    }
  }

  goToIndex(event: MouseEvent): void {
    const header = this.indexHeader();
    if (header === '(root)/') {
      this.nav.navigateToPage('index', event.ctrlKey || event.metaKey, event.shiftKey);
    } else if (header) {
      this.nav.navigateToPage(header + 'index', event.ctrlKey || event.metaKey, event.shiftKey);
    }
  }

  formatIndexPageName(page: Page): string {
    if (page.name === 'index') {
      // Show last segment of dirPath: "a/b/c/" → "c/"
      const trimmed = page.dirPath.endsWith('/') ? page.dirPath.slice(0, -1) : page.dirPath;
      const lastSlash = trimmed.lastIndexOf('/');
      return (lastSlash < 0 ? trimmed : trimmed.substring(lastSlash + 1)) + '/';
    }
    return page.name;
  }

  /** Loads the recent pages list on component initialization. */
  ngOnInit(): void {
    this.recentPages.refresh();
  }

  goToDate(event: { date: string; ctrlKey: boolean; shiftKey: boolean }): void {
    this.nav.navigateToPage(event.date, event.ctrlKey, event.shiftKey);
  }

  /**
   * Navigates to the weekly journal for the selected week (e.g., "2026-W08").
   * @param event - Contains the year, week number, and whether Ctrl was held.
   */
  goToWeek(event: { year: number; week: number; ctrlKey: boolean; shiftKey: boolean }): void {
    const fileName = `${event.year}-W${String(event.week).padStart(2, '0')}`;
    this.nav.navigateToPage(fileName, event.ctrlKey, event.shiftKey);
  }

  /**
   * Navigates to the monthly journal for the selected month (e.g., "2026-02").
   * @param event - Contains the year, month number, and whether Ctrl was held.
   */
  goToMonth(event: { year: number; month: number; ctrlKey: boolean; shiftKey: boolean }): void {
    const fileName = `${event.year}-${String(event.month).padStart(2, '0')}`;
    this.nav.navigateToPage(fileName, event.ctrlKey, event.shiftKey);
  }

  /**
   * Navigates to the yearly journal for the selected year (e.g., "2026").
   * @param event - Contains the year and whether Ctrl was held.
   */
  goToYear(event: { year: number; ctrlKey: boolean; shiftKey: boolean }): void {
    this.nav.navigateToPage(String(event.year), event.ctrlKey, event.shiftKey);
  }

  /**
   * Navigates to a recently visited page. Opens in a new tab if Ctrl/Meta is held.
   * @param page - The page to navigate to.
   * @param event - The mouse event, used to detect Ctrl/Meta key modifier.
   */
  goToPage(page: Page, event: MouseEvent): void {
    this.nav.navigateToPage(page.dirPath + page.name, event.ctrlKey || event.metaKey, event.shiftKey);
  }

  /**
   * Formats the display title for a page. Journals are formatted as human-readable dates;
   * documents show their directory path and file name.
   * @param page - The page to format the title for.
   * @returns The formatted display title string.
   */
  collapsed(section: string): boolean {
    return this.collapsedSections().has(section);
  }

  toggleSection(section: string): void {
    this.collapsedSections.update(set => {
      const next = new Set(set);
      if (next.has(section)) next.delete(section);
      else next.add(section);
      localStorage.setItem('sidebar-collapsed', JSON.stringify([...next]));
      return next;
    });
  }

  private loadCollapsedSections(): Set<string> {
    try {
      const stored = localStorage.getItem('sidebar-collapsed');
      return stored ? new Set(JSON.parse(stored)) : new Set();
    } catch {
      return new Set();
    }
  }

  scrollToItem(itemId: string): void {
    this.outlineState.requestScrollToItem(itemId);
  }

  formatPageTitle(page: Page): string {
    if (page.type === 'journal') {
      return formatJournalDate(page.name);
    }
    if (page.name === 'index') {
      return page.dirPath ? page.dirPath : '(root)/';
    }
    return page.dirPath ? page.dirPath + page.name : page.name;
  }
}
