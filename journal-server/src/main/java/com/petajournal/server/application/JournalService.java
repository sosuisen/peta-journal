package com.petajournal.server.application;

import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.ItemPosition;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.domain.IdGenerator;
import com.petajournal.server.domain.FractionalIndexer;
import com.petajournal.server.domain.JournalFileNameMatcher;
import com.petajournal.server.domain.JournalFileNameMatcher.JournalType;
import com.petajournal.server.infrastructure.export.MarkdownExportService;
import com.sosuisha.hmtid.HMTID;
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

@ApplicationScoped
public class JournalService {

    private static final String JOURNAL_TYPE = "journal";

    private final PageRepository pageRepository;
    private final ItemRepository itemRepository;
    private final SearchService searchService;
    private final ConfigService configService;
    private final ItemService itemService;
    private final IdGenerator idGenerator;
    private final MarkdownExportService exportService;

    @Inject
    public JournalService(
            PageRepository pageRepository,
            ItemRepository itemRepository,
            SearchService searchService,
            ConfigService configService,
            ItemService itemService,
            IdGenerator idGenerator,
            MarkdownExportService exportService) {
        this.pageRepository = pageRepository;
        this.itemRepository = itemRepository;
        this.searchService = searchService;
        this.configService = configService;
        this.itemService = itemService;
        this.idGenerator = idGenerator;
        this.exportService = exportService;
    }

    /** Result of getting or creating a journal page. */
    public record JournalResult(Page page, List<Item> items, boolean created) {}

    /** Result of finding the most recent journal before a date. */
    public record MostRecentJournalResult(boolean found, Page page, List<Item> items) {}

    /**
     * Gets an existing daily journal page for the given date, or creates a new one
     * with a single empty item.
     *
     * @param date the journal date
     * @return the journal page with its items and whether it was newly created
     */
    public JournalResult getOrCreateJournal(LocalDate date) {
        Optional<Page> existing = pageRepository.findJournal(date);
        if (existing.isPresent()) {
            return existingJournal(existing.get());
        }

        Page page = createJournalPage(date.format(DateTimeFormatter.ISO_LOCAL_DATE));
        Instant now = page.createdDate();

        // Create initial empty item
        HMTID itemId = idGenerator.generateItemId();
        Item item = new Item(itemId, page.id(), null, page.dirPath(), page.name(), page.type(), now, now,
                FractionalIndexer.initial(), 1, "", "", false, List.of(), null);
        item = itemRepository.save(item);
        exportService.queueExport(page.id());

        return new JournalResult(page, List.of(item), true);
    }

    /**
     * Gets or creates a journal page by file name.
     * Supports daily ({@code YYYY-MM-DD}), weekly ({@code YYYY-WNN}),
     * monthly ({@code YYYY-MM}), and yearly ({@code YYYY}) formats.
     * A new weekly, monthly or yearly journal is filled with one wiki link per
     * day, week or month it covers.
     *
     * @param fileName the journal file name
     * @return the journal page with its items and whether it was newly created
     * @throws IllegalArgumentException if the file name is not a valid journal format
     */
    public JournalResult getOrCreateJournalByName(String fileName) {
        JournalType type = JournalFileNameMatcher.classify(fileName);
        return switch (type) {
            case NONE -> throw new IllegalArgumentException("Not a journal filename: " + fileName);
            case DAILY -> getOrCreateJournal(LocalDate.parse(fileName));
            case WEEKLY -> {
                // Parse "YYYY-WN"
                String[] parts = fileName.split("-W");
                int year = Integer.parseInt(parts[0]);
                int week = Integer.parseInt(parts[1]);
                yield getOrCreateLinkJournal(year + "-W" + twoDigits(week), weekDayNames(year, week));
            }
            case MONTHLY -> {
                // Parse "YYYY-MM"
                String[] parts = fileName.split("-");
                int year = Integer.parseInt(parts[0]);
                int month = Integer.parseInt(parts[1]);
                yield getOrCreateLinkJournal(year + "-" + twoDigits(month), weekNamesInMonth(year, month));
            }
            case YEARLY -> {
                int year = Integer.parseInt(fileName);
                yield getOrCreateLinkJournal(String.valueOf(year), monthNames(year));
            }
        };
    }

    /**
     * Gets a journal page that, when created, holds one wiki link per period it covers.
     *
     * @param fileName    the journal file name
     * @param linkTargets the journal names to link to, one item each
     */
    private JournalResult getOrCreateLinkJournal(String fileName, List<String> linkTargets) {
        Optional<Page> existing = pageRepository.findByTypeDirPathFileName(JOURNAL_TYPE, "", fileName);
        if (existing.isPresent()) {
            return existingJournal(existing.get());
        }

        Page page = createJournalPage(fileName);

        List<Item> items = new ArrayList<>();
        ItemPosition position = ItemPosition.FIRST;
        for (String target : linkTargets) {
            Item item = itemService.createItem(page.id(), position, 1, "[[" + target + "]]", null);
            items.add(item);
            position = ItemPosition.after(item.id());
        }

        return new JournalResult(page, items, true);
    }

    private JournalResult existingJournal(Page page) {
        return new JournalResult(page, itemRepository.findByPageId(page.id()), false);
    }

    /** Saves and indexes a new journal page without items. */
    private Page createJournalPage(String fileName) {
        HMTID id = idGenerator.generatePageId();
        Instant now = Instant.now();
        Page page = pageRepository.save(new Page(id, null, "", fileName, JOURNAL_TYPE, now, now));
        searchService.indexPage(page);
        return page;
    }

    private static String twoDigits(int n) {
        return String.format("%02d", n);
    }

    private boolean isIsoWeekNumbering() {
        return "iso".equals(configService.getWeekNumbering());
    }

    /** The names of the seven daily journals of a week, e.g. {@code 2026-02-16}. */
    private List<String> weekDayNames(int year, int week) {
        LocalDate startOfWeek;
        if (isIsoWeekNumbering()) {
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

        List<String> days = new ArrayList<>();
        for (int i = 0; i < 7; i++) {
            days.add(startOfWeek.plusDays(i).format(DateTimeFormatter.ISO_LOCAL_DATE));
        }
        return days;
    }

    /** The names of the twelve monthly journals of a year, e.g. {@code 2026-02}. */
    private static List<String> monthNames(int year) {
        List<String> months = new ArrayList<>();
        for (int m = 1; m <= 12; m++) {
            months.add(year + "-" + twoDigits(m));
        }
        return months;
    }

    /** The names of the weekly journals that overlap a month, e.g. {@code 2026-W07}. */
    private List<String> weekNamesInMonth(int year, int month) {
        boolean iso = isIsoWeekNumbering();

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
        return weeks.stream().map(w -> year + "-W" + twoDigits(w)).toList();
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
        return toMostRecentResult(pageRepository.findMostRecentJournalBefore(beforeDate));
    }

    /**
     * Finds the most recent weekly journal page before the specified week.
     *
     * @param beforeWeek the exclusive upper bound week name (e.g., "2026-W09")
     * @return the result containing the weekly journal page if found
     */
    public MostRecentJournalResult getMostRecentWeeklyJournalBefore(String beforeWeek) {
        return toMostRecentResult(pageRepository.findMostRecentWeeklyJournalBefore(beforeWeek));
    }

    private MostRecentJournalResult toMostRecentResult(Optional<Page> page) {
        return page
                .map(p -> new MostRecentJournalResult(true, p, itemRepository.findByPageId(p.id())))
                .orElseGet(() -> new MostRecentJournalResult(false, null, List.of()));
    }
}
