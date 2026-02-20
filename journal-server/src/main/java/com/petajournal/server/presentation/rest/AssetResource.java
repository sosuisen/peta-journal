package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.ConfigService;
import jakarta.inject.Inject;
import jakarta.ws.rs.Consumes;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import org.jboss.logging.Logger;
import org.jboss.resteasy.reactive.multipart.FileUpload;
import org.jboss.resteasy.reactive.RestForm;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;

@Path("/api/assets")
public class AssetResource {

    private static final Logger LOG = Logger.getLogger(AssetResource.class);

    @Inject
    ConfigService configService;

    @POST
    @Path("/upload")
    @Consumes(MediaType.MULTIPART_FORM_DATA)
    @Produces(MediaType.APPLICATION_JSON)
    public Response upload(@RestForm("file") FileUpload file) {
        String markdownDir = configService.getMarkdownDir();
        if (markdownDir == null || markdownDir.isBlank()) {
            return Response.status(Response.Status.INTERNAL_SERVER_ERROR)
                    .entity("{\"error\":\"Markdown directory not configured\"}")
                    .build();
        }

        java.nio.file.Path assetsDir = Paths.get(markdownDir, "assets");
        try {
            Files.createDirectories(assetsDir);
        } catch (IOException e) {
            LOG.error("Failed to create assets directory", e);
            return Response.status(Response.Status.INTERNAL_SERVER_ERROR)
                    .entity("{\"error\":\"Failed to create assets directory\"}")
                    .build();
        }

        String originalName = file.fileName();
        if (originalName == null || originalName.isBlank()) {
            originalName = "image.png";
        }
        // Sanitize filename
        originalName = originalName.replaceAll("[^a-zA-Z0-9._-]", "_");

        String savedName = resolveUniqueName(assetsDir, originalName);
        java.nio.file.Path target = assetsDir.resolve(savedName);

        try {
            Files.copy(file.filePath(), target, StandardCopyOption.REPLACE_EXISTING);
            LOG.infov("Saved asset: {0}", target);
            return Response.ok("{\"fileName\":\"" + savedName + "\"}").build();
        } catch (IOException e) {
            LOG.error("Failed to save asset", e);
            return Response.status(Response.Status.INTERNAL_SERVER_ERROR)
                    .entity("{\"error\":\"Failed to save file\"}")
                    .build();
        }
    }

    /**
     * If a file with the given name already exists, appends -1, -2, etc. before the extension.
     */
    private String resolveUniqueName(java.nio.file.Path dir, String name) {
        if (!Files.exists(dir.resolve(name))) {
            return name;
        }

        String baseName;
        String extension;
        int dotIdx = name.lastIndexOf('.');
        if (dotIdx > 0) {
            baseName = name.substring(0, dotIdx);
            extension = name.substring(dotIdx);
        } else {
            baseName = name;
            extension = "";
        }

        int counter = 1;
        while (true) {
            String candidate = baseName + "-" + counter + extension;
            if (!Files.exists(dir.resolve(candidate))) {
                return candidate;
            }
            counter++;
        }
    }
}
