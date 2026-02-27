import { Component, inject, OnInit, effect, signal } from '@angular/core';
import { RecentPagesService } from '../../core/services/recent-pages.service';
import { CurrentPageService } from '../../core/services/current-page.service';
import { ApiService } from '../../core/services/api.service';
import { Page } from '../../core/models/page.model';
import { CalendarComponent } from '../../shared/calendar';
import { BookmarksSidebarComponent } from './bookmarks-sidebar';
import { formatJournalDate, formatDateToIso } from '../../core/utils/date.utils';
import { Router } from '@angular/router';
import { NavigationService } from '../../core/services/navigation.service';

@Component({
  selector: 'app-sidebar',
  imports: [CalendarComponent, BookmarksSidebarComponent],
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

        <div class="sidebar-section">
          <div class="sidebar-item journal-link" (click)="goToToday()">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#666" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="1" width="16" height="20" rx="1.5" ry="1.5"/>
              <line x1="3" y1="13" x2="19" y2="13" stroke="#666" stroke-width="1"/>
              <line x1="3" y1="17" x2="19" y2="17" stroke="#666" stroke-width="1"/>
              <line x1="14" y1="2" x2="14" y2="20" stroke="#666" stroke-width="3"/>
              <rect x="5" y="21" width="3" height="2" fill="#666" stroke="none"/>
            </svg>
            Journal
          </div>
        </div>

        @if (indexPages().length > 0) {
          <div class="sidebar-section">
            <h3 class="section-title section-title-link" (click)="goToIndex($event)">{{ indexHeader() }}</h3>
            @for (page of indexPages(); track page.id) {
              <div class="sidebar-item recent-page-item" (click)="goToPage(page, $event)"
                   [class.active]="page.id === currentPageService.currentPage()?.id">
                <svg class="page-icon" width="11" height="11"><use href="#icon-page"/></svg>
                {{ formatIndexPageName(page) }}
              </div>
            }
          </div>
        }

        <div class="sidebar-section">
          <h3 class="section-title">Recent Pages</h3>
          @for (page of recentPages.recentPages(); track page.id) {
            <div class="sidebar-item recent-page-item" (click)="goToPage(page, $event)">
              <svg class="page-icon" width="11" height="11"><use href="#icon-page"/></svg>
              {{ formatPageTitle(page) }}
            </div>
          }
        </div>

        <div class="sidebar-section">
          <app-bookmarks-sidebar />
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
    .journal-link { display: flex; align-items: center; gap: 6px; }
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
  private readonly router = inject(Router);
  private readonly nav = inject(NavigationService);
  readonly today = formatDateToIso(new Date());

  readonly indexHeader = signal('');
  readonly indexPages = signal<Page[]>([]);

  constructor() {
    effect(() => {
      const page = this.currentPageService.currentPage();
      if (!page || !page.dirPath) {
        this.indexHeader.set('');
        this.indexPages.set([]);
        return;
      }
      this.loadIndexPages(page);
    });
  }

  private async loadIndexPages(page: Page): Promise<void> {
    if (page.name === 'index') {
      // Index page: show its dirPath, list children
      this.indexHeader.set(page.dirPath);
      const children = await this.api.getChildPages(page.id);
      this.indexPages.set(children);
    } else if (page.parentId) {
      // Non-index page: show parent's dirPath, list siblings
      const parent = await this.api.getPageById(page.parentId);
      this.indexHeader.set(parent.page.dirPath);
      const siblings = await this.api.getChildPages(page.parentId);
      this.indexPages.set(siblings);
    } else {
      this.indexHeader.set('');
      this.indexPages.set([]);
    }
  }

  goToIndex(event: MouseEvent): void {
    const header = this.indexHeader();
    if (header) {
      this.nav.navigateToPage(header + 'index', event.ctrlKey || event.metaKey);
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

  /**
   * Navigates to the daily journal for the selected date.
   * @param event - Contains the ISO date string and whether Ctrl was held.
   */
  goToToday(): void {
    this.router.navigateByUrl('/today');
  }

  goToDate(event: { date: string; ctrlKey: boolean }): void {
    this.nav.navigateToPage(event.date, event.ctrlKey);
  }

  /**
   * Navigates to the weekly journal for the selected week (e.g., "2026-W08").
   * @param event - Contains the year, week number, and whether Ctrl was held.
   */
  goToWeek(event: { year: number; week: number; ctrlKey: boolean }): void {
    const fileName = `${event.year}-W${String(event.week).padStart(2, '0')}`;
    this.nav.navigateToPage(fileName, event.ctrlKey);
  }

  /**
   * Navigates to the monthly journal for the selected month (e.g., "2026-02").
   * @param event - Contains the year, month number, and whether Ctrl was held.
   */
  goToMonth(event: { year: number; month: number; ctrlKey: boolean }): void {
    const fileName = `${event.year}-${String(event.month).padStart(2, '0')}`;
    this.nav.navigateToPage(fileName, event.ctrlKey);
  }

  /**
   * Navigates to the yearly journal for the selected year (e.g., "2026").
   * @param event - Contains the year and whether Ctrl was held.
   */
  goToYear(event: { year: number; ctrlKey: boolean }): void {
    this.nav.navigateToPage(String(event.year), event.ctrlKey);
  }

  /**
   * Navigates to a recently visited page. Opens in a new tab if Ctrl/Meta is held.
   * @param page - The page to navigate to.
   * @param event - The mouse event, used to detect Ctrl/Meta key modifier.
   */
  goToPage(page: Page, event: MouseEvent): void {
    this.nav.navigateToPage(page.dirPath + page.name, event.ctrlKey || event.metaKey);
  }

  /**
   * Formats the display title for a page. Journals are formatted as human-readable dates;
   * documents show their directory path and file name.
   * @param page - The page to format the title for.
   * @returns The formatted display title string.
   */
  formatPageTitle(page: Page): string {
    if (page.type === 'journal') {
      return formatJournalDate(page.name);
    }
    if (page.name === 'index' && page.dirPath) {
      return page.dirPath;
    }
    return page.dirPath ? page.dirPath + page.name : page.name;
  }
}
