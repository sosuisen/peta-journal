package com.petajournal.server.api;

import com.sosuisha.hmtid.HMTID;
import io.quarkus.test.junit.QuarkusTest;
import io.quarkus.test.junit.TestProfile;
import io.restassured.path.json.JsonPath;
import org.junit.jupiter.api.Test;

import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

@QuarkusTest
@TestProfile(IntegrationTestProfile.class)
class ItemsApiTest extends ApiTestSupport {

    // ---- Creating ----

    @Test
    void itemsGoFirstLastOrAfterAnotherItem() {
        PageRef page = newDocPage("items-create");
        List<ItemRow> rows = fill(page.id(), "a", "b");

        createItem(page.id(), "first", 1, "x");
        createItem(page.id(), "last", 1, "y");
        createItem(page.id(), "after:" + rows.get(0).id(), 2, "z");

        assertEquals(List.of("1:x", "1:a", "2:z", "1:b", "1:y"), outlineOf(page.id()));
    }

    @Test
    void aClientIdIsKept() {
        PageRef page = newDocPage("items-client-id");
        String id = HMTID.monotonicFactory().generate().toString();  // what the frontend makes up for an optimistic item

        ItemRow item = ItemRow.of(json().body(Map.of("pageId", page.id(), "position", "last", "level", 1, "body", "mine", "id", id))
                .post("/api/items").then().statusCode(200).extract().jsonPath().getMap("item"));

        assertEquals(id, item.id());
    }

    // ---- Updating ----

    @Test
    void theBodyIsParsedIntoAnAstAndTypes() {
        PageRef page = newDocPage("items-parse");
        ItemRow plain = fill(page.id(), "plain").get(0);

        ItemRow heading = updateItem(plain.id(), Map.of("body", "## Title"));

        assertTrue(heading.types().contains("heading"), "types: " + heading.types());
        assertTrue(json().get("/api/items/" + plain.id()).then().statusCode(200)
                .extract().jsonPath().getString("item.ast").contains("\"heading\""));
    }

    @Test
    void levelAndCollapsedStateAreSaved() {
        PageRef page = newDocPage("items-flags");
        List<ItemRow> rows = fill(page.id(), "p", "  c");

        updateItem(rows.get(0).id(), Map.of("collapsed", true));
        updateItem(rows.get(1).id(), Map.of("level", 3));

        List<ItemRow> after = items(page.id());
        assertTrue(after.get(0).collapsed());
        assertEquals(3, after.get(1).level());
    }

    @Test
    void aPropertyCanBeSetOnAnItem() {
        PageRef page = newDocPage("items-property");
        ItemRow item = fill(page.id(), "task-like").get(0);

        json().body(Map.of("valueType", "date", "value", "2026-03-01"))
                .put("/api/items/" + item.id() + "/properties/plan").then().statusCode(200);

        assertEquals("2026-03-01", items(page.id()).get(0).properties().get("plan"));
    }

    @Test
    void aDeletedItemIsGoneFromItsPage() {
        PageRef page = newDocPage("items-delete");
        List<ItemRow> rows = fill(page.id(), "keep", "drop");

        json().delete("/api/items/" + rows.get(1).id()).then().statusCode(204);

        assertEquals(List.of("1:keep"), outlineOf(page.id()));
    }

    // ---- Levels ----

    @Test
    void shiftLevelMovesAGroupByARelativeAmount() {
        PageRef page = newDocPage("items-shift");
        List<ItemRow> rows = fill(page.id(), "a", "b", "  b1", "c");

        json().body(Map.of("itemIds", List.of(rows.get(1).id(), rows.get(2).id()), "delta", 1))
                .post("/api/items/shift-level").then().statusCode(200);

        assertEquals(List.of("1:a", "2:b", "3:b1", "1:c"), outlineOf(page.id()));
    }

    // ---- Moving ----

    @Test
    void aSubtreeMovesPastItsSiblingOnTheSamePage() {
        PageRef page = newDocPage("items-move");
        List<ItemRow> rows = fill(page.id(), "a", "b", "  b1", "c");

        moveItems(List.of(rows.get(1).id(), rows.get(2).id()), page.id(), "after:" + rows.get(3).id(), null);
        assertEquals(List.of("1:a", "1:c", "1:b", "2:b1"), outlineOf(page.id()));

        moveItems(List.of(rows.get(1).id(), rows.get(2).id()), page.id(), "first", null);
        assertEquals(List.of("1:b", "2:b1", "1:a", "1:c"), outlineOf(page.id()));
    }

    @Test
    void itemsMoveToAnotherPageWithTheirLevelsShifted() {
        PageRef from = newDocPage("items-move-from");
        PageRef to = newDocPage("items-move-to");
        List<ItemRow> rows = fill(from.id(), "stay", "go", "  go-child");
        ItemRow target = fill(to.id(), "parent").get(0);

        List<ItemRow> moved = moveItems(List.of(rows.get(1).id(), rows.get(2).id()), to.id(), "after:" + target.id(), 1);

        assertEquals(List.of("2:go", "3:go-child"), moved.stream().map(ItemRow::outline).toList());
        assertEquals(List.of("1:stay"), outlineOf(from.id()));
        assertEquals(List.of("1:parent", "2:go", "3:go-child"), outlineOf(to.id()));
    }

    // ---- Tasks ----

    @Test
    void cycleTaskGoesThroughTaskRunningDoneAndBackToNormal() {
        PageRef page = newDocPage("items-task");
        ItemRow item = fill(page.id(), "buy milk").get(0);
        String cycle = "/api/items/" + item.id() + "/cycle-task";

        json().post(cycle).then().statusCode(200);
        ItemRow task = items(page.id()).get(0);
        assertTrue(task.types().contains("task"));
        assertFalse(task.properties().containsKey("begin"));

        // The response carries the times it just set, so the frontend can show them at once
        ItemRow running = ItemRow.of(json().post(cycle).then().statusCode(200).extract().jsonPath().getMap("item"));
        assertTrue(running.properties().containsKey("begin"), "response: " + running.properties());
        assertTrue(items(page.id()).get(0).properties().containsKey("begin"));

        ItemRow done = ItemRow.of(json().post(cycle).then().statusCode(200).extract().jsonPath().getMap("item"));
        assertTrue(done.properties().containsKey("end"), "response: " + done.properties());
        assertTrue(items(page.id()).get(0).properties().containsKey("end"));

        json().post(cycle).then().statusCode(200);
        ItemRow normal = items(page.id()).get(0);
        assertFalse(normal.types().contains("task"));
        assertFalse(normal.properties().containsKey("begin"));
        assertFalse(normal.properties().containsKey("end"));
    }

    @Test
    void tasksCanBeListedByType() {
        PageRef page = newDocPage("items-task-list");
        ItemRow item = fill(page.id(), "listed task").get(0);
        json().post("/api/items/" + item.id() + "/cycle-task").then().statusCode(200);

        List<String> ids = json().get("/api/items?types=task").then().statusCode(200).extract().jsonPath().getList("items.id");

        assertTrue(ids.contains(item.id()));
    }

    // ---- Transclusion ----

    @Test
    void aTransclusionShowsTheSourceTreeWithThePage() {
        PageRef sourcePage = newDocPage("items-transclusion-source");
        List<ItemRow> source = fill(sourcePage.id(), "root", "  child");
        PageRef page = newDocPage("items-transclusion");
        ItemRow host = fill(page.id(), "placeholder").get(0);

        ItemRow linked = ItemRow.of(json().body(Map.of("sourceItemId", source.get(0).id()))
                .post("/api/items/" + host.id() + "/transclude").then().statusCode(200).extract().jsonPath().getMap("item"));
        assertEquals(source.get(0).id(), linked.transcludeItemId());

        JsonPath resp = json().get("/api/pages/" + page.id()).then().statusCode(200).extract().jsonPath();
        assertEquals(List.of("root", "child"), resp.getList("transcludedTrees." + source.get(0).id() + ".body"));

        json().delete("/api/items/" + host.id() + "/transclude").then().statusCode(200);
        assertNull(items(page.id()).get(0).transcludeItemId());
    }
}
