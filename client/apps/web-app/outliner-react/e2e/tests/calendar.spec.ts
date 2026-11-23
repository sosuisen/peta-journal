import { expect } from '@playwright/test';
import { test } from './support';

const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August',
  'September', 'October', 'November', 'December'];
const pad = (n: number) => String(n).padStart(2, '0');
const now = new Date();
const todayName = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
/** The first day of the month after this one. */
const next = new Date(now.getFullYear(), now.getMonth() + 1, 1);
const nextMonthName = `${next.getFullYear()}-${pad(next.getMonth() + 1)}`;

test.describe('calendar', () => {
  test('the sidebar calendar shows the month of today, moves to the next month, and opens a day, the month and the year', async ({ page }) => {
    await page.goto('/today');
    const calendar = page.locator('.sidebar app-calendar');

    await expect(calendar.locator('.month-label')).toHaveText(MONTHS[now.getMonth()]);
    await expect(calendar.locator('.year-label')).toHaveText(String(now.getFullYear()));
    // Today is the selected day of the sidebar
    await expect(calendar.locator('.day-cell.today.selected')).toHaveText(String(now.getDate()));
    // Full weeks, each behind its number
    const days = await calendar.locator('.day-cell').count();
    expect(days % 7).toBe(0);
    await expect(calendar.locator('.week-number')).toHaveCount(days / 7);

    await calendar.getByRole('button', { name: '>' }).click();
    await expect(calendar.locator('.month-label')).toHaveText(MONTHS[next.getMonth()]);
    await expect(calendar.locator('.year-label')).toHaveText(String(next.getFullYear()));

    await calendar.locator('.day-cell:not(.other-month)').getByText('15', { exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/pages/${nextMonthName}-15$`));
    // The calendar stays at the month it was moved to
    await expect(calendar.locator('.month-label')).toHaveText(MONTHS[next.getMonth()]);

    await calendar.locator('.month-label').click();
    await expect(page).toHaveURL(new RegExp(`/pages/${nextMonthName}$`));
    await calendar.locator('.year-label').click();
    await expect(page).toHaveURL(new RegExp(`/pages/${next.getFullYear()}$`));

    await calendar.getByRole('button', { name: '<' }).click();
    await expect(calendar.locator('.month-label')).toHaveText(MONTHS[now.getMonth()]);
  });

  test('the calendar of the journal header opens at the month of today with the day of the journal selected, and closes when a week is opened', async ({ page }) => {
    await page.goto('/today');
    await page.locator('app-outline-item').first().waitFor();
    const sidebar = page.locator('.sidebar app-calendar');
    await sidebar.getByRole('button', { name: '>' }).click();
    await expect(sidebar.locator('.month-label')).toHaveText(MONTHS[next.getMonth()]);

    await page.locator('.calendar-btn').click();
    const popup = page.locator('app-journal app-calendar');
    await expect(popup.locator('.month-label')).toHaveText(MONTHS[now.getMonth()]);
    await expect(popup.locator('.day-cell.selected')).toHaveText(String(now.getDate()));

    // The config of the test server numbers the weeks as ISO 8601 does
    const week = await popup.locator('.week-number').first().textContent();
    await popup.locator('.week-number').first().click();
    await expect(page).toHaveURL(new RegExp(`/pages/\\d{4}-W${week}$`));
    await expect(popup).toHaveCount(0);

    // A day opened from a week page: the journal of that day
    await page.goto(`/pages/${todayName}`);
    await page.locator('.calendar-btn').click();
    await popup.locator('.day-cell.today').click();
    await expect(page).toHaveURL(new RegExp(`/pages/${todayName}$`));
    await expect(popup).toHaveCount(0);
  });
});
