import { fireEvent, screen } from '@testing-library/react';
import { describe, expect, test } from 'vitest';
import { CalendarDay, CalendarView, CalendarWeek } from './border/peta-types';
import Calendar from './Calendar';
import { draw, shell } from './test/screens';

/**
 * November 2026 as the core shows it, cut to two weeks. The 23rd is today, and the 24th is
 * the day of the journal on the screen.
 */
function november(): CalendarView {
  const day = (date: string, other = false, today = false) =>
    new CalendarDay(
      date,
      Number(date.slice(8)),
      other,
      today,
      date === '2026-11-24',
    );
  return new CalendarView(
    2026,
    11,
    'November',
    '2026-11',
    '2026',
    ['Mo', 'Tu'],
    [
      new CalendarWeek(44, '2026-W44', [
        day('2026-10-26', true),
        day('2026-10-27', true),
      ]),
      new CalendarWeek(48, '2026-W48', [
        day('2026-11-23', false, true),
        day('2026-11-24'),
      ]),
    ],
  );
}

function calendar() {
  const ui = shell();
  ui.wasm.push([], [{ kind: 'Calendar', value: november() }]);
  return { ...ui, ...draw(ui, <Calendar />) };
}

const here = { kind: 'Here' } as const;

// カレンダー
describe('the calendar', () => {
  // コアの月の名前、年、曜日、週の番号、日を出す。
  test('it shows the month, the year, the days of the week, the weeks and the days of the core', () => {
    const { container } = calendar();
    const texts = (selector: string) =>
      [...container.querySelectorAll(selector)].map(e => e.textContent);

    expect(texts('.month-label')).toEqual(['November']);
    expect(texts('.year-label')).toEqual(['2026']);
    expect(texts('.day-label')).toEqual(['Mo', 'Tu']);
    expect(texts('.week-number')).toEqual(['44', '48']);
    expect(texts('.day-cell')).toEqual(['26', '27', '23', '24']);
  });

  // 一桁の週の番号は、0を前につけて2桁で出す。
  test('a week of one digit has a zero before it', () => {
    const ui = shell();
    const view = november();
    view.weeks = [new CalendarWeek(5, '2026-W05', [])];
    ui.wasm.push([], [{ kind: 'Calendar', value: view }]);
    const { container } = draw(ui, <Calendar />);

    expect(container.querySelector('.week-number')?.textContent).toBe('05');
  });

  // 他の月の日と今日には印がつく。今日は、選んだ日としても印がつく。
  test('the days of other months and today are marked, and today is the selected day', () => {
    const { container } = calendar();

    expect(
      [...container.querySelectorAll('.day-cell')].map(e => e.className),
    ).toEqual([
      'day-cell other-month',
      'day-cell other-month',
      'day-cell today selected',
      'day-cell',
    ]);
  });

  // 日誌の見出しのカレンダーは、画面の日誌の日を、選んだ日として印をつける。
  test('in the header of a journal, the day of the journal is the selected day', () => {
    const ui = shell();
    ui.wasm.push([], [{ kind: 'Calendar', value: november() }]);
    const { container } = draw(ui, <Calendar selects="JournalDay" />);

    expect(
      [...container.querySelectorAll('.day-cell')].map(e => e.className),
    ).toEqual([
      'day-cell other-month',
      'day-cell other-month',
      'day-cell today',
      'day-cell selected',
    ]);
  });

  // ジャーナルを開いたあと、ほかのタブか窓で開いたかを知らせる。日誌の見出しは、それでカレンダーを閉じるかを決める。
  test('after a journal is opened, it tells whether it opened in another tab or window', () => {
    const ui = shell();
    ui.wasm.push([], [{ kind: 'Calendar', value: november() }]);
    const opened: boolean[] = [];
    draw(ui, <Calendar onNavigate={elsewhere => opened.push(elsewhere)} />);

    fireEvent.click(screen.getByText('23'));
    fireEvent.click(screen.getByText('48'), { ctrlKey: true });
    fireEvent.click(screen.getByText('November'), { shiftKey: true });
    fireEvent.click(screen.getByRole('button', { name: '>' }));

    expect(opened).toEqual([false, true, true]);
  });

  // 日、週の番号、月、年を押すと、そのジャーナルを開く。押したキーで、開く場所が決まる。
  test('a day, a week, the month and the year open their journal, where the held keys say', () => {
    const { wasm } = calendar();

    fireEvent.click(screen.getByText('23'));
    fireEvent.click(screen.getByText('48'), { ctrlKey: true });
    fireEvent.click(screen.getByText('November'), { shiftKey: true });
    fireEvent.click(screen.getByText('2026'));

    expect(wasm.events).toEqual([
      { kind: 'GoToPage', path: '2026-11-23', open_in: here },
      { kind: 'GoToPage', path: '2026-W48', open_in: { kind: 'NewView' } },
      { kind: 'GoToPage', path: '2026-11', open_in: { kind: 'Browser' } },
      { kind: 'GoToPage', path: '2026', open_in: here },
    ]);
  });

  // 前と次のボタンは、月の移動をコアに頼む。
  test('the buttons before and after ask the core for the month before and after', () => {
    const { wasm } = calendar();

    fireEvent.click(screen.getByRole('button', { name: '<' }));
    fireEvent.click(screen.getByRole('button', { name: '>' }));

    expect(wasm.events).toEqual([
      { kind: 'CalendarNav', value: { kind: 'PrevMonth' } },
      { kind: 'CalendarNav', value: { kind: 'NextMonth' } },
    ]);
  });

  // コアがまだカレンダーを出していなければ、何も描かない。
  test('nothing is drawn before the core gives a calendar', () => {
    const { container } = draw(shell(), <Calendar />);

    expect(container.querySelector('.calendar')).toBeNull();
  });
});
