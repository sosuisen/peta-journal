import { Component, inject, OnInit } from '@angular/core';
import { RecentPagesService } from '../../core/services/recent-pages.service';
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

        <div class="sidebar-section">
          <h3 class="section-title">Recent Pages</h3>
          @for (page of recentPages.recentPages(); track page.id) {
            <div class="sidebar-item" (click)="goToPage(page, $event)">
              {{ formatPageTitle(page) }}
            </div>
          }
        </div>

        <div class="sidebar-section">
          <h3 class="section-title">Bookmarks</h3>
          <div class="sidebar-placeholder">No bookmarks yet</div>
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
      padding: 12px;
    }
    .sidebar-section { margin-bottom: 20px; }
    app-calendar { display: block; }
    .section-title {
      font-size: 11px;
      text-transform: uppercase;
      color: #888;
      margin: 0 0 8px 12px;
      letter-spacing: 0.5px;
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
  private readonly nav = inject(NavigationService);
  readonly today = formatDateToIso(new Date());

  /** Loads the recent pages list on component initialization. */
  ngOnInit(): void {
    this.recentPages.refresh();
  }

  /**
   * Navigates to the daily journal for the selected date.
   * @param event - Contains the ISO date string and whether Ctrl was held.
   */
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
    this.nav.navigateToPage(page.dirPath + page.fileName, event.ctrlKey || event.metaKey);
  }

  /**
   * Formats the display title for a page. Journals are formatted as human-readable dates;
   * documents show their directory path and file name.
   * @param page - The page to format the title for.
   * @returns The formatted display title string.
   */
  formatPageTitle(page: Page): string {
    if (page.type === 'journal') {
      return formatJournalDate(page.fileName);
    }
    return page.dirPath ? page.dirPath + page.fileName : page.fileName;
  }
}
