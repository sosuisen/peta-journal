package com.petajournal.server.infrastructure.startup;

import com.petajournal.server.application.ConfigService;
import com.petajournal.server.application.FolderChooserService;
import io.quarkus.runtime.Startup;
import jakarta.annotation.PostConstruct;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

@Startup
@ApplicationScoped
public class ConfigurationStartup {

    private static final Logger LOG = Logger.getLogger(ConfigurationStartup.class);

    @Inject
    ConfigService configService;

    @Inject
    FolderChooserService folderChooserService;

    @PostConstruct
    void checkConfiguration() {

        String markdownDir = configService.getMarkdownDir();
        LOG.infov("Current markdown directory: [{0}]", markdownDir);

        if (markdownDir == null || markdownDir.trim().isEmpty()) {
            LOG.warn("Markdown directory not configured. Opening folder chooser...");

            FolderChooserService.FolderChoice choice = folderChooserService.chooseFolder();
            LOG.infov("Folder chooser returned: selected={0}, path=[{1}]", choice.selected, choice.path);

            if (choice.selected && !choice.path.isEmpty()) {
                configService.setMarkdownDir(choice.path);
                LOG.infov("Markdown directory configured: {0}", choice.path);
            } else {
                LOG.error("No markdown directory selected. Application may not function correctly.");
            }
        } else {
            // already configured
        }

    }
}
