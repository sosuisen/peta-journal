package com.petajournal.server.infrastructure.export;

import com.petajournal.server.domain.FractionalIndexer;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/**
 * Decides fractional indices for items in the order they appear in a file.
 * Items that already have an index keep it whenever possible (longest increasing
 * subsequence of existing indices); everything else gets a fresh index between
 * its neighbours. This keeps churn on the DB and on other devices to a minimum.
 */
public final class ItemOrderPlanner {

    private ItemOrderPlanner() {}

    /**
     * @param existingIdx per file position: the index the item should have if possible
     *                    (from the file, else the local DB), or {@code null} for a new item
     * @return an index for every position, strictly increasing; positions whose input was kept
     *         return that same string, so callers can tell which ones were recomputed
     * @throws IllegalStateException when the fractional index space between two kept items is exhausted
     */
    public static List<String> plan(List<String> existingIdx) {
        int n = existingIdx.size();
        boolean[] keep = longestIncreasingSubsequence(existingIdx);

        String[] result = new String[n];
        for (int i = 0; i < n; i++) {
            if (keep[i]) result[i] = existingIdx.get(i);
        }

        String prev = null;
        for (int i = 0; i < n; i++) {
            if (result[i] == null) {
                String next = null;
                for (int j = i + 1; j < n; j++) {
                    if (result[j] != null) {
                        next = result[j];
                        break;
                    }
                }
                result[i] = FractionalIndexer.between(prev, next);
            }
            prev = result[i];
        }
        return Arrays.asList(result);
    }

    /** Marks the positions forming the longest strictly increasing run of non-null indices. */
    private static boolean[] longestIncreasingSubsequence(List<String> idx) {
        int n = idx.size();
        int[] length = new int[n];
        int[] prev = new int[n];
        int bestEnd = -1;
        for (int i = 0; i < n; i++) {
            prev[i] = -1;
            if (idx.get(i) == null) continue;
            length[i] = 1;
            for (int j = 0; j < i; j++) {
                if (idx.get(j) == null) continue;
                if (idx.get(j).compareTo(idx.get(i)) < 0 && length[j] + 1 > length[i]) {
                    length[i] = length[j] + 1;
                    prev[i] = j;
                }
            }
            if (bestEnd < 0 || length[i] > length[bestEnd]) bestEnd = i;
        }

        boolean[] keep = new boolean[n];
        for (int i = bestEnd; i >= 0; i = prev[i]) {
            keep[i] = true;
        }
        return keep;
    }

    /** Fresh indices for every position, used when the index space is exhausted. */
    public static List<String> reindexAll(int n) {
        return new ArrayList<>(FractionalIndexer.generateIndices(n));
    }
}
