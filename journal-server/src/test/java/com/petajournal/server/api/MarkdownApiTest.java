package com.petajournal.server.api;

import io.quarkus.test.junit.QuarkusTest;
import io.quarkus.test.junit.TestProfile;
import io.restassured.path.json.JsonPath;
import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

/** The markdown conversions the frontend uses for copy, paste and the clipboard. */
@QuarkusTest
@TestProfile(IntegrationTestProfile.class)
class MarkdownApiTest extends ApiTestSupport {

    private static JsonPath parse(String markdown) {
        return json().body(Map.of("markdown", markdown)).post("/api/items/parse").then().statusCode(200).extract().jsonPath();
    }

    @Test
    void flatMarkdownIsSplitIntoItemsAtBlockBoundaries() {
        JsonPath resp = parse("# Night\n\n- [ ] Hanon\n\n- [ ] Upload\n");

        assertEquals(List.of("# Night", "- [ ] Hanon\n\n- [ ] Upload"), resp.getList("items.body"));
        assertEquals(List.of(1, 1), resp.getList("items.level"));
    }

    @Test
    void metadataLinesCarryTheLevelOfEachItem() {
        JsonPath resp = parse("# Night\n\n<!-- @@ -->\n- [ ] Hanon\n\n<!-- @@ -->\n- [ ] Upload\n");

        assertEquals(List.of("# Night", "- [ ] Hanon", "- [ ] Upload"), resp.getList("items.body"));
        assertEquals(List.of(1, 2, 2), resp.getList("items.level"));
    }

    @Test
    void exportedItemsComeBackThroughImportAndParseUnchanged() {
        PageRef page = newDocPage("markdown-roundtrip");
        List<ItemRow> rows = fill(page.id(), "parent", "  child one", "    grandchild", "  child two", "sibling");

        JsonPath exported = json().body(Map.of("itemIds", rows.stream().map(ItemRow::id).toList()))
                .post("/api/items/export").then().statusCode(200).extract().jsonPath();
        String treeMarkdown = exported.getString("treeMarkdown");
        assertTrue(treeMarkdown.contains("- parent"), treeMarkdown);
        assertTrue(treeMarkdown.contains("    - grandchild"), treeMarkdown);
        assertTrue(exported.getString("ast").contains("\"type\""));
        assertTrue(exported.getString("flatMarkdown").contains("<!-- @"), "flat markdown keeps the item metadata");

        String flatMarkdown = json().body(Map.of("treeMarkdown", treeMarkdown))
                .post("/api/items/import").then().statusCode(200).extract().jsonPath().getString("flatMarkdown");
        JsonPath parsed = parse(flatMarkdown);

        assertEquals(rows.stream().map(ItemRow::body).toList(), parsed.getList("items.body"));
        assertEquals(rows.stream().map(ItemRow::level).toList(), parsed.getList("items.level"));
    }

    @Test
    void aWholePageExportsLikeItsItems() {
        PageRef page = newDocPage("markdown-page");
        List<ItemRow> rows = fill(page.id(), "one", "  two");

        String byPage = json().queryParam("pageId", page.id()).post("/api/items/export-page").then().statusCode(200)
                .extract().jsonPath().getString("treeMarkdown");
        String byItems = json().body(Map.of("itemIds", rows.stream().map(ItemRow::id).toList()))
                .post("/api/items/export").then().statusCode(200).extract().jsonPath().getString("treeMarkdown");

        assertEquals(byItems, byPage);
    }
}
