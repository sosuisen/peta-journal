import { Component, inject, signal, computed, OnInit, OnDestroy } from '@angular/core';
import { Title } from '@angular/platform-browser';
import { ActivatedRoute, Router } from '@angular/router';
import { Subscription } from 'rxjs';
import { OutlineComponent } from '../outline/outline';
import { ApiService } from '../../core/services/api.service';
import { Page } from '../../core/models/page.model';
import { CalendarComponent } from '../../shared/calendar';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { formatJournalDate, formatDateToIso } from '../../core/utils/date.utils';

@Component({
  selector: 'app-journal',
  imports: [OutlineComponent, CalendarComponent],
  template: `
    <div class="journal-header">
      <h2 class="journal-date" (click)="showCalendar.set(!showCalendar())">{{ displayDate() }}</h2>
      <button class="calendar-btn" (click)="showCalendar.set(!showCalendar())">📅</button>
      <button class="nav-btn" (click)="goToPrevDay()">&lt;</button>
      <button class="nav-btn" (click)="goToNextDay()">&gt;</button>
    </div>
    @if (showCalendar()) {
      <app-calendar
        [selectedDate]="currentDate()"
        (dateSelected)="onDateSelected($event)"
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
      padding: 4px 8px;
      cursor: pointer;
      font-size: 18px;
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
export class JournalComponent implements OnInit, OnDestroy {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly api = inject(ApiService);
  private readonly title = inject(Title);
  private readonly outlineState = inject(OutlineStateService);
  private routeSub?: Subscription;
  private loadedDates = new Set<string>();
  private loading = false;

  readonly currentDate = signal('');
  readonly topVisibleDate = signal('');  // Date of the top visible block
  readonly page = signal<Page | null>(null);
  readonly showCalendar = signal(false);

  readonly displayDate = computed(() => {
    const d = this.topVisibleDate() || this.currentDate();
    if (!d) return '';
    return formatJournalDate(d);
  });

  ngOnInit(): void {
    this.routeSub = this.route.paramMap.subscribe(params => {
      let date = params.get('date');
      if (!date || date === 'today') {
        date = formatDateToIso(new Date());
      }
      this.currentDate.set(date);
      this.loadJournal(date);
    });
  }

  ngOnDestroy(): void {
    this.routeSub?.unsubscribe();
  }

  private async loadJournal(date: string): Promise<void> {
    const result = await this.api.getJournal(date);
    this.page.set(result.page);
    const todayStr = formatDateToIso(new Date());
    this.title.setTitle(date === todayStr ? 'Today' : date);
    this.topVisibleDate.set(date);  // Reset to current date
    this.loadedDates.clear();
    this.loadedDates.add(date);

    // Manually load today's blocks first
    this.outlineState.setBlocks(result.blocks);

    // Pre-load previous journals to fill the buffer
    await this.loadInitialBuffer(date);
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

  goToPrevDay(): void {
    const d = new Date(this.currentDate() + 'T00:00:00');
    d.setDate(d.getDate() - 1);
    this.router.navigate(['/journals', formatDateToIso(d)]);
  }

  goToNextDay(): void {
    const d = new Date(this.currentDate() + 'T00:00:00');
    d.setDate(d.getDate() + 1);
    this.router.navigate(['/journals', formatDateToIso(d)]);
  }

  onDateSelected(date: string): void {
    this.showCalendar.set(false);
    this.router.navigate(['/journals', date]);
  }

  onTopBlockChanged(fileName: string): void {
    this.topVisibleDate.set(fileName);
  }
}
