package com.petajournal.server.application;

import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.ItemPosition;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.domain.ItemIdGenerator;
import com.petajournal.server.domain.FractionalIndexer;
import com.petajournal.server.domain.JournalFileNameMatcher;
import com.petajournal.server.domain.JournalFileNameMatcher.JournalType;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import java.time.DayOfWeek;
import java.time.Instant;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.time.temporal.TemporalAdjusters;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.UUID;

@ApplicationScoped
public class JournalService {

    @Inject
    PageRepository pageRepository;

    @Inject
    ItemRepository itemRepository;

    @Inject
    SearchService searchService;

    @Inject
    ConfigService configService;

    @Inject
    ItemService itemService;

    /** Result of getting or creating a journal page. */
    public record JournalResult(Page page, List<Item> items, boolean created) {}

    /** Result of finding the most recent journal before a date. */
    public record MostRecentJournalResult(boolean found, Page page, List<Item> items) {}

    /**
     * Gets an existing daily journal page for the given date, or creates a new one.
     *
     * @param date the journal date
     * @return the journal page with its items and whether it was newly created
     */
    public JournalResult getOrCreateJournal(LocalDate date) {
        Optional<Page> existing = pageRepository.findJournal(date);
        if (existing.isPresent()) {
            Page page = existing.get();
            List<Item> items = itemRepository.findByPageId(page.id());
            return new JournalResult(page, items, false);
        }

        // Create new journal page
        UUID id = UUID.randomUUID();
        Instant now = Instant.now();

        String fileName = date.format(DateTimeFormatter.ISO_LOCAL_DATE);

        Page page = new Page(id, null, "", fileName, "journal", now, now);
        page = pageRepository.save(page);
        searchService.indexPage(page);

        // Create initial empty item
        UUID itemId = ItemIdGenerator.generate();
        Item item = new Item(itemId, id, null, page.dirPath(), page.name(), page.type(), now, now, FractionalIndexer.initial(), 1, "", "", false, List.of(), null);
        item = itemRepository.save(item);

        return new JournalResult(page, List.of(item), true);
    }

    private JournalResult getOrCreateWeeklyJournal(int year, int week) {
        String fileName = year + "-W" + String.format("%02d", week);
        Optional<Page> existing = pageRepository.findByTypeDirPathFileName("journal", "", fileName);
        if (existing.isPresent()) {
            Page page = existing.get();
            List<Item> items = itemRepository.findByPageId(page.id());
            return new JournalResult(page, items, false);
        }

        // Create new weekly journal page
        UUID id = UUID.randomUUID();
        Instant now = Instant.now();

        Page page = new Page(id, null, "", fileName, "journal", now, now);
        page = pageRepository.save(page);
        searchService.indexPage(page);

        // Create items with wiki links to each day of the week
        List<LocalDate> weekDays = getWeekDays(year, week);
        List<Item> items = new ArrayList<>();
        ItemPosition position = ItemPosition.FIRST;
        for (LocalDate day : weekDays) {
            String body = "[[" + day.format(DateTimeFormatter.ISO_LOCAL_DATE) + "]]";
            Item item = itemService.createItem(id, position, 1, body, null);
            items.add(item);
            position = ItemPosition.after(item.id());
        }

        return new JournalResult(page, items, true);
    }

    private List<LocalDate> getWeekDays(int year, int week) {
        String weekNumbering = configService.getWeekNumbering();
        boolean iso = "iso".equals(weekNumbering);

        LocalDate startOfWeek;
        if (iso) {
            // ISO: weeks start Monday, Jan 4 is always in week 1
            LocalDate jan4 = LocalDate.of(year, 1, 4);
            LocalDate monday1 = jan4.with(TemporalAdjusters.previousOrSame(DayOfWeek.MONDAY));
            startOfWeek = monday1.plusWeeks(week - 1);
        } else {
            // US: weeks start Sunday, week 1 contains Jan 1
            LocalDate jan1 = LocalDate.of(year, 1, 1);
            LocalDate sunday1 = jan1.with(TemporalAdjusters.previousOrSame(DayOfWeek.SUNDAY));
            startOfWeek = sunday1.plusWeeks(week - 1);
        }

        List<LocalDate> days = new ArrayList<>();
        for (int i = 0; i < 7; i++) {
            days.add(startOfWeek.plusDays(i));
        }
        return days;
    }

    /**
     * Gets or creates a journal page by file name.
     * Supports daily ({@code YYYY-MM-DD}), weekly ({@code YYYY-WNN}),
     * monthly ({@code YYYY-MM}), and yearly ({@code YYYY}) formats.
     *
     * @param fileName the journal file name
     * @return the journal page with its items and whether it was newly created
     * @throws IllegalArgumentException if the file name is not a valid journal format
     */
    public JournalResult getOrCreateJournalByName(String fileName) {
        JournalType type = JournalFileNameMatcher.classify(fileName);
        if (type == JournalType.NONE) {
            throw new IllegalArgumentException("Not a journal filename: " + fileName);
        }
        if (type == JournalType.DAILY) {
            return getOrCreateJournal(LocalDate.parse(fileName));
        }
        if (type == JournalType.WEEKLY) {
            // Parse "YYYY-WN"
            String[] parts = fileName.split("-W");
            int year = Integer.parseInt(parts[0]);
            int week = Integer.parseInt(parts[1]);
            return getOrCreateWeeklyJournal(year, week);
        }
        if (type == JournalType.MONTHLY) {
            // Parse "YYYY-MM"
            String[] parts = fileName.split("-");
            int year = Integer.parseInt(parts[0]);
            int month = Integer.parseInt(parts[1]);
            return getOrCreateMonthlyJournal(year, month);
        }
        // YEARLY: parse "YYYY"
        return getOrCreateYearlyJournal(Integer.parseInt(fileName));
    }

    private JournalResult getOrCreateYearlyJournal(int year) {
        String fileName = String.valueOf(year);
        Optional<Page> existing = pageRepository.findByTypeDirPathFileName("journal", "", fileName);
        if (existing.isPresent()) {
            Page page = existing.get();
            List<Item> items = itemRepository.findByPageId(page.id());
            return new JournalResult(page, items, false);
        }

        UUID id = UUID.randomUUID();
        Instant now = Instant.now();

        Page page = new Page(id, null, "", fileName, "journal", now, now);
        page = pageRepository.save(page);
        searchService.indexPage(page);

        // Create items with wiki links to each month
        List<Item> items = new ArrayList<>();
        ItemPosition position = ItemPosition.FIRST;
        for (int m = 1; m <= 12; m++) {
            String body = "[[" + year + "-" + String.format("%02d", m) + "]]";
            Item item = itemService.createItem(id, position, 1, body, null);
            items.add(item);
            position = ItemPosition.after(item.id());
        }

        return new JournalResult(page, items, true);
    }

    private JournalResult getOrCreateMonthlyJournal(int year, int month) {
        String fileName = year + "-" + String.format("%02d", month);
        Optional<Page> existing = pageRepository.findByTypeDirPathFileName("journal", "", fileName);
        if (existing.isPresent()) {
            Page page = existing.get();
            List<Item> items = itemRepository.findByPageId(page.id());
            return new JournalResult(page, items, false);
        }

        UUID id = UUID.randomUUID();
        Instant now = Instant.now();

        Page page = new Page(id, null, "", fileName, "journal", now, now);
        page = pageRepository.save(page);
        searchService.indexPage(page);

        // Create items with wiki links to each week that overlaps this month
        List<Integer> weekNumbers = getWeeksInMonth(year, month);
        List<Item> items = new ArrayList<>();
        ItemPosition position = ItemPosition.FIRST;
        for (int w : weekNumbers) {
            String body = "[[" + year + "-W" + String.format("%02d", w) + "]]";
            Item item = itemService.createItem(id, position, 1, body, null);
            items.add(item);
            position = ItemPosition.after(item.id());
        }

        return new JournalResult(page, items, true);
    }

    private List<Integer> getWeeksInMonth(int year, int month) {
        String weekNumbering = configService.getWeekNumbering();
        boolean iso = "iso".equals(weekNumbering);

        LocalDate firstOfMonth = LocalDate.of(year, month, 1);
        LocalDate lastOfMonth = firstOfMonth.withDayOfMonth(firstOfMonth.lengthOfMonth());

        List<Integer> weeks = new ArrayList<>();
        LocalDate date = firstOfMonth;
        while (!date.isAfter(lastOfMonth)) {
            int weekNum = iso ? getISOWeekNumber(date) : getUSWeekNumber(date, year);
            if (weeks.isEmpty() || weeks.get(weeks.size() - 1) != weekNum) {
                weeks.add(weekNum);
            }
            date = date.plusDays(1);
        }
        return weeks;
    }

    private int getISOWeekNumber(LocalDate date) {
        // ISO 8601: week starts Monday, Jan 4 is always in week 1
        LocalDate jan4 = LocalDate.of(date.getYear(), 1, 4);
        // If date is before the Monday of week 1, it belongs to last year's last week
        LocalDate monday1 = jan4.with(TemporalAdjusters.previousOrSame(DayOfWeek.MONDAY));
        if (date.isBefore(monday1)) {
            // Belongs to last year - recalculate
            LocalDate prevJan4 = LocalDate.of(date.getYear() - 1, 1, 4);
            LocalDate prevMonday1 = prevJan4.with(TemporalAdjusters.previousOrSame(DayOfWeek.MONDAY));
            return (int) ((date.toEpochDay() - prevMonday1.toEpochDay()) / 7) + 1;
        }
        return (int) ((date.toEpochDay() - monday1.toEpochDay()) / 7) + 1;
    }

    private int getUSWeekNumber(LocalDate date, int year) {
        // US: week 1 contains Jan 1, weeks start Sunday
        LocalDate jan1 = LocalDate.of(year, 1, 1);
        LocalDate sunday1 = jan1.with(TemporalAdjusters.previousOrSame(DayOfWeek.SUNDAY));
        return (int) ((date.toEpochDay() - sunday1.toEpochDay()) / 7) + 1;
    }

    /**
     * Finds the most recent daily journal page before the specified date.
     *
     * @param beforeDate the exclusive upper bound date
     * @return the result containing the journal page if found
     */
    public MostRecentJournalResult getMostRecentJournalBefore(LocalDate beforeDate) {
        Optional<Page> pageOpt = pageRepository.findMostRecentJournalBefore(beforeDate);
        if (pageOpt.isEmpty()) {
            return new MostRecentJournalResult(false, null, List.of());
        }

        Page page = pageOpt.get();
        List<Item> items = itemRepository.findByPageId(page.id());
        return new MostRecentJournalResult(true, page, items);
    }

    /**
     * Finds the most recent weekly journal page before the specified week.
     *
     * @param beforeWeek the exclusive upper bound week name (e.g., "2026-W09")
     * @return the result containing the weekly journal page if found
     */
    public MostRecentJournalResult getMostRecentWeeklyJournalBefore(String beforeWeek) {
        Optional<Page> pageOpt = pageRepository.findMostRecentWeeklyJournalBefore(beforeWeek);
        if (pageOpt.isEmpty()) {
            return new MostRecentJournalResult(false, null, List.of());
        }

        Page page = pageOpt.get();
        List<Item> items = itemRepository.findByPageId(page.id());
        return new MostRecentJournalResult(true, page, items);
    }
}
