import { describe, it, expect } from 'vitest';
import { isoWeekNumber, isoWeekYear, usWeekNumber, weeksInYear } from './week.utils';

describe('isoWeekNumber', () => {
  it('counts week 1 from the week holding January 4th', () => {
    expect(isoWeekNumber(new Date(2026, 0, 4))).toBe(1);
    expect(isoWeekNumber(new Date(2026, 0, 5))).toBe(2);
  });

  it('puts the first days of a year into the last week of the year before', () => {
    expect(isoWeekNumber(new Date(2027, 0, 1))).toBe(53);
    expect(isoWeekYear(new Date(2027, 0, 1))).toBe(2026);
  });
});

describe('usWeekNumber', () => {
  it('starts week 1 on the Sunday on or before January 1st', () => {
    expect(usWeekNumber(new Date(2026, 0, 1))).toBe(1);
    expect(usWeekNumber(new Date(2026, 0, 3))).toBe(1);
    expect(usWeekNumber(new Date(2026, 0, 4))).toBe(2);
  });
});

describe('weeksInYear', () => {
  it('knows ISO years with 52 and 53 weeks', () => {
    expect(weeksInYear(2025, 'iso')).toBe(52);
    expect(weeksInYear(2026, 'iso')).toBe(53);
    expect(weeksInYear(2020, 'iso')).toBe(53);
  });

  it('knows US years with 53 and 54 weeks', () => {
    expect(weeksInYear(2025, 'us')).toBe(53);
    expect(weeksInYear(2028, 'us')).toBe(54);
  });
});
