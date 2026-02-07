import { Component, input, output, signal, computed } from '@angular/core';

interface CalendarDay {
  date: string;
  day: number;
  isCurrentMonth: boolean;
  isToday: boolean;
  isSelected: boolean;
}

@Component({
  selector: 'app-calendar',
  template: `
    <div class="calendar">
      <div class="calendar-header">
        <button (click)="prevMonth()">&lt;</button>
        <span>{{ monthLabel() }}</span>
        <button (click)="nextMonth()">&gt;</button>
      </div>
      <div class="calendar-grid">
        @for (label of dayLabels; track label) {
          <div class="day-label">{{ label }}</div>
        }
        @for (day of days(); track day.date) {
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
      </div>
    </div>
  `,
  styles: [`
    .calendar {
      background: white;
      border: 1px solid #ddd;
      border-radius: 8px;
      padding: 12px;
      width: 280px;
      box-shadow: 0 2px 8px rgba(0,0,0,0.1);
    }
    .calendar-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 8px;
      font-weight: 600;
    }
    .calendar-header button {
      background: none;
      border: none;
      cursor: pointer;
      padding: 4px 8px;
      font-size: 14px;
    }
    .calendar-grid {
      display: grid;
      grid-template-columns: repeat(7, 1fr);
      gap: 2px;
      text-align: center;
    }
    .day-label {
      font-size: 11px;
      color: #888;
      padding: 4px;
    }
    .day-cell {
      padding: 6px;
      cursor: pointer;
      border-radius: 4px;
      font-size: 13px;
    }
    .day-cell:hover { background: #f0f0f0; }
    .day-cell.other-month { color: #ccc; }
    .day-cell.today { font-weight: bold; border: 1px solid #007bff; }
    .day-cell.selected { background: #007bff; color: white; }
  `]
})
export class CalendarComponent {
  readonly selectedDate = input('');
  readonly dateSelected = output<string>();

  readonly dayLabels = ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'];
  readonly viewYear = signal(new Date().getFullYear());
  readonly viewMonth = signal(new Date().getMonth());

  readonly monthLabel = computed(() => {
    const date = new Date(this.viewYear(), this.viewMonth(), 1);
    return date.toLocaleDateString('en-US', { month: 'long', year: 'numeric' });
  });

  readonly days = computed((): CalendarDay[] => {
    const year = this.viewYear();
    const month = this.viewMonth();
    const today = new Date().toISOString().split('T')[0];
    const selected = this.selectedDate();

    const firstDay = new Date(year, month, 1).getDay();
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

  selectDay(day: CalendarDay): void {
    this.dateSelected.emit(day.date);
  }

  private formatDate(year: number, month: number, day: number): string {
    const d = new Date(year, month, day);
    return d.toISOString().split('T')[0];
  }
}
