package com.petajournal.server.api;

import io.quarkus.test.junit.QuarkusTest;
import io.quarkus.test.junit.TestProfile;
import io.restassured.path.json.JsonPath;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

@QuarkusTest
@TestProfile(IntegrationTestProfile.class)
class SearchApiTest extends ApiTestSupport {

    private static JsonPath search(String path, String query) {
        return json().queryParam("q", query).queryParam("maxResults", 10).get(path).then().statusCode(200).extract().jsonPath();
    }

    @Test
    void pageNamesAreFoundByPrefix() {
        PageRef page = newDocPage("Searchable");

        List<String> titles = search("/api/search/pages", page.title().substring(0, 12)).getList("results.pageTitle");

        assertTrue(titles.contains(page.title()), "titles: " + titles);
    }

    @Test
    void itemTextIsFoundWithASnippet() {
        PageRef page = newDocPage("search-text");
        String word = "zebra" + System.nanoTime();
        ItemRow item = fill(page.id(), "the " + word + " crossing", "nothing here").get(0);

        JsonPath resp = search("/api/search/combined", word);

        assertEquals(List.of(item.id()), resp.getList("results.itemId"));
        assertEquals(page.id(), resp.getString("results[0].pageId"));
        // The analyzer cuts letters from digits, so the match is marked word by word
        assertTrue(resp.getString("results[0].snippet").contains("<mark>zebra"), resp.getString("results[0].snippet"));
    }

    @Test
    void twoWordsMustBothBeInTheItem() {
        PageRef page = newDocPage("search-and");
        String a = "apple" + System.nanoTime();
        String b = "banana" + System.nanoTime();
        List<ItemRow> rows = fill(page.id(), a + " and " + b, a + " alone");

        List<String> ids = search("/api/search/combined", a + " " + b).getList("results.itemId");

        assertEquals(List.of(rows.get(0).id()), ids);
    }

    @Test
    void aHashtagInTheBodyIsFoundAsATag() {
        PageRef page = newDocPage("search-tag");
        String tag = "tag" + System.nanoTime();
        ItemRow item = fill(page.id(), "remember #" + tag).get(0);

        assertEquals(List.of(tag), items(page.id()).get(0).properties().get("tag"));
        assertEquals(List.of(item.id()), search("/api/search/tags", tag).getList("results.itemId"));
    }

    @Test
    void aDeletedItemIsNoLongerFound() {
        PageRef page = newDocPage("search-deleted");
        String word = "ghost" + System.nanoTime();
        ItemRow item = fill(page.id(), word, "stays").get(0);

        json().delete("/api/items/" + item.id()).then().statusCode(204);

        assertEquals(List.of(), search("/api/search/combined", word).getList("results.itemId"));
    }
}
