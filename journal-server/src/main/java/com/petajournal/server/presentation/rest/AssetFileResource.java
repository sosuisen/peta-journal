package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.ConfigService;
import jakarta.inject.Inject;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.PathParam;
import jakarta.ws.rs.core.Response;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;

@Path("/assets")
public class AssetFileResource {

    @Inject
    ConfigService configService;

    @GET
    @Path("/{fileName}")
    public Response getAsset(@PathParam("fileName") String fileName) {
        String markdownDir = configService.getMarkdownDir();
        if (markdownDir == null || markdownDir.isBlank()) {
            return Response.status(Response.Status.NOT_FOUND).build();
        }

        // Prevent path traversal
        if (fileName.contains("..") || fileName.contains("/") || fileName.contains("\\")) {
            return Response.status(Response.Status.BAD_REQUEST).build();
        }

        java.nio.file.Path file = Paths.get(markdownDir, "assets", fileName);
        if (!Files.exists(file) || !Files.isRegularFile(file)) {
            return Response.status(Response.Status.NOT_FOUND).build();
        }

        try {
            String contentType = Files.probeContentType(file);
            if (contentType == null) {
                contentType = "application/octet-stream";
            }
            byte[] data = Files.readAllBytes(file);
            return Response.ok(data, contentType).build();
        } catch (IOException e) {
            return Response.status(Response.Status.INTERNAL_SERVER_ERROR).build();
        }
    }
}
