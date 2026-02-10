import { Component, inject, input, output, signal, computed } from '@angular/core';
import { ConfigService } from '../core/services/config.service';
import { formatDateToIso } from '../core/utils/date.utils';

interface CalendarDay {
  date: string;
  day: number;
  isCurrentMonth: boolean;
  isToday: boolean;
  isSelected: boolean;
}

interface CalendarWeek {
  weekNumber: number;
  days: CalendarDay[];
}

@Component({
  selector: 'app-calendar',
  template: `
    <div class="calendar">
      <div class="calendar-header">
        <button (click)="prevMonth()">&lt;</button>
        <span><span class="month-label" (click)="onMonthClick()">{{ monthName() }}</span> <span class="year-label" (click)="onYearClick()">{{ viewYear() }}</span></span>
        <button (click)="nextMonth()">&gt;</button>
      </div>
      <div class="calendar-grid">
        <div class="week-label"></div>
        @for (label of dayLabels(); track label) {
          <div class="day-label">{{ label }}</div>
        }
        @for (week of weeks(); track week.weekNumber) {
          <div class="week-number" (click)="onWeekClick(week)">{{ week.weekNumber.toString().padStart(2, '0') }}</div>
          @for (day of week.days; track day.date) {
            <div
              class="day-cell"
              [class.other-month]="!day.isCurrentMonth"
              [class.today]="day.isToday"
              [class.selected]="day.isSelected"
              (click)="selectDay(day)"
            >
              {{ day.day }}
            </div>
          }
        }
      </div>
    </div>
  `,
  styles: [`
    .calendar {
      background: white;
      border: 1px solid #ddd;
      border-radius: 8px;
      padding: 6px;
      width: 100%;
      box-sizing: border-box;
    }
    .calendar-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 4px;
      font-weight: 600;
      font-size: 12px;
    }
    .calendar-header button {
      background: none;
      border: none;
      cursor: pointer;
      padding: 2px 6px;
      font-size: 12px;
    }
    .month-label, .year-label {
      cursor: pointer;
      border-radius: 4px;
      padding: 2px 4px;
    }
    .month-label:hover, .year-label:hover {
      background: #e9ecef;
      color: #007bff;
    }
    .calendar-grid {
      display: grid;
      grid-template-columns: auto repeat(7, 1fr);
      gap: 1px;
      text-align: center;
    }
    .week-label, .week-number {
      font-size: 9px;
      color: #aaa;
      padding: 2px 4px 2px 0;
      text-align: right;
      border-right: 1px solid #ddd;
    }
    .week-number {
      cursor: pointer;
      border-radius: 2px;
    }
    .week-number:hover {
      background: #e9ecef;
      color: #007bff;
    }
    .day-label {
      font-size: 10px;
      color: #888;
      padding: 2px;
    }
    .day-cell {
      padding: 3px;
      cursor: pointer;
      border-radius: 4px;
      font-size: 11px;
    }
    .day-cell:hover { background: #f0f0f0; }
    .day-cell.other-month { color: #ccc; }
    .day-cell.today { font-weight: bold; border: 1px solid #007bff; }
    .day-cell.selected { background: #007bff; color: white; }
  `]
})
export class CalendarComponent {
  private readonly configService = inject(ConfigService);

  readonly selectedDate = input('');
  readonly dateSelected = output<string>();
  readonly weekClicked = output<{ year: number; week: number }>();
  readonly monthClicked = output<{ year: number; month: number }>();
  readonly yearClicked = output<number>();

  private readonly US_DAY_LABELS = ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'];
  private readonly ISO_DAY_LABELS = ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'];

  readonly dayLabels = computed(() =>
    this.configService.weekNumbering() === 'iso' ? this.ISO_DAY_LABELS : this.US_DAY_LABELS
  );
  readonly viewYear = signal(new Date().getFullYear());
  readonly viewMonth = signal(new Date().getMonth());

  readonly monthName = computed(() => {
    const date = new Date(this.viewYear(), this.viewMonth(), 1);
    return date.toLocaleDateString('en-US', { month: 'long' });
  });

  readonly weeks = computed((): CalendarWeek[] => {
    const allDays = this.days();
    const iso = this.configService.weekNumbering() === 'iso';
    const result: CalendarWeek[] = [];
    for (let i = 0; i < allDays.length; i += 7) {
      const weekDays = allDays.slice(i, i + 7);
      const firstOfWeek = new Date(weekDays[0].date + 'T00:00:00');
      const weekNumber = iso
        ? this.getISOWeekNumber(firstOfWeek)
        : this.getUSWeekNumber(firstOfWeek);
      result.push({ weekNumber, days: weekDays });
    }
    return result;
  });

  private readonly days = computed((): CalendarDay[] => {
    const year = this.viewYear();
    const month = this.viewMonth();
    const iso = this.configService.weekNumbering() === 'iso';
    const today = formatDateToIso(new Date());
    const selected = this.selectedDate();

    const rawDay = new Date(year, month, 1).getDay();
    // US: Sunday=0, ISO: Monday=0
    const firstDay = iso ? (rawDay + 6) % 7 : rawDay;
    const daysInMonth = new Date(year, month + 1, 0).getDate();
    const daysInPrevMonth = new Date(year, month, 0).getDate();

    const result: CalendarDay[] = [];

    for (let i = firstDay - 1; i >= 0; i--) {
      const d = daysInPrevMonth - i;
      const date = this.formatDate(year, month - 1, d);
      result.push({ date, day: d, isCurrentMonth: false, isToday: date === today, isSelected: date === selected });
    }

    for (let d = 1; d <= daysInMonth; d++) {
      const date = this.formatDate(year, month, d);
      result.push({ date, day: d, isCurrentMonth: true, isToday: date === today, isSelected: date === selected });
    }

    const remaining = 42 - result.length;
    for (let d = 1; d <= remaining; d++) {
      const date = this.formatDate(year, month + 1, d);
      result.push({ date, day: d, isCurrentMonth: false, isToday: date === today, isSelected: date === selected });
    }

    return result;
  });

  prevMonth(): void {
    if (this.viewMonth() === 0) {
      this.viewYear.update(y => y - 1);
      this.viewMonth.set(11);
    } else {
      this.viewMonth.update(m => m - 1);
    }
  }

  nextMonth(): void {
    if (this.viewMonth() === 11) {
      this.viewYear.update(y => y + 1);
      this.viewMonth.set(0);
    } else {
      this.viewMonth.update(m => m + 1);
    }
  }

  onMonthClick(): void {
    this.monthClicked.emit({ year: this.viewYear(), month: this.viewMonth() + 1 });
  }

  onYearClick(): void {
    this.yearClicked.emit(this.viewYear());
  }

  onWeekClick(week: CalendarWeek): void {
    const firstOfWeek = new Date(week.days[0].date + 'T00:00:00');
    const iso = this.configService.weekNumbering() === 'iso';
    const year = iso ? this.getISOWeekYear(firstOfWeek) : firstOfWeek.getFullYear();
    this.weekClicked.emit({ year, week: week.weekNumber });
  }

  private getISOWeekYear(date: Date): number {
    const d = new Date(date.getTime());
    d.setHours(0, 0, 0, 0);
    d.setDate(d.getDate() + 3 - ((d.getDay() + 6) % 7));
    return d.getFullYear();
  }

  selectDay(day: CalendarDay): void {
    this.dateSelected.emit(day.date);
  }

  private getISOWeekNumber(date: Date): number {
    const d = new Date(date.getTime());
    d.setHours(0, 0, 0, 0);
    d.setDate(d.getDate() + 3 - ((d.getDay() + 6) % 7));
    const yearStart = new Date(d.getFullYear(), 0, 4);
    return 1 + Math.round(((d.getTime() - yearStart.getTime()) / 86400000 - 3 + ((yearStart.getDay() + 6) % 7)) / 7);
  }

  private getUSWeekNumber(date: Date): number {
    // US: week containing Jan 1 = week 1, weeks start Sunday
    const d = new Date(date.getTime());
    d.setHours(0, 0, 0, 0);
    const yearStart = new Date(d.getFullYear(), 0, 1);
    // Move yearStart back to its containing Sunday
    const yearStartSunday = new Date(yearStart.getTime());
    yearStartSunday.setDate(yearStartSunday.getDate() - yearStartSunday.getDay());
    const diff = d.getTime() - yearStartSunday.getTime();
    return 1 + Math.floor(diff / (7 * 86400000));
  }

  private formatDate(year: number, month: number, day: number): string {
    const d = new Date(year, month, day);
    return formatDateToIso(d);
  }
}
