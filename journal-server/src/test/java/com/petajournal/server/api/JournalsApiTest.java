package com.petajournal.server.api;

import io.quarkus.test.junit.QuarkusTest;
import io.quarkus.test.junit.TestProfile;
import io.restassured.path.json.JsonPath;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

/** Journal pages are named after their day or week; the tests use 1999 so that nothing else is near. */
@QuarkusTest
@TestProfile(IntegrationTestProfile.class)
class JournalsApiTest extends ApiTestSupport {

    private static JsonPath journal(String name) {
        return json().post("/api/journals/" + name).then().statusCode(200).extract().jsonPath();
    }

    @Test
    void aDayIsCreatedWithOneEmptyItemAndFoundAgainByItsDate() {
        JsonPath first = journal("1999-06-15");
        JsonPath again = journal("1999-06-15");

        assertTrue(first.getBoolean("created"));
        assertEquals("journal", first.getString("page.type"));
        assertEquals("1999-06-15", first.getString("page.name"));
        assertEquals(List.of(""), first.getList("items.body"));
        assertFalse(again.getBoolean("created"));
        assertEquals(first.getString("page.id"), again.getString("page.id"));
    }

    @Test
    void theMostRecentDayBeforeADateIsFound() {
        journal("1999-03-05");
        journal("1999-03-10");

        JsonPath before = json().get("/api/journals/before?date=1999-03-10").then().statusCode(200).extract().jsonPath();
        JsonPath nothing = json().get("/api/journals/before?date=1999-01-01").then().statusCode(200).extract().jsonPath();

        assertTrue(before.getBoolean("found"));
        assertEquals("1999-03-05", before.getString("page.name"));
        assertEquals(1, before.getList("items").size());
        assertFalse(nothing.getBoolean("found"));
    }

    @Test
    void aWeekIsCreatedWithOneLinkPerDay() {
        JsonPath week = journal("1999-W10");

        assertTrue(week.getBoolean("created"));
        List<String> bodies = week.getList("items.body");
        assertEquals(7, bodies.size());
        assertEquals("[[1999-03-08]]", bodies.get(0));  // ISO week 10 of 1999 starts on Monday the 8th
        assertEquals("[[1999-03-14]]", bodies.get(6));
    }

    @Test
    void theMostRecentWeekBeforeAWeekIsFound() {
        journal("1999-W20");
        journal("1999-W22");

        JsonPath before = json().get("/api/journals/weekly/before?week=1999-W22").then().statusCode(200).extract().jsonPath();

        assertTrue(before.getBoolean("found"));
        assertEquals("1999-W20", before.getString("page.name"));
    }

    @Test
    void aNameThatIsNotAJournalIsRefused() {
        json().post("/api/journals/not-a-date").then().statusCode(400);
    }

    @Test
    void anItemMovesToAnotherDay() {
        String from = journal("1999-09-01").getString("page.id");
        String to = journal("1999-09-02").getString("page.id");
        ItemRow moved = fill(from, "carry over").get(0);

        moveItems(List.of(moved.id()), to, "last", null);

        assertEquals(List.of(), outlineOf(from));
        assertEquals(List.of("1:", "1:carry over"), outlineOf(to));
    }
}
