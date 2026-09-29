package com.petajournal.server.domain;

import java.util.ArrayList;
import java.util.List;

/**
 * Generates fractional index strings for ordering items.
 * Uses lexicographic midpoint calculation between two strings.
 * Alphabet: 0-9 A-Z a-z (62 characters, in ASCII order).
 */
public final class FractionalIndexer {

    private static final String ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    private static final int RADIX = ALPHABET.length(); // 62
    private static final char MIN_CHAR = ALPHABET.charAt(0);            // '0'
    private static final char MID_CHAR = ALPHABET.charAt(RADIX / 2);    // 'V'
    private static final char MAX_CHAR = ALPHABET.charAt(RADIX - 1);    // 'z'
    private static final String FIRST = "z0";

    private FractionalIndexer() {}

    /**
     * Returns the initial index for the first item.
     */
    public static String initial() {
        return FIRST;
    }

    /**
     * Returns a new index that sorts between the two given indices.
     * If before is null, returns an index before after.
     * If after is null, returns an index after before.
     */
    public static String between(String before, String after) {
        if (before == null && after == null) {
            return FIRST;
        }
        if (before == null) {
            return prepend(after);
        }
        if (after == null) {
            return append(before);
        }
        return midpoint(before, after);
    }

    /**
     * Returns new indices in ascending order that all sort between the two given indices.
     * Use this to place a group of items into one gap: every index stays inside the gap,
     * however narrow it is.
     *
     * @param before the index the new indices must sort after, or null if there is none
     * @param after  the index the new indices must sort before, or null if there is none
     * @param count  the number of indices to create
     * @return count indices in ascending order
     * @throws IllegalArgumentException if count is negative
     * @throws IllegalStateException    if the gap cannot hold the indices; the caller must reindex the page
     */
    public static List<String> between(String before, String after, int count) {
        if (count < 0) {
            throw new IllegalArgumentException("count must not be negative: " + count);
        }
        List<String> indices = new ArrayList<>(count);
        if (before != null && after != null) {
            if (before.compareTo(after) >= 0) {
                throw new IllegalStateException("No gap between " + before + " and " + after);
            }
            fillBetween(before, after, count, indices);
            return indices;
        }
        String current = after == null ? before : after;
        for (int i = 0; i < count; i++) {
            current = after == null ? between(current, null) : between(null, current);
            indices.add(current);
        }
        if (after != null) {
            indices = indices.reversed();
        }
        return indices;
    }

    /** Splits the gap in the middle first, so the indices grow in length as slowly as possible. */
    private static void fillBetween(String before, String after, int count, List<String> out) {
        if (count == 0) {
            return;
        }
        String mid = midpoint(before, after);
        if (mid.compareTo(before) <= 0 || mid.compareTo(after) >= 0) {
            throw new IllegalStateException("No index between " + before + " and " + after);
        }
        int left = (count - 1) / 2;
        fillBetween(before, mid, left, out);
        out.add(mid);
        fillBetween(mid, after, count - 1 - left, out);
    }

    private static int indexOf(char c) {
        int idx = ALPHABET.indexOf(c);
        if (idx < 0) {
            throw new IllegalArgumentException("Character not in alphabet: " + c);
        }
        return idx;
    }

    private static String append(String idx) {
        int len = idx.length();
        char last = idx.charAt(len - 1);
        int pos = indexOf(last);
        if (pos < RADIX - 1) {
            return idx.substring(0, len - 1) + ALPHABET.charAt(pos + 1);
        }
        // Last char is max, extend with mid character
        return idx + MID_CHAR;
    }

    private static String prepend(String idx) {
        // Find the rightmost character that can be decremented
        for (int i = idx.length() - 1; i >= 0; i--) {
            int pos = indexOf(idx.charAt(i));
            if (pos > 0) {
                // Decrement this character and truncate everything after it
                return idx.substring(0, i) + ALPHABET.charAt(pos - 1);
            }
        }
        // All characters are '0' (minimum). This shouldn't happen with initial "a0".
        throw new IllegalStateException("Cannot prepend before index: " + idx);
    }

    private static String midpoint(String a, String b) {
        // Pad to same length using the minimum character
        int maxLen = Math.max(a.length(), b.length());
        String pa = pad(a, maxLen);
        String pb = pad(b, maxLen);

        StringBuilder result = new StringBuilder();
        boolean found = false;

        for (int i = 0; i < maxLen; i++) {
            int ia = indexOf(pa.charAt(i));
            int ib = indexOf(pb.charAt(i));

            if (!found) {
                if (ia == ib) {
                    result.append(ALPHABET.charAt(ia));
                } else if (ib - ia > 1) {
                    result.append(ALPHABET.charAt(ia + (ib - ia) / 2));
                    found = true;
                } else {
                    // Adjacent positions, carry to next level
                    result.append(ALPHABET.charAt(ia));
                }
            }
        }

        if (!found) {
            // Couldn't find midpoint within existing length, extend
            result.append(MID_CHAR);
        }

        return result.toString();
    }

    /**
     * Generates evenly spaced indices for re-indexing.
     */
    public static List<String> generateIndices(int count) {
        List<String> indices = new ArrayList<>(count);
        if (count == 0) return indices;
        String current = FIRST;
        indices.add(current);
        for (int i = 1; i < count; i++) {
            current = append(current);
            indices.add(current);
        }
        return indices;
    }

    private static String pad(String s, int length) {
        if (s.length() >= length) return s;
        StringBuilder sb = new StringBuilder(s);
        while (sb.length() < length) {
            sb.append(MIN_CHAR);
        }
        return sb.toString();
    }
}
