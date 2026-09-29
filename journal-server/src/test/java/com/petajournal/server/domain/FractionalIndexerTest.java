package com.petajournal.server.domain;

import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class FractionalIndexerTest {

    private static void assertAscendingBetween(String before, String after, List<String> indices) {
        String previous = before;
        for (String idx : indices) {
            if (previous != null) {
                assertTrue(previous.compareTo(idx) < 0, previous + " should sort before " + idx);
            }
            previous = idx;
        }
        if (after != null && previous != null) {
            assertTrue(previous.compareTo(after) < 0, previous + " should sort before " + after);
        }
    }

    @Test
    void groupStaysInsideANarrowGap() {
        // 17 items after "z0" used to reach "z0V", the index of the next item
        List<String> indices = FractionalIndexer.between("z0", "z0V", 40);

        assertEquals(40, indices.size());
        assertAscendingBetween("z0", "z0V", indices);
    }

    @Test
    void groupStaysInsideAGapOfAdjacentIndices() {
        List<String> indices = FractionalIndexer.between("z0U", "z0V", 25);

        assertAscendingBetween("z0U", "z0V", indices);
    }

    @Test
    void gapsStayUsableAfterManyMoves() {
        String before = "z0";
        String after = "z1";
        for (int round = 0; round < 50; round++) {
            List<String> indices = FractionalIndexer.between(before, after, 5);
            assertAscendingBetween(before, after, indices);
            after = indices.getFirst();
        }
    }

    @Test
    void groupAtTheEndSortsAfterTheLastItem() {
        List<String> indices = FractionalIndexer.between("z3", null, 70);

        assertEquals(70, indices.size());
        assertAscendingBetween("z3", null, indices);
    }

    @Test
    void groupAtTheTopSortsBeforeTheFirstItem() {
        List<String> indices = FractionalIndexer.between(null, "z0", 20);

        assertEquals(20, indices.size());
        assertAscendingBetween(null, "z0", indices);
    }

    @Test
    void zeroItemsNeedNoIndex() {
        assertEquals(new ArrayList<String>(), FractionalIndexer.between("z0", "z1", 0));
    }

    @Test
    void missingGapIsReported() {
        assertThrows(IllegalStateException.class, () -> FractionalIndexer.between("z1", "z0", 1));
        assertThrows(IllegalStateException.class, () -> FractionalIndexer.between("z1", "z1", 1));
    }

    @Test
    void negativeCountIsRejected() {
        assertThrows(IllegalArgumentException.class, () -> FractionalIndexer.between("z0", "z1", -1));
    }
}
