package com.petajournal.server.application;

import jakarta.inject.Inject;
import com.fasterxml.jackson.databind.ObjectMapper;
import jakarta.enterprise.context.ApplicationScoped;
import org.eclipse.microprofile.config.inject.ConfigProperty;
import org.jboss.logging.Logger;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.HashMap;
import java.util.Map;

@ApplicationScoped
public class ConfigService {

    private static final Logger LOG = Logger.getLogger(ConfigService.class);
    private static final String CONFIG_FILE_NAME = "config.json";

    private final String configDir;

    @Inject
    public ConfigService(@ConfigProperty(name = "petajournal.config.dir") String configDir) {
        this.configDir = configDir;
    }

    private final ObjectMapper objectMapper = new ObjectMapper();

    /**
     * Returns the configured markdown export directory path.
     *
     * @return the directory path, or an empty string if not configured
     */
    public String getMarkdownDir() {
        Map<String, Object> config = loadConfig();
        return (String) config.getOrDefault("markdownDir", "");
    }

    /**
     * Sets the markdown export directory path.
     *
     * @param markdownDir the directory path
     */
    public void setMarkdownDir(String markdownDir) {
        Map<String, Object> config = loadConfig();
        config.put("markdownDir", markdownDir);
        saveConfig(config);
    }

    /**
     * Returns the week numbering system.
     *
     * @return {@code "us"} or {@code "iso"} (defaults to {@code "us"})
     */
    public String getWeekNumbering() {
        Map<String, Object> config = loadConfig();
        return (String) config.getOrDefault("weekNumbering", "us");
    }

    /**
     * Sets the week numbering system.
     *
     * @param weekNumbering {@code "us"} or {@code "iso"}
     */
    public void setWeekNumbering(String weekNumbering) {
        Map<String, Object> config = loadConfig();
        config.put("weekNumbering", weekNumbering);
        saveConfig(config);
    }

    public boolean getLogicalOutdent() {
        Map<String, Object> config = loadConfig();
        return Boolean.TRUE.equals(config.getOrDefault("logicalOutdent", false));
    }

    public void setLogicalOutdent(boolean logicalOutdent) {
        Map<String, Object> config = loadConfig();
        config.put("logicalOutdent", logicalOutdent);
        saveConfig(config);
    }

    private Map<String, Object> loadConfig() {
        try {
            Path configPath = getConfigPath();
            if (Files.exists(configPath)) {
                byte[] bytes = Files.readAllBytes(configPath);
                @SuppressWarnings("unchecked")
                Map<String, Object> config = objectMapper.readValue(bytes, Map.class);
                return config;
            }
        } catch (IOException e) {
            LOG.warn("Failed to load config file", e);
        }
        return new HashMap<>();
    }

    private void saveConfig(Map<String, Object> config) {
        try {
            Path configPath = getConfigPath();
            Files.createDirectories(configPath.getParent());
            byte[] bytes = objectMapper.writerWithDefaultPrettyPrinter().writeValueAsBytes(config);
            Files.write(configPath, bytes);
            LOG.infov("Config saved to {0}", configPath);
        } catch (IOException e) {
            LOG.error("Failed to save config file", e);
        }
    }

    /**
     * Returns the configuration directory path.
     *
     * @return the directory path
     */
    public String getConfigDir() {
        return configDir;
    }

    private Path getConfigPath() {
        return Paths.get(configDir, CONFIG_FILE_NAME);
    }
}
