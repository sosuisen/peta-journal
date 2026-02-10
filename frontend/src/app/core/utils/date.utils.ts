/**
 * Format a journal fileName (e.g. "2026-02-10") as a localized Japanese date with weekday.
 * Returns the fileName as-is if parsing fails.
 */
export function formatJournalDate(fileName: string): string {
  // Yearly journal: "2024" → "2024年"
  const yearMatch = fileName.match(/^\d{4}$/);
  if (yearMatch) {
    return `${fileName}年`;
  }

  // Monthly journal: "2024-08" → "2024年8月"
  const monthMatch = fileName.match(/^(\d{4})-(\d{2})$/);
  if (monthMatch) {
    return `${monthMatch[1]}年${parseInt(monthMatch[2], 10)}月`;
  }

  // Weekly journal: "2026-W07" → "2026 Week 7"
  const weekMatch = fileName.match(/^(\d{4})-W(\d+)$/);
  if (weekMatch) {
    return `${weekMatch[1]} Week ${weekMatch[2]}`;
  }

  try {
    const date = new Date(fileName + 'T00:00:00');
    const dateStr = date.toLocaleDateString('ja-JP', {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
    const weekdayStr = date.toLocaleDateString('ja-JP', {
      weekday: 'long',
    });
    return `${dateStr} ${weekdayStr}`;
  } catch {
    return fileName;
  }
}

/**
 * Format a Date object as an ISO date string (YYYY-MM-DD).
 */
export function formatDateToIso(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}
