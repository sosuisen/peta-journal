package com.petajournal.server.api;

import io.restassured.http.ContentType;
import io.restassured.path.json.JsonPath;
import io.restassured.specification.RequestSpecification;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

import static io.restassured.RestAssured.given;

/**
 * Talks to the running server over HTTP the way the frontend does. Each test creates its
 * own pages, so the tests do not depend on each other or on their order.
 */
abstract class ApiTestSupport {

    /** An item as the server returns it; only the fields the tests look at. */
    record ItemRow(String id, int level, String body, List<String> types, Map<String, Object> properties,
                   boolean collapsed, String transcludeItemId) {
        static ItemRow of(Map<String, Object> json) {
            @SuppressWarnings("unchecked")
            List<String> types = (List<String>) json.getOrDefault("types", List.of());
            @SuppressWarnings("unchecked")
            Map<String, Object> properties = (Map<String, Object>) json.getOrDefault("properties", Map.of());
            return new ItemRow((String) json.get("id"), (Integer) json.get("level"), (String) json.get("body"),
                    types, properties, Boolean.TRUE.equals(json.get("collapsed")), (String) json.get("transcludeItemId"));
        }

        /** "level:body", the shape most assertions compare. */
        String outline() {
            return level + ":" + body;
        }
    }

    record PageRef(String id, String title) {}

    private static long counter = System.currentTimeMillis();

    static RequestSpecification json() {
        return given().contentType(ContentType.JSON).accept(ContentType.JSON);
    }

    /** Creates an empty doc page with a unique title; a new page starts with one empty item. */
    static PageRef newDocPage(String prefix) {
        String title = prefix + "-" + (counter++);
        JsonPath resp = json().body(Map.of("title", title, "dirPath", ""))
                .post("/api/pages/doc").then().statusCode(200).extract().jsonPath();
        return new PageRef(resp.getString("page.id"), title);
    }

    static List<ItemRow> items(String pageId) {
        List<Map<String, Object>> rows = json().get("/api/pages/" + pageId)
                .then().statusCode(200).extract().jsonPath().getList("items");
        return rows.stream().map(ItemRow::of).toList();
    }

    static List<String> outlineOf(String pageId) {
        return items(pageId).stream().map(ItemRow::outline).collect(Collectors.toList());
    }

    static ItemRow createItem(String pageId, String position, int level, String body) {
        Map<String, Object> req = new HashMap<>(Map.of("pageId", pageId, "position", position, "level", level, "body", body));
        req.put("id", "");
        return ItemRow.of(json().body(req).post("/api/items").then().statusCode(200).extract().jsonPath().getMap("item"));
    }

    /**
     * Fills a page with items given as "body" or "  body" (two spaces per level below 1),
     * replacing the empty item a new page starts with, and returns them in page order.
     */
    static List<ItemRow> fill(String pageId, String... lines) {
        ItemRow first = items(pageId).get(0);
        String after = first.id();
        for (String line : lines) {
            String body = line.stripLeading();
            int level = 1 + (line.length() - body.length()) / 2;
            after = createItem(pageId, "after:" + after, level, body).id();
        }
        json().delete("/api/items/" + first.id()).then().statusCode(204);
        return items(pageId);
    }

    static ItemRow updateItem(String itemId, Map<String, Object> changes) {
        return ItemRow.of(json().body(changes).put("/api/items/" + itemId).then().statusCode(200).extract().jsonPath().getMap("item"));
    }

    static List<ItemRow> moveItems(List<String> itemIds, String targetPageId, String position, Integer levelDelta) {
        Map<String, Object> req = new HashMap<>(Map.of("itemIds", itemIds, "targetPageId", targetPageId, "position", position));
        if (levelDelta != null) req.put("levelDelta", levelDelta);
        List<Map<String, Object>> rows = json().body(req).post("/api/items/move").then().statusCode(200).extract().jsonPath().getList("items");
        return rows.stream().map(ItemRow::of).toList();
    }

    static ItemRow byBody(List<ItemRow> rows, String body) {
        return rows.stream().filter(r -> r.body().equals(body)).findFirst().orElseThrow();
    }
}
