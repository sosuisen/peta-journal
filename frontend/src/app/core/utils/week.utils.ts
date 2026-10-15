import { WeekNumbering } from '../services/config.service';

const DAY_MS = 86400000;

/** The Thursday of the ISO week the date is in; ISO weeks belong to the year of their Thursday. */
function isoThursdayOf(date: Date): Date {
  const d = new Date(date.getTime());
  d.setHours(0, 0, 0, 0);
  d.setDate(d.getDate() + 3 - ((d.getDay() + 6) % 7));
  return d;
}

/** ISO 8601 week number: weeks start on Monday and week 1 holds January 4th. */
export function isoWeekNumber(date: Date): number {
  const thursday = isoThursdayOf(date);
  const yearStart = new Date(thursday.getFullYear(), 0, 4);
  return 1 + Math.round(((thursday.getTime() - yearStart.getTime()) / DAY_MS - 3 + ((yearStart.getDay() + 6) % 7)) / 7);
}

/** The year an ISO week belongs to, which differs from the calendar year around New Year. */
export function isoWeekYear(date: Date): number {
  return isoThursdayOf(date).getFullYear();
}

/** US week number: weeks start on Sunday and week 1 holds January 1st. */
export function usWeekNumber(date: Date): number {
  const d = new Date(date.getTime());
  d.setHours(0, 0, 0, 0);
  // Move the year start back to its containing Sunday
  const yearStartSunday = new Date(d.getFullYear(), 0, 1);
  yearStartSunday.setDate(yearStartSunday.getDate() - yearStartSunday.getDay());
  return 1 + Math.floor((d.getTime() - yearStartSunday.getTime()) / (7 * DAY_MS));
}

/**
 * The number of weeks a year has, so that week navigation does not skip the last one.
 * ISO years have 52 or 53 weeks; US numbering can reach 54 when a leap year starts on a Saturday.
 */
export function weeksInYear(year: number, numbering: WeekNumbering): number {
  if (numbering === 'iso') {
    // December 28th is always in the last ISO week of its year
    return isoWeekNumber(new Date(year, 11, 28));
  }
  return usWeekNumber(new Date(year, 11, 31));
}
