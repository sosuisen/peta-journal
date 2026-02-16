package com.petajournal.server.infrastructure.startup;

import com.fasterxml.jackson.databind.ObjectMapper;
import io.quarkus.runtime.StartupEvent;
import jakarta.annotation.Priority;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.enterprise.event.Observes;
import org.eclipse.microprofile.config.inject.ConfigProperty;
import org.jboss.logging.Logger;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Map;

@ApplicationScoped
public class SchemaVersionCheck {

    /** Bump this constant whenever the DB schema changes. */
    public static final String SCHEMA_VERSION = "0.1";

    private static final Logger LOG = Logger.getLogger(SchemaVersionCheck.class);
    private static final String VERSION_FILE = "versions.json";
    private static final ObjectMapper MAPPER = new ObjectMapper();

    @ConfigProperty(name = "petajournal.config.dir")
    String configDir;

    /**
     * Runs after DatabaseInitializer (priority 1) but early in startup.
     * Reads or creates schema-version.json and warns on mismatch.
     */
    void onStartup(@Observes @Priority(10) StartupEvent event) {
        Path versionFile = Paths.get(configDir, VERSION_FILE);

        LOG.infov("Schema version (embedded): {0}", SCHEMA_VERSION);

        if (Files.exists(versionFile)) {
            try {
                byte[] bytes = Files.readAllBytes(versionFile);
                @SuppressWarnings("unchecked")
                Map<String, Object> json = MAPPER.readValue(bytes, Map.class);
                String fileVersion = (String) json.get("schemaVersion");
                LOG.infov("Schema version (file):     {0}", fileVersion);

                if (!SCHEMA_VERSION.equals(fileVersion)) {
                    LOG.warnv("Schema version MISMATCH! embedded={0}, file={1}. " +
                            "A database migration may be required.", SCHEMA_VERSION, fileVersion);
                }
            } catch (IOException e) {
                LOG.error("Failed to read " + VERSION_FILE, e);
            }
        } else {
            LOG.infov("Schema version file not found. Creating {0}", versionFile);
            writeVersionFile(versionFile);
        }
    }

    private void writeVersionFile(Path versionFile) {
        try {
            Files.createDirectories(versionFile.getParent());
            Map<String, Object> json = Map.of("schemaVersion", SCHEMA_VERSION);
            byte[] bytes = MAPPER.writerWithDefaultPrettyPrinter().writeValueAsBytes(json);
            Files.write(versionFile, bytes);
            LOG.infov("Wrote {0}", versionFile);
        } catch (IOException e) {
            LOG.error("Failed to write " + VERSION_FILE, e);
        }
    }
}
