package com.petajournal.server.api;

import io.quarkus.test.junit.QuarkusTestProfile;

import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;

/**
 * Runs the whole server against a throw-away directory: the database, the search index,
 * the markdown folder and the config file all live under one temporary root, so the
 * tests never touch {@code ~/.peta-journal}.
 *
 * The markdown folder is written into the config file before the server starts; without it
 * the startup would open the native folder chooser.
 */
public class IntegrationTestProfile implements QuarkusTestProfile {

    /** One root per JVM; every test class that uses this profile shares the running server. */
    static final Path ROOT;
    static final Path MARKDOWN_DIR;

    static {
        try {
            ROOT = Files.createTempDirectory("peta-journal-it-");
            MARKDOWN_DIR = Files.createDirectories(ROOT.resolve("markdown"));
            Path configDir = Files.createDirectories(ROOT.resolve("config"));
            String markdownDirJson = MARKDOWN_DIR.toString().replace("\\", "\\\\");
            Files.writeString(configDir.resolve("config.json"),
                    "{\"markdownDir\": \"" + markdownDirJson + "\", \"weekNumbering\": \"iso\"}");
        } catch (IOException e) {
            throw new UncheckedIOException(e);
        }
    }

    @Override
    public Map<String, String> getConfigOverrides() {
        return Map.of(
                "petajournal.data.dir", ROOT.resolve("data").toString(),
                "petajournal.data.db", ROOT.resolve("data/journal.db").toString(),
                "petajournal.index.dir", ROOT.resolve("index").toString(),
                "petajournal.export.dir", MARKDOWN_DIR.toString(),
                "petajournal.config.dir", ROOT.resolve("config").toString(),
                "petajournal.export.interval", "1");
    }
}
