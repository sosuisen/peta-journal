package com.petajournal.server.application;

import com.fasterxml.jackson.databind.ObjectMapper;
import io.quarkus.runtime.StartupEvent;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.enterprise.event.Observes;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.io.IOException;
import java.nio.file.DirectoryStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;

@ApplicationScoped
public class ExtensionFunctionService {

    private static final Logger LOG = Logger.getLogger(ExtensionFunctionService.class);
    private static final String FUNCTIONS_DIR = "functions";
    private static final String CONFIG_FILE = "config.json";

    @Inject
    ConfigService configService;

    private final ObjectMapper objectMapper = new ObjectMapper();
    private List<ExtensionFunction> functions = Collections.emptyList();

    /** An extension function loaded from the functions directory. */
    public record ExtensionFunction(String dirName, String name) {}

    void onStart(@Observes StartupEvent ev) {
        Path functionsDir = Paths.get(configService.getConfigDir(), FUNCTIONS_DIR);
        if (!Files.isDirectory(functionsDir)) {
            LOG.infov("Functions directory not found: {0}", functionsDir);
            return;
        }

        List<ExtensionFunction> found = new ArrayList<>();
        try (DirectoryStream<Path> stream = Files.newDirectoryStream(functionsDir)) {
            for (Path entry : stream) {
                if (!Files.isDirectory(entry)) continue;
                Path configPath = entry.resolve(CONFIG_FILE);
                if (!Files.exists(configPath)) {
                    LOG.warnv("No config.json in function directory: {0}", entry.getFileName());
                    continue;
                }
                try {
                    @SuppressWarnings("unchecked")
                    Map<String, Object> config = objectMapper.readValue(configPath.toFile(), Map.class);
                    String name = (String) config.getOrDefault("name", entry.getFileName().toString());
                    found.add(new ExtensionFunction(entry.getFileName().toString(), name));
                } catch (IOException e) {
                    LOG.warnv("Failed to read config.json in {0}: {1}", entry.getFileName(), e.getMessage());
                }
            }
        } catch (IOException e) {
            LOG.error("Failed to scan functions directory", e);
        }
        this.functions = Collections.unmodifiableList(found);
        for (ExtensionFunction f : found) {
            LOG.infov("Loaded extension function: {0}", f.name());
        }
    }

    /**
     * Returns the list of discovered extension functions.
     *
     * @return an unmodifiable list of extension functions
     */
    public List<ExtensionFunction> listFunctions() {
        return functions;
    }

    /**
     * Returns the path to the functions directory.
     *
     * @return the functions directory path
     */
    public Path getFunctionsDir() {
        return Paths.get(configService.getConfigDir(), FUNCTIONS_DIR);
    }
}
