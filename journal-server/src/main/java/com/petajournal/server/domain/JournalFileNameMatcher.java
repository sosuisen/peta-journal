package com.petajournal.server.domain;

import java.util.regex.Pattern;

public final class JournalFileNameMatcher {

    private static final Pattern DAILY = Pattern.compile("^\\d{4}-\\d{2}-\\d{2}$");
    private static final Pattern WEEKLY = Pattern.compile("^\\d{4}-W\\d{1,2}$");
    private static final Pattern MONTHLY = Pattern.compile("^\\d{4}-\\d{2}$");
    private static final Pattern YEARLY = Pattern.compile("^\\d{4}$");

    public enum JournalType { DAILY, WEEKLY, MONTHLY, YEARLY, NONE }

    private JournalFileNameMatcher() {}

    public static JournalType classify(String fileName) {
        if (fileName == null || fileName.isEmpty()) return JournalType.NONE;
        if (DAILY.matcher(fileName).matches()) return JournalType.DAILY;
        if (WEEKLY.matcher(fileName).matches()) return JournalType.WEEKLY;
        if (MONTHLY.matcher(fileName).matches()) return JournalType.MONTHLY;
        if (YEARLY.matcher(fileName).matches()) return JournalType.YEARLY;
        return JournalType.NONE;
    }

    public static boolean isJournalFileName(String fileName) {
        return classify(fileName) != JournalType.NONE;
    }
}
