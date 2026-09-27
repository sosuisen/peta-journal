package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.ConfigService;
import com.petajournal.server.application.FolderChooserService;
import com.petajournal.server.infrastructure.export.MarkdownExportService;
import com.petajournal.server.infrastructure.persistence.PropertyRepository;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;

import java.util.Map;

@Path("/api/config")
@Produces(MediaType.APPLICATION_JSON)
@Consumes(MediaType.APPLICATION_JSON)
public class ConfigResource {

    @Inject
    ConfigService configService;

    @Inject
    PropertyRepository propertyRepository;

    @Inject
    FolderChooserService folderChooserService;

    @Inject
    MarkdownExportService markdownExportService;

    @Inject
    com.petajournal.server.infrastructure.watch.MarkdownFolderWatcher folderWatcher;

    record ConfigResponse(String markdownDir, String weekNumbering, boolean logicalOutdent) {}
    record UpdateConfigRequest(String markdownDir, String weekNumbering, Boolean logicalOutdent) {}
    record PropertyDefinitionsResponse(Map<String, String> propertyDefinitions) {}
    record BrowseFolderResponse(boolean selected, String path) {}

    @GET
    public ConfigResponse getConfig() {
        return new ConfigResponse(
                configService.getMarkdownDir(),
                configService.getWeekNumbering(),
                configService.getLogicalOutdent());
    }

    @GET
    @Path("/property-definitions")
    public PropertyDefinitionsResponse getPropertyDefinitions() {
        return new PropertyDefinitionsResponse(propertyRepository.findAllDefinitions());
    }

    @POST
    @Path("/browse-folder")
    public BrowseFolderResponse browseFolder() {
        var choice = folderChooserService.chooseFolder();
        return new BrowseFolderResponse(choice.selected, choice.path);
    }

    @PUT
    public ConfigResponse updateConfig(UpdateConfigRequest req) {
        if (req.markdownDir() != null) {
            configService.setMarkdownDir(req.markdownDir());
            markdownExportService.exportAllIfEmpty(req.markdownDir());
            folderWatcher.restart();
        }
        if (req.weekNumbering() != null) configService.setWeekNumbering(req.weekNumbering());
        if (req.logicalOutdent() != null) configService.setLogicalOutdent(req.logicalOutdent());
        return new ConfigResponse(
                configService.getMarkdownDir(),
                configService.getWeekNumbering(),
                configService.getLogicalOutdent());
    }
}
