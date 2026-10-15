package com.petajournal.server.api;

import io.quarkus.test.junit.QuarkusTest;
import io.quarkus.test.junit.TestProfile;
import io.restassured.path.json.JsonPath;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.time.Instant;
import java.util.List;
import java.util.function.BooleanSupplier;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

/**
 * The markdown folder next to the database: pages are written to it by the export
 * scheduler, and changes made to the files are merged back by a scan.
 */
@QuarkusTest
@TestProfile(IntegrationTestProfile.class)
class FolderSyncTest extends ApiTestSupport {

    private static final Path ROOT = IntegrationTestProfile.MARKDOWN_DIR;

    private static Path fileOf(PageRef page) {
        return ROOT.resolve("docs").resolve(page.title() + ".md");
    }

    /** The export scheduler starts a few seconds after the server and then runs every second. */
    private static void waitUntil(String what, BooleanSupplier condition) throws InterruptedException {
        Instant deadline = Instant.now().plus(Duration.ofSeconds(20));
        while (!condition.getAsBoolean()) {
            assertTrue(Instant.now().isBefore(deadline), "timed out waiting until " + what);
            Thread.sleep(200);
        }
    }

    private static String read(Path file) {
        try {
            return Files.readString(file);
        } catch (IOException e) {
            return "";
        }
    }

    private static JsonPath scan() {
        return json().post("/api/sync/scan").then().statusCode(200).extract().jsonPath();
    }

    @Test
    void theStatusNamesTheFolderAndSaysItIsThere() {
        JsonPath status = json().get("/api/sync/status").then().statusCode(200).extract().jsonPath();

        assertTrue(status.getBoolean("importEnabled"));
        assertEquals(ROOT.toString(), status.getString("markdownRoot"));
        assertTrue(status.getBoolean("rootAvailable"));
    }

    @Test
    void aPageIsWrittenToItsMarkdownFile() throws InterruptedException {
        PageRef page = newDocPage("sync-export");
        fill(page.id(), "first line", "  second line");

        waitUntil("the page is exported", () -> read(fileOf(page)).contains("second line"));

        String markdown = read(fileOf(page));
        assertTrue(markdown.startsWith("---\nid: " + page.id() + "\n"), markdown);
        assertTrue(markdown.contains("first line"), markdown);
    }

    @Test
    void aChangeMadeToTheFileIsMergedIntoThePage() throws Exception {
        PageRef page = newDocPage("sync-import");
        fill(page.id(), "before the edit");
        waitUntil("the page is exported", () -> read(fileOf(page)).contains("before the edit"));

        Files.writeString(fileOf(page), read(fileOf(page)).replace("before the edit", "edited on disk, a bit longer"));
        JsonPath result = scan();

        assertEquals(1, result.getInt("imported"));
        assertEquals(List.of("1:edited on disk, a bit longer"), outlineOf(page.id()));
    }

    @Test
    void aNewFileInTheFolderBecomesAPage() throws Exception {
        String name = "sync-new-" + System.nanoTime();
        Files.createDirectories(ROOT.resolve("docs"));
        Files.writeString(ROOT.resolve("docs").resolve(name + ".md"), "hello from disk\n\nsecond block\n");

        JsonPath result = scan();

        assertTrue(result.getInt("imported") >= 1);
        List<String> ids = json().queryParam("q", name).get("/api/search/pages").then().statusCode(200)
                .extract().jsonPath().getList("results.pageId");
        assertEquals(1, ids.size(), "the new page is found by its file name");
        assertEquals(List.of("1:hello from disk", "1:second block"), outlineOf(ids.get(0)));
    }

    @Test
    void anUnchangedFileIsLeftAlone() throws InterruptedException {
        PageRef page = newDocPage("sync-unchanged");
        fill(page.id(), "stable");
        waitUntil("the page is exported", () -> read(fileOf(page)).contains("stable"));
        scan();

        JsonPath again = scan();

        assertEquals(0, again.getInt("imported"));
        assertEquals(0, again.getInt("conflicts"));
        assertEquals(List.of("1:stable"), outlineOf(page.id()));
    }
}
