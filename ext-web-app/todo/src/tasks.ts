import type { Item } from './api';

export type TaskLabel = 'TODO' | 'RUNNING' | 'DONE';

/** The state of a task, from the times it was begun and ended. */
export function taskLabel(item: Item): TaskLabel {
  if (item.properties.end) return 'DONE';
  if (item.properties.begin) return 'RUNNING';
  return 'TODO';
}

function dateProperty(item: Item, key: string): string | null {
  const value = item.properties[key];
  return typeof value === 'string' ? value : null;
}

/** The date the task is planned to start on (YYYY-MM-DD), or null. */
export function getPlan(item: Item): string | null {
  return dateProperty(item, 'plan');
}

/** The date the task is due on (YYYY-MM-DD), or null. */
export function getDue(item: Item): string | null {
  return dateProperty(item, 'due');
}

/**
 * The day of the moment in the local time, as YYYY-MM-DD.
 *
 * Not `toISOString()`: that is the day in UTC, which in Japan is the day before until
 * 9 in the morning.
 */
export function localISODate(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${date.getFullYear()}-${month}-${day}`;
}

export interface Filters {
  /** Hides the tasks planned to start after this date (YYYY-MM-DD). Empty: hides none. */
  untilDate: string;
  showDone: boolean;
  showJournal: boolean;
  showPage: boolean;
}

/**
 * Whether the page of the item is a journal, of a day, a week, a month or a year.
 *
 * The server says so. The name does not: a doc page can have a name like `2026`.
 */
function isJournal(item: Item): boolean {
  return item.type === 'journal';
}

/** Whether the filters let the task through. */
export function isShown(item: Item, filters: Filters): boolean {
  if (!filters.showDone && item.properties.end) return false;
  const pageIsShown = isJournal(item) ? filters.showJournal : filters.showPage;
  if (!pageIsShown) return false;
  // A task without a plan date is always shown
  const plan = getPlan(item);
  if (filters.untilDate && plan && plan > filters.untilDate) return false;
  return true;
}

export type DisplayEntry =
  { kind: 'heading'; label: string } | { kind: 'item'; item: Item };

/**
 * The tasks in the order of their plan dates, with a heading before each date. The tasks
 * without a plan date come last, under a heading of their own.
 */
export function groupByPlan(items: Item[]): DisplayEntry[] {
  const planned = items
    .flatMap(item => {
      const plan = getPlan(item);
      return plan ? [{ plan, item }] : [];
    })
    .sort((a, b) => a.plan.localeCompare(b.plan));
  const unplanned = items.filter(item => !getPlan(item));

  const entries: DisplayEntry[] = [];
  let currentDate = '';
  for (const { plan, item } of planned) {
    if (plan !== currentDate) {
      currentDate = plan;
      entries.push({ kind: 'heading', label: plan });
    }
    entries.push({ kind: 'item', item });
  }
  if (unplanned.length > 0) {
    entries.push({ kind: 'heading', label: '開始日なし' });
    for (const item of unplanned) {
      entries.push({ kind: 'item', item });
    }
  }
  return entries;
}

/** The path of the PetaJournal web app that opens the page of the item at the item. */
export function pagePath(item: Item): string {
  const prefix = isJournal(item) ? '/journals' : '/docs';
  return `${prefix}/${item.dirPath}${item.name}#${item.id}`;
}
