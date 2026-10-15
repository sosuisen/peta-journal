package com.petajournal.server.api;

import io.quarkus.test.junit.QuarkusTest;
import io.quarkus.test.junit.TestProfile;
import io.restassured.path.json.JsonPath;
import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

@QuarkusTest
@TestProfile(IntegrationTestProfile.class)
class PagesApiTest extends ApiTestSupport {

    @Test
    void aNewDocPageStartsWithOneEmptyItemAndIsFoundAgainByItsTitle() {
        String title = "pages-new-" + System.nanoTime();

        JsonPath first = json().body(Map.of("title", title, "dirPath", ""))
                .post("/api/pages/doc").then().statusCode(200).extract().jsonPath();
        JsonPath again = json().body(Map.of("title", title, "dirPath", ""))
                .post("/api/pages/doc").then().statusCode(200).extract().jsonPath();

        assertTrue(first.getBoolean("created"));
        assertEquals(List.of(""), first.getList("items.body"));
        assertEquals("doc", first.getString("page.type"));
        assertFalse(again.getBoolean("created"));
        assertEquals(first.getString("page.id"), again.getString("page.id"));
    }

    @Test
    void getPageReturnsThePageWithItsItemsInOrder() {
        PageRef page = newDocPage("pages-get");
        fill(page.id(), "one", "  two", "three");

        JsonPath resp = json().get("/api/pages/" + page.id()).then().statusCode(200).extract().jsonPath();

        assertEquals(page.title(), resp.getString("page.name"));
        assertEquals(List.of("one", "two", "three"), resp.getList("items.body"));
        assertEquals(List.of(1, 2, 1), resp.getList("items.level"));
    }

    @Test
    void renamingAPageChangesItsTitleAndPath() {
        PageRef page = newDocPage("pages-rename");

        JsonPath resp = json().body(Map.of("title", page.title() + "-renamed", "dirPath", "notes/"))
                .put("/api/pages/" + page.id()).then().statusCode(200).extract().jsonPath();

        assertEquals(page.title() + "-renamed", resp.getString("page.name"));
        assertEquals("notes/", resp.getString("page.dirPath"));
        assertEquals(page.title() + "-renamed",
                json().get("/api/pages/" + page.id()).then().statusCode(200).extract().jsonPath().getString("page.name"));
    }

    @Test
    void theMostRecentlyTouchedPageComesFirstInRecentPages() {
        PageRef older = newDocPage("pages-recent-older");
        PageRef newer = newDocPage("pages-recent-newer");

        List<String> ids = json().get("/api/pages/recent?maxResults=5&type=doc")
                .then().statusCode(200).extract().jsonPath().getList("pages.id");

        assertEquals(newer.id(), ids.get(0));
        assertTrue(ids.indexOf(newer.id()) < ids.indexOf(older.id()));
    }

    @Test
    void aDeletedPageIsGone() {
        PageRef page = newDocPage("pages-delete");

        json().delete("/api/pages/" + page.id()).then().statusCode(204);

        json().get("/api/pages/" + page.id()).then().statusCode(404);
    }

    @Test
    void aWikiLinkToAPageShowsUpInItsBacklinks() {
        PageRef target = newDocPage("pages-target");
        PageRef source = newDocPage("pages-source");
        fill(source.id(), "see [[" + target.title() + "]]");

        JsonPath resp = json().get("/api/pages/" + target.id() + "/backlinks").then().statusCode(200).extract().jsonPath();

        assertEquals(List.of(source.id()), resp.getList("entries.sourcePage.id"));
        assertEquals("see [[" + target.title() + "]]", resp.getString("entries[0].sourceItem.body"));
    }
}
