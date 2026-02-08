import { Component, inject, signal, computed, OnInit, OnDestroy } from '@angular/core';
import { Title } from '@angular/platform-browser';
import { ActivatedRoute, Router } from '@angular/router';
import { Subscription } from 'rxjs';
import { OutlineComponent } from '../outline/outline';
import { ApiService } from '../../core/services/api.service';
import { Page } from '../../core/models/page.model';
import { CalendarComponent } from '../../shared/calendar';

@Component({
  selector: 'app-journal',
  imports: [OutlineComponent, CalendarComponent],
  template: `
    <div class="journal-header">
      <button class="nav-btn" (click)="goToPrevDay()">&lt;</button>
      <h2 class="journal-date" (click)="showCalendar.set(!showCalendar())">{{ displayDate() }}</h2>
      <button class="nav-btn" (click)="goToNextDay()">&gt;</button>
    </div>
    @if (showCalendar()) {
      <app-calendar
        [selectedDate]="currentDate()"
        (dateSelected)="onDateSelected($event)"
      />
    }
    @if (page()) {
      <app-outline [pageId]="page()!.id" />
    }
  `,
  styles: [`
    :host { display: flex; flex-direction: column; height: 100%; }
    .journal-header {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 16px 0;
    }
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
  private routeSub?: Subscription;

  readonly currentDate = signal('');
  readonly page = signal<Page | null>(null);
  readonly showCalendar = signal(false);

  readonly displayDate = computed(() => {
    const d = this.currentDate();
    if (!d) return '';
    try {
      return new Date(d + 'T00:00:00').toLocaleDateString('en-US', {
        weekday: 'long', year: 'numeric', month: 'long', day: 'numeric'
      });
    } catch {
      return d;
    }
  });

  ngOnInit(): void {
    this.routeSub = this.route.paramMap.subscribe(params => {
      let date = params.get('date');
      if (!date || date === 'today') {
        const today = new Date();
        const year = today.getFullYear();
        const month = String(today.getMonth() + 1).padStart(2, '0');
        const day = String(today.getDate()).padStart(2, '0');
        date = `${year}-${month}-${day}`;
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
    this.title.setTitle(date);
  }

  goToPrevDay(): void {
    const d = new Date(this.currentDate() + 'T00:00:00');
    d.setDate(d.getDate() - 1);
    const dateStr = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
    this.router.navigate(['/journals', dateStr]);
  }

  goToNextDay(): void {
    const d = new Date(this.currentDate() + 'T00:00:00');
    d.setDate(d.getDate() + 1);
    const dateStr = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
    this.router.navigate(['/journals', dateStr]);
  }

  onDateSelected(date: string): void {
    this.showCalendar.set(false);
    this.router.navigate(['/journals', date]);
  }
}
