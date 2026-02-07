package com.petajournal.server.infrastructure.persistence;

/**
 * Generates fractional index strings for ordering blocks.
 * Uses lexicographic midpoint calculation between two strings.
 */
public final class FractionalIndexer {

    private static final String FIRST = "a0";
    private static final char BASE = 'a';
    private static final int RADIX = 26; // a-z

    private FractionalIndexer() {}

    /**
     * Returns the initial index for the first block.
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

    private static String append(String idx) {
        // Simply add a character beyond the last
        int len = idx.length();
        char last = idx.charAt(len - 1);
        if (last < 'z') {
            return idx.substring(0, len - 1) + (char)(last + 1);
        }
        // Extend with 'n' (middle of a-z)
        return idx + "n";
    }

    private static String prepend(String idx) {
        int len = idx.length();
        char first = idx.charAt(len - 1);
        if (first > 'a') {
            return idx.substring(0, len - 1) + (char)(first - 1);
        }
        // Need to go deeper
        return idx + "a";
    }

    private static String midpoint(String a, String b) {
        // Pad to same length
        int maxLen = Math.max(a.length(), b.length());
        String pa = pad(a, maxLen);
        String pb = pad(b, maxLen);

        StringBuilder result = new StringBuilder();
        boolean found = false;

        for (int i = 0; i < maxLen; i++) {
            char ca = pa.charAt(i);
            char cb = pb.charAt(i);

            if (!found) {
                if (ca == cb) {
                    result.append(ca);
                } else if (cb - ca > 1) {
                    result.append((char)(ca + (cb - ca) / 2));
                    found = true;
                } else {
                    // Adjacent chars, carry to next position
                    result.append(ca);
                }
            }
        }

        if (!found) {
            // Couldn't find midpoint within existing length, extend
            result.append('n');
        }

        return result.toString();
    }

    private static String pad(String s, int length) {
        if (s.length() >= length) return s;
        StringBuilder sb = new StringBuilder(s);
        while (sb.length() < length) {
            sb.append('a');
        }
        return sb.toString();
    }
}
