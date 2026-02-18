package com.petajournal.server.domain;

import java.util.regex.Pattern;

/** Classifies file names into journal types based on date patterns. */
public final class JournalFileNameMatcher {

    private static final Pattern DAILY = Pattern.compile("^\\d{4}-\\d{2}-\\d{2}$");
    private static final Pattern WEEKLY = Pattern.compile("^\\d{4}-W\\d{1,2}$");
    private static final Pattern MONTHLY = Pattern.compile("^\\d{4}-\\d{2}$");
    private static final Pattern YEARLY = Pattern.compile("^\\d{4}$");

    /** The type of journal period. */
    public enum JournalType { DAILY, WEEKLY, MONTHLY, YEARLY, NONE }

    private JournalFileNameMatcher() {}

    /**
     * Classifies a file name into a journal type.
     *
     * @param fileName the file name to classify
     * @return the journal type, or {@link JournalType#NONE} if not a journal name
     */
    public static JournalType classify(String fileName) {
        if (fileName == null || fileName.isEmpty()) return JournalType.NONE;
        if (DAILY.matcher(fileName).matches()) return JournalType.DAILY;
        if (WEEKLY.matcher(fileName).matches()) return JournalType.WEEKLY;
        if (MONTHLY.matcher(fileName).matches()) return JournalType.MONTHLY;
        if (YEARLY.matcher(fileName).matches()) return JournalType.YEARLY;
        return JournalType.NONE;
    }

    /**
     * Checks whether a file name matches any journal pattern.
     *
     * @param fileName the file name to check
     * @return {@code true} if the file name is a journal-type name
     */
    public static boolean isJournalFileName(String fileName) {
        return classify(fileName) != JournalType.NONE;
    }
}
