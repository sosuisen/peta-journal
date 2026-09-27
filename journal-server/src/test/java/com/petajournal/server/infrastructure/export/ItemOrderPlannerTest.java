package com.petajournal.server.infrastructure.export;

import org.junit.jupiter.api.Test;

import java.util.Arrays;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class ItemOrderPlannerTest {

    private static void assertStrictlyIncreasing(List<String> idx) {
        for (int i = 1; i < idx.size(); i++) {
            assertTrue(idx.get(i - 1).compareTo(idx.get(i)) < 0,
                    idx.get(i - 1) + " should sort before " + idx.get(i));
        }
    }

    @Test
    void unchangedOrderKeepsEveryIndex() {
        List<String> existing = List.of("a0", "a1", "a2");
        assertEquals(existing, ItemOrderPlanner.plan(existing));
    }

    @Test
    void newItemsGetIndicesBetweenNeighbours() {
        List<String> plan = ItemOrderPlanner.plan(Arrays.asList("a0", null, "a2", null));
        assertEquals("a0", plan.get(0));
        assertEquals("a2", plan.get(2));
        assertStrictlyIncreasing(plan);
    }

    @Test
    void movedItemIsReindexedOthersKept() {
        // File order: a2 moved to the top
        List<String> plan = ItemOrderPlanner.plan(Arrays.asList("a2", "a0", "a1"));
        assertEquals("a0", plan.get(1));
        assertEquals("a1", plan.get(2));
        assertNotEquals("a2", plan.get(0));
        assertStrictlyIncreasing(plan);
    }

    @Test
    void allNewItems() {
        List<String> plan = ItemOrderPlanner.plan(Arrays.asList(null, null, null));
        assertEquals(3, plan.size());
        assertStrictlyIncreasing(plan);
    }

    @Test
    void emptyInput() {
        assertTrue(ItemOrderPlanner.plan(List.of()).isEmpty());
    }
}
