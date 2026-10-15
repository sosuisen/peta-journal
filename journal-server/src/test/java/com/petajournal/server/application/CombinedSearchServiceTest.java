package com.petajournal.server.application;

import com.petajournal.server.application.CombinedSearchService.QueryTerms;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class CombinedSearchServiceTest {

    @Test
    void plainWordsAreTheTextQuery() {
        QueryTerms terms = QueryTerms.parse("hello  world");

        assertEquals("hello world", terms.text());
        assertFalse(terms.hasFilters());
    }

    @Test
    void eachFilterKindIsRecognizedByItsPrefix() {
        QueryTerms terms = QueryTerms.parse("#urgent in:Projects/ created:2026-03 modified:>2026-03-01 status=done report");

        assertEquals(List.of("urgent"), terms.tags());
        assertEquals(List.of("Projects/"), terms.pagePrefixes());
        assertEquals(2, terms.dateFilters().size());
        assertArrayEquals(new String[]{"created_date", "2026-03"}, terms.dateFilters().get(0));
        assertArrayEquals(new String[]{"modified_date", ">2026-03-01"}, terms.dateFilters().get(1));
        assertEquals(1, terms.propertyFilters().size());
        assertArrayEquals(new String[]{"status", "done"}, terms.propertyFilters().get(0));
        assertEquals("report", terms.text());
        assertTrue(terms.hasFilters());
    }

    @Test
    void aPrefixWithoutAValueIsOrdinaryText() {
        QueryTerms terms = QueryTerms.parse("# in: created: =x a==b");

        assertFalse(terms.hasFilters());
        assertEquals("# in: created: =x a==b", terms.text());
    }
}
