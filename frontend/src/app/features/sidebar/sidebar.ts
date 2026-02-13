import { Component, inject, OnInit } from '@angular/core';
import { Router } from '@angular/router';
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
            <div class="sidebar-item" (click)="goToPage(page)">
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
  private readonly router = inject(Router);
  private readonly nav = inject(NavigationService);
  readonly today = formatDateToIso(new Date());

  ngOnInit(): void {
    this.recentPages.refresh();
  }

  goToDate(date: string): void {
    this.router.navigate(['/journals', date]);
  }

  goToWeek(event: { year: number; week: number; ctrlKey: boolean }): void {
    const fileName = `${event.year}-W${String(event.week).padStart(2, '0')}`;
    this.nav.navigateToPage(fileName, event.ctrlKey);
  }

  goToMonth(event: { year: number; month: number; ctrlKey: boolean }): void {
    const fileName = `${event.year}-${String(event.month).padStart(2, '0')}`;
    this.nav.navigateToPage(fileName, event.ctrlKey);
  }

  goToYear(event: { year: number; ctrlKey: boolean }): void {
    this.nav.navigateToPage(String(event.year), event.ctrlKey);
  }

  goToPage(page: Page): void {
    this.nav.navigateToPage(page.dirPath + page.fileName);
  }

  formatPageTitle(page: Page): string {
    if (page.type === 'journal') {
      return formatJournalDate(page.fileName);
    }
    return page.dirPath ? page.dirPath + page.fileName : page.fileName;
  }
}
