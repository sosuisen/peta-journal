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
import { ConfigService } from '../../core/services/config.service';
import { weeksInYear } from '../../core/utils/week.utils';

/** How many items the outline keeps loaded ahead of the scroll position. */
const BUFFER_ITEMS = 100;

@Component({
  selector: 'app-journal',
  imports: [OutlineComponent, CalendarComponent],
  template: `
    <div class="journal-header" [class.scrolled]="scrolled()">
      <h2 class="journal-date" (click)="showCalendar.set(!showCalendar())">{{ displayDate() }}</h2>
      <button class="calendar-btn" (click)="showCalendar.set(!showCalendar())"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#666" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/></svg></button>
      @if (journalType() === 'daily') {
        <button class="nav-btn" (click)="goToPrevDay()">&lt;</button>
        <button class="nav-btn" (click)="goToNextDay()">&gt;</button>
      }
      @if (journalType() === 'weekly') {
        <button class="nav-btn" (click)="goToPrevWeek()">&lt;</button>
        <button class="nav-btn" (click)="goToNextWeek()">&gt;</button>
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
      <app-outline [pageId]="page()!.id" [disableAutoLoad]="true" [currentPageDate]="topVisibleDate()" (loadMore)="onLoadMore($event)" (topItemChanged)="onTopItemChanged($event)" (scrolledFromTop)="scrolled.set($event)" (crossDayMoved)="onCrossDayMoved($event)" />
    }
  `,
  styles: [`
    :host {
      display: flex;
      flex-direction: column;
      height: 100%;
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
    .journal-header.scrolled {
      border-bottom: 1px dashed #909090;
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
  private readonly config = inject(ConfigService);
  /** Names of the journals already appended to the outline. */
  private readonly loadedDates = new Set<string>();
  /** True while older journals are being fetched; a second request is dropped. */
  private loading = false;

  readonly currentDate = signal('');
  readonly journalType = signal<JournalType>('daily');
  readonly topVisibleDate = signal('');  // Date of the top visible item
  readonly page = signal<Page | null>(null);
  readonly showCalendar = signal(false);
  readonly scrolled = signal(false);

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

    // Manually load items first
    this.outlineState.setItems(result.items);
    if (result.transcludedTrees) {
      this.outlineState.setTranscludedTrees(result.transcludedTrees);
    }

    if (result.created) {
      this.extensionFunctions.emitPageCreate({
        pageId: result.page.id,
        fileName: date,
        dirName: result.page.dirPath,
        pageType: result.page.type
      });
    }

    // Pre-load previous journals to fill the buffer
    const type = this.journalType();
    if (type === 'daily' || type === 'weekly') {
      await this.loadPreviousJournals(date, BUFFER_ITEMS);
    }
  }

  /**
   * Appends older journals, one page at a time, until the outline holds at least
   * targetItemCount items or there is no older journal. Only one load runs at a time.
   * @param fromName - The journal to start searching backwards from
   * @param targetItemCount - The number of items the outline should hold afterwards
   */
  private async loadPreviousJournals(fromName: string, targetItemCount: number): Promise<void> {
    if (this.loading) return;
    this.loading = true;

    try {
      let name = fromName;
      while (this.outlineState.items().length < targetItemCount) {
        const result = await this.fetchPreviousJournal(name);
        if (!result.found) break;

        name = result.page!.name;
        if (this.loadedDates.has(name)) continue;
        this.loadedDates.add(name);

        this.outlineState.setItems([...this.outlineState.items(), ...result.items]);
        if (result.transcludedTrees) {
          this.outlineState.mergeTranscludedTrees(result.transcludedTrees);
        }
      }
    } catch (e) {
      console.error('Failed to load previous journals:', e);
    } finally {
      this.loading = false;
    }
  }

  /** Fetches the previous journal based on current journal type. */
  private fetchPreviousJournal(name: string) {
    if (this.journalType() === 'weekly') {
      return this.api.getMostRecentWeeklyJournalBefore(name);
    }
    return this.api.getMostRecentJournalBefore(name);
  }

  /**
   * Loads additional previous journal entries when the user scrolls near the end of the outline.
   * Continues loading until a buffer of 100 items beyond the current index is reached.
   * @param event - Contains the filepath of the bottom-most item and the current scroll index.
   */
  async onLoadMore(event: { filepath: string; currentIndex: number }): Promise<void> {
    if (this.loading) return;

    // Extract journal name from filepath based on type
    const journalName = this.extractJournalName(event.filepath);
    if (!journalName) {
      console.error('Could not extract journal name from filepath:', event.filepath);
      return;
    }

    await this.loadPreviousJournals(journalName, event.currentIndex + BUFFER_ITEMS);
  }

  /** Extracts the journal name from a item's filepath based on journal type. */
  private extractJournalName(filepath: string): string | null {
    if (this.journalType() === 'weekly') {
      const match = filepath.match(/(\d{4}-W\d{2})/);
      return match ? match[1] : null;
    }
    const match = filepath.match(/(\d{4}-\d{2}-\d{2})/);
    return match ? match[1] : null;
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

  /** Navigates to the journal for the previous week. */
  goToPrevWeek(): void {
    this.goToWeekOffset(-1);
  }

  /** Navigates to the journal for the next week. */
  goToNextWeek(): void {
    this.goToWeekOffset(1);
  }

  /**
   * Navigates one week back or forward from the current weekly journal.
   * The year boundary follows the number of weeks the year really has
   * under the configured week numbering (52 to 54), not a fixed 52.
   */
  private goToWeekOffset(delta: 1 | -1): void {
    const match = this.currentDate().match(/^(\d{4})-W(\d{2})$/);
    if (!match) return;
    let year = parseInt(match[1], 10);
    let week = parseInt(match[2], 10) + delta;
    const numbering = this.config.weekNumbering();
    if (week < 1) {
      year--;
      week = weeksInYear(year, numbering);
    } else if (week > weeksInYear(year, numbering)) {
      year++;
      week = 1;
    }
    this.nav.navigateToPage(`${year}-W${String(week).padStart(2, '0')}`);
  }

  /**
   * Navigates to the weekly journal for the selected week and hides the calendar.
   * @param event - Contains the year, week number, and whether Ctrl was held.
   */
  goToWeek(event: { year: number; week: number; ctrlKey: boolean; shiftKey: boolean }): void {
    if (!event.ctrlKey && !event.shiftKey) this.showCalendar.set(false);
    const fileName = `${event.year}-W${String(event.week).padStart(2, '0')}`;
    this.nav.navigateToPage(fileName, event.ctrlKey, event.shiftKey);
  }

  /**
   * Navigates to the monthly journal for the selected month and hides the calendar.
   * @param event - Contains the year, month number, and whether Ctrl was held.
   */
  goToMonth(event: { year: number; month: number; ctrlKey: boolean; shiftKey: boolean }): void {
    if (!event.ctrlKey && !event.shiftKey) this.showCalendar.set(false);
    const fileName = `${event.year}-${String(event.month).padStart(2, '0')}`;
    this.nav.navigateToPage(fileName, event.ctrlKey, event.shiftKey);
  }

  /**
   * Navigates to the yearly journal for the selected year and hides the calendar.
   * @param event - Contains the year and whether Ctrl was held.
   */
  goToYear(event: { year: number; ctrlKey: boolean; shiftKey: boolean }): void {
    if (!event.ctrlKey && !event.shiftKey) this.showCalendar.set(false);
    this.nav.navigateToPage(String(event.year), event.ctrlKey, event.shiftKey);
  }

  /**
   * Handles a date selection from the calendar picker and navigates to that daily journal.
   * @param event - Contains the ISO date string and whether Ctrl was held.
   */
  onDateSelected(event: { date: string; ctrlKey: boolean; shiftKey: boolean }): void {
    if (!event.ctrlKey && !event.shiftKey) this.showCalendar.set(false);
    this.nav.navigateToPage(event.date, event.ctrlKey, event.shiftKey);
  }

  /**
   * Updates the displayed date in the header when the top visible item changes during scrolling.
   * @param fileName - The file name (date string) of the top visible item's page.
   */
  onTopItemChanged(fileName: string): void {
    this.topVisibleDate.set(fileName);
  }

  /** Handles cross-day move: navigates to target date or reloads current journal. */
  async onCrossDayMoved(event: { targetDate: string; navigate: boolean; itemId: string }): Promise<void> {
    if (event.navigate) {
      this.nav.navigateToPage(event.targetDate);
      setTimeout(() => this.outlineState.requestEditItem(event.itemId), 300);
    } else {
      await this.loadJournal(this.currentDate());
      this.outlineState.requestEditItem(event.itemId);
    }
  }
}
