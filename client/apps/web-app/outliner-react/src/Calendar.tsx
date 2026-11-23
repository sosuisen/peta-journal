import type { MouseEvent } from 'react';
import { useCore, usePanel } from './context';
import { openInOf } from './keys';

/**
 * The month calendar of the sidebar. The core decides what it shows (`CalendarView`, ADR 052);
 * this only draws it and sends the clicks. Today is the selected day.
 */
export default function Calendar() {
  const core = useCore();
  const calendar = usePanel('Calendar');
  if (calendar === undefined) {
    return null;
  }
  // Opens a journal by its name; Ctrl opens a tab, Shift a window
  const open = (page: string) => (event: MouseEvent) =>
    core.send({
      kind: 'GoToPage',
      path: page,
      open_in: openInOf(event.nativeEvent),
    });
  return (
    <app-calendar>
      <div className="calendar">
        <div className="calendar-header">
          <button
            onClick={() =>
              core.send({ kind: 'CalendarNav', value: { kind: 'PrevMonth' } })
            }
          >
            {'<'}
          </button>
          <span>
            <span className="month-label" onClick={open(calendar.month_page)}>
              {calendar.month_name}
            </span>{' '}
            <span className="year-label" onClick={open(calendar.year_page)}>
              {calendar.year}
            </span>
          </span>
          <button
            onClick={() =>
              core.send({ kind: 'CalendarNav', value: { kind: 'NextMonth' } })
            }
          >
            {'>'}
          </button>
        </div>
        <div className="calendar-grid">
          <div className="week-label" />
          {calendar.day_labels.map(label => (
            <div key={label} className="day-label">
              {label}
            </div>
          ))}
          {calendar.weeks.map(week => [
            <div
              key={week.page}
              className="week-number"
              onClick={open(week.page)}
            >
              {String(week.number).padStart(2, '0')}
            </div>,
            ...week.days.map(day => (
              <div
                key={day.date}
                className={dayClass(day.other_month, day.today)}
                onClick={open(day.date)}
              >
                {day.day}
              </div>
            )),
          ])}
        </div>
      </div>
    </app-calendar>
  );
}

function dayClass(otherMonth: boolean, today: boolean): string {
  return [
    'day-cell',
    otherMonth ? 'other-month' : '',
    today ? 'today selected' : '',
  ]
    .filter(name => name !== '')
    .join(' ');
}

declare module 'react' {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace JSX {
    interface IntrinsicElements {
      'app-calendar': React.DetailedHTMLProps<
        React.HTMLAttributes<HTMLElement>,
        HTMLElement
      >;
    }
  }
}
