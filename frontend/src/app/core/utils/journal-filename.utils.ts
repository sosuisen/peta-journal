export type JournalType = 'daily' | 'weekly' | 'monthly' | 'yearly' | 'none';

const DAILY = /^\d{4}-\d{2}-\d{2}$/;
const WEEKLY = /^\d{4}-W\d{1,2}$/;
const MONTHLY = /^\d{4}-\d{2}$/;
const YEARLY = /^\d{4}$/;

export function classifyJournalFileName(fileName: string): JournalType {
  if (!fileName) return 'none';
  if (DAILY.test(fileName)) return 'daily';
  if (WEEKLY.test(fileName)) return 'weekly';
  if (MONTHLY.test(fileName)) return 'monthly';
  if (YEARLY.test(fileName)) return 'yearly';
  return 'none';
}

export function isJournalFileName(fileName: string): boolean {
  return classifyJournalFileName(fileName) !== 'none';
}
