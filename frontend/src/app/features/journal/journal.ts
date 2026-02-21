import { Component, inject, input, signal, computed, effect } from '@angular/core';
import { Title } from '@angular/platform-browser';
import { OutlineComponent } from '../outline/outline';
import { ApiService } from '../../core/services/api.service';
import { Page } from '../../core/models/page.model';
import { CalendarComponent } from '../../shared/calendar';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { formatJournalDate, formatDateToIso } from '../../core/utils/date.utils';
import { classifyJournalFileName, JournalType } from '../../core/utils/journal-filename.utils';
import { CurrentPageService } from '../../core/services/current-page.service';
import { RecentPagesService } from '../../core/services/recent-pages.service';
import { NavigationService } from '../../core/services/navigation.service';
import { ExtensionFunctionService } from '../../core/services/extension-function.service';

@Component({
  selector: 'app-journal',
  imports: [OutlineComponent, CalendarComponent],
  template: `
    <div class="journal-header">
      <h2 class="journal-date" (click)="showCalendar.set(!showCalendar())">{{ displayDate() }}</h2>
      <button class="calendar-btn" (click)="showCalendar.set(!showCalendar())"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#666" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/></svg></button>
      @if (journalType() === 'daily') {
        <button class="nav-btn" (click)="goToPrevDay()">&lt;</button>
        <button class="nav-btn" (click)="goToNextDay()">&gt;</button>
      }
    </div>
    @if (showCalendar()) {
      <app-calendar
        [selectedDate]="currentDate()"
        (dateSelected)="onDateSelected($event)"
        (weekClicked)="goToWeek($event)"
        (monthClicked)="goToMonth($event)"
        (yearClicked)="goToYear($event)"
      />
    }
    @if (page()) {
      <app-outline [pageId]="page()!.id" [disableAutoLoad]="true" [currentPageDate]="topVisibleDate()" (loadMore)="onLoadMore($event)" (topBlockChanged)="onTopBlockChanged($event)" />
    }
  `,
  styles: [`
    :host {
      display: flex;
      flex-direction: column;
      height: 100%;
      padding-right: 8px;
    }
    @media (min-width: 768px) {
      :host { padding-right: 24px; }
    }
    .journal-header {
      display: flex;
      align-items: center;
      gap: 4px;
      padding: 16px 0;
    }
    .calendar-btn {
      background: none;
      border: 1px solid #ddd;
      border-radius: 4px;
      padding: 4px 12px;
      cursor: pointer;
      font-size: 16px;
      display: flex;
      align-items: center;
      justify-content: center;
    }
    .calendar-btn:hover { background: #f0f0f0; }
    .nav-btn {
      background: none;
      border: 1px solid #ddd;
      border-radius: 4px;
      padding: 4px 12px;
      cursor: pointer;
      font-size: 16px;
    }
    .nav-btn:hover { background: #f0f0f0; }
    .journal-date {
      margin: 0;
      margin-right: 12px;
      cursor: pointer;
      user-select: none;
    }
  `]
})
export class JournalComponent {
  readonly pageName = input.required<string>();
  private readonly api = inject(ApiService);
  private readonly title = inject(Title);
  private readonly outlineState = inject(OutlineStateService);
  private readonly currentPageService = inject(CurrentPageService);
  private readonly recentPages = inject(RecentPagesService);
  private readonly nav = inject(NavigationService);
  private readonly extensionFunctions = inject(ExtensionFunctionService);
  private loadedDates = new Set<string>();
  private loading = false;

  readonly currentDate = signal('');
  readonly journalType = signal<JournalType>('daily');
  readonly topVisibleDate = signal('');  // Date of the top visible block
  readonly page = signal<Page | null>(null);
  readonly showCalendar = signal(false);

  readonly displayDate = computed(() => {
    const d = this.topVisibleDate() || this.currentDate();
    if (!d) return '';
    return formatJournalDate(d);
  });

  constructor() {
    effect(() => {
      let date = this.pageName();
      if (!date || date === 'today') {
        date = formatDateToIso(new Date());
      }
      this.currentDate.set(date);
      this.journalType.set(classifyJournalFileName(date));
      this.loadJournal(date);
    });
  }

  private async loadJournal(date: string): Promise<void> {
    const result = await this.api.getOrCreateJournal(date);
    this.page.set(result.page);
    this.currentPageService.setCurrentPage(result.page);
    if (result.created) {
      this.recentPages.refresh();
    }
    const todayStr = formatDateToIso(new Date());
    this.title.setTitle(date === todayStr ? 'Today' : formatJournalDate(date));
    this.topVisibleDate.set(date);  // Reset to current date
    this.loadedDates.clear();
    this.loadedDates.add(date);

    // Manually load blocks first
    this.outlineState.setBlocks(result.blocks);

    if (result.created) {
      this.extensionFunctions.emitPageCreate({
        pageId: result.page.id,
        fileName: date,
        dirName: result.page.dirPath
      });
    }

    // Pre-load previous journals to fill the buffer (daily only)
    if (this.journalType() === 'daily') {
      await this.loadInitialBuffer(date);
    }
  }

  private async loadInitialBuffer(currentDate: string): Promise<void> {
    if (this.loading) return;
    this.loading = true;

    try {
      let dateStr = currentDate;
      const targetBufferSize = 100;

      while (this.outlineState.blocks().length < targetBufferSize) {
        const result = await this.api.getMostRecentJournalBefore(dateStr);

        if (!result.found) {
          break;
        }

        if (this.loadedDates.has(result.page!.fileName)) {
          dateStr = result.page!.fileName;
          continue;
        }

        this.loadedDates.add(result.page!.fileName);
        const currentBlocks = this.outlineState.blocks();
        this.outlineState.setBlocks([...currentBlocks, ...result.blocks]);

        dateStr = result.page!.fileName;
      }
    } catch (e) {
      console.error('Failed to load initial buffer:', e);
    } finally {
      this.loading = false;
    }
  }

  /**
   * Loads additional previous journal entries when the user scrolls near the end of the outline.
   * Continues loading until a buffer of 100 blocks beyond the current index is reached.
   * @param event - Contains the filepath of the bottom-most block and the current scroll index.
   */
  async onLoadMore(event: { filepath: string; currentIndex: number }): Promise<void> {
    if (this.loading) return;

    this.loading = true;

    try {
      // Extract date from filepath (e.g., "journals2026-02-08" → "2026-02-08")
      const match = event.filepath.match(/(\d{4}-\d{2}-\d{2})/);
      if (!match) {
        console.error('Could not extract date from filepath:', event.filepath);
        return;
      }

      let currentDateStr = match[1];

      // Keep loading until we have 100 blocks of buffer
      while (this.outlineState.blocks().length < event.currentIndex + 100) {
        const result = await this.api.getMostRecentJournalBefore(currentDateStr);

        if (!result.found) {
          break;
        }

        if (this.loadedDates.has(result.page!.fileName)) {
          currentDateStr = result.page!.fileName;
          continue;
        }

        this.loadedDates.add(result.page!.fileName);

        // Append blocks from this journal
        const currentBlocks = this.outlineState.blocks();
        this.outlineState.setBlocks([...currentBlocks, ...result.blocks]);

        currentDateStr = result.page!.fileName;
      }
    } catch (e) {
      console.error('Failed to load previous days:', e);
    } finally {
      this.loading = false;
    }
  }

  /** Navigates to the journal for the previous day. */
  goToPrevDay(): void {
    const d = new Date(this.currentDate() + 'T00:00:00');
    d.setDate(d.getDate() - 1);
    this.nav.navigateToPage(formatDateToIso(d));
  }

  /** Navigates to the journal for the next day. */
  goToNextDay(): void {
    const d = new Date(this.currentDate() + 'T00:00:00');
    d.setDate(d.getDate() + 1);
    this.nav.navigateToPage(formatDateToIso(d));
  }

  /**
   * Navigates to the weekly journal for the selected week and hides the calendar.
   * @param event - Contains the year, week number, and whether Ctrl was held.
   */
  goToWeek(event: { year: number; week: number; ctrlKey: boolean }): void {
    if (!event.ctrlKey) this.showCalendar.set(false);
    const fileName = `${event.year}-W${String(event.week).padStart(2, '0')}`;
    this.nav.navigateToPage(fileName, event.ctrlKey);
  }

  /**
   * Navigates to the monthly journal for the selected month and hides the calendar.
   * @param event - Contains the year, month number, and whether Ctrl was held.
   */
  goToMonth(event: { year: number; month: number; ctrlKey: boolean }): void {
    if (!event.ctrlKey) this.showCalendar.set(false);
    const fileName = `${event.year}-${String(event.month).padStart(2, '0')}`;
    this.nav.navigateToPage(fileName, event.ctrlKey);
  }

  /**
   * Navigates to the yearly journal for the selected year and hides the calendar.
   * @param event - Contains the year and whether Ctrl was held.
   */
  goToYear(event: { year: number; ctrlKey: boolean }): void {
    if (!event.ctrlKey) this.showCalendar.set(false);
    this.nav.navigateToPage(String(event.year), event.ctrlKey);
  }

  /**
   * Handles a date selection from the calendar picker and navigates to that daily journal.
   * @param event - Contains the ISO date string and whether Ctrl was held.
   */
  onDateSelected(event: { date: string; ctrlKey: boolean }): void {
    if (!event.ctrlKey) this.showCalendar.set(false);
    this.nav.navigateToPage(event.date, event.ctrlKey);
  }

  /**
   * Updates the displayed date in the header when the top visible block changes during scrolling.
   * @param fileName - The file name (date string) of the top visible block's page.
   */
  onTopBlockChanged(fileName: string): void {
    this.topVisibleDate.set(fileName);
  }
}
