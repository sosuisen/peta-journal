package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.ExtensionFunctionService;
import com.petajournal.server.application.ExtensionFunctionService.ExtensionFunction;
import jakarta.inject.Inject;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.PathParam;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import org.jboss.logging.Logger;

import java.io.IOException;
import java.nio.file.Files;
import java.util.List;

@Path("/api/functions")
public class FunctionsResource {

    private static final Logger LOG = Logger.getLogger(FunctionsResource.class);

    @Inject
    ExtensionFunctionService extensionFunctionService;

    public record FunctionInfo(String name, String url) {}

    @GET
    @Produces(MediaType.APPLICATION_JSON)
    public List<FunctionInfo> listFunctions() {
        return extensionFunctionService.listFunctions().stream()
            .map(f -> new FunctionInfo(f.name(), "/api/functions/" + f.dirName() + "/index.html"))
            .toList();
    }

    @GET
    @Path("/{dirName}/{path:.*}")
    public Response serveFile(@PathParam("dirName") String dirName, @PathParam("path") String path) {
        // Security: reject path traversal
        if (dirName.contains("..") || path.contains("..")) {
            return Response.status(Response.Status.FORBIDDEN).build();
        }

        java.nio.file.Path filePath = extensionFunctionService.getFunctionsDir().resolve(dirName).resolve(path);

        if (!Files.exists(filePath) || !Files.isRegularFile(filePath)) {
            return Response.status(Response.Status.NOT_FOUND).build();
        }

        try {
            byte[] content = Files.readAllBytes(filePath);
            String contentType = guessContentType(path);
            return Response.ok(content).type(contentType).build();
        } catch (IOException e) {
            LOG.errorv("Failed to serve file: {0}/{1}", dirName, path);
            return Response.serverError().build();
        }
    }

    private String guessContentType(String path) {
        String lower = path.toLowerCase();
        if (lower.endsWith(".html")) return "text/html";
        if (lower.endsWith(".js")) return "application/javascript";
        if (lower.endsWith(".css")) return "text/css";
        if (lower.endsWith(".json")) return "application/json";
        if (lower.endsWith(".md")) return "text/markdown";
        if (lower.endsWith(".png")) return "image/png";
        if (lower.endsWith(".jpg") || lower.endsWith(".jpeg")) return "image/jpeg";
        if (lower.endsWith(".svg")) return "image/svg+xml";
        return "application/octet-stream";
    }
}
