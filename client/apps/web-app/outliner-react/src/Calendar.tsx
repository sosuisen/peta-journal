import type { MouseEvent } from 'react';
import { useCore, usePanel } from './context';
import { openInOf } from './keys';

/**
 * Which day is drawn as the selected one: today, as in the sidebar, or the day of the journal
 * on the screen, as in the header of a journal.
 */
type Selects = 'Today' | 'JournalDay';

/**
 * The month calendar of the sidebar and of the header of a journal. The core decides what it
 * shows (`CalendarView`, ADR 052); this only draws it and sends the clicks.
 *
 * `onNavigate` is called after a journal is opened, with whether it opened in another tab or
 * window. The header of a journal closes its calendar by it.
 */
export default function Calendar({
  selects = 'Today',
  onNavigate,
}: {
  readonly selects?: Selects;
  readonly onNavigate?: (elsewhere: boolean) => void;
}) {
  const core = useCore();
  const calendar = usePanel('Calendar');
  if (calendar === undefined) {
    return null;
  }
  // Opens a journal by its name; Ctrl opens a tab, Shift a window
  const open = (page: string) => (event: MouseEvent) => {
    const open_in = openInOf(event.nativeEvent);
    core.send({ kind: 'GoToPage', path: page, open_in });
    onNavigate?.(open_in.kind !== 'Here');
  };
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
                className={dayClass(
                  day.other_month,
                  day.today,
                  selects === 'Today' ? day.today : day.selected,
                )}
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

function dayClass(
  otherMonth: boolean,
  today: boolean,
  selected: boolean,
): string {
  return [
    'day-cell',
    otherMonth ? 'other-month' : '',
    today ? 'today' : '',
    selected ? 'selected' : '',
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
