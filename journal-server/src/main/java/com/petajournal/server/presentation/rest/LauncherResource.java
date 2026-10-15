package com.petajournal.server.presentation.rest;

import com.petajournal.server.infrastructure.launcher.AppLauncher;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import org.jboss.logging.Logger;

@Path("/api/launcher")
@Produces(MediaType.APPLICATION_JSON)
@Consumes(MediaType.APPLICATION_JSON)
public class LauncherResource {

    private static final Logger LOG = Logger.getLogger(LauncherResource.class);

    private final AppLauncher appLauncher;

    @Inject
    public LauncherResource(AppLauncher appLauncher) {
        this.appLauncher = appLauncher;
    }

    public record LaunchRequest(String url) {}
    public record LaunchResponse(boolean success, String message) {}

    @POST
    @Path("/open")
    public Response launch(LaunchRequest request) {
        if (request.url == null || request.url.isEmpty()) {
            return Response.status(Response.Status.BAD_REQUEST)
                    .entity(new LaunchResponse(false, "URL is required"))
                    .build();
        }

        // Security: Reject dangerous protocols
        String lowerUrl = request.url.toLowerCase();
        if (lowerUrl.startsWith("javascript:") ||
            lowerUrl.startsWith("data:") ||
            lowerUrl.startsWith("file:")) {
            LOG.warnv("Rejected dangerous URL: {0}", request.url);
            return Response.status(Response.Status.FORBIDDEN)
                    .entity(new LaunchResponse(false, "Dangerous protocols are not allowed"))
                    .build();
        }

        // Security: Only allow custom protocols (must contain ://)
        if (!request.url.contains("://")) {
            LOG.warnv("Rejected invalid URL: {0}", request.url);
            return Response.status(Response.Status.BAD_REQUEST)
                    .entity(new LaunchResponse(false, "URL must be a valid protocol URL (e.g., app://)"))
                    .build();
        }

        // Allow custom protocols like peta-journal://, obsidian://, etc.
        boolean success = appLauncher.launch(request.url);

        if (success) {
            LOG.infov("Successfully launched: {0}", request.url);
            return Response.ok(new LaunchResponse(true, "App launched successfully")).build();
        } else {
            return Response.serverError()
                    .entity(new LaunchResponse(false, "Failed to launch app"))
                    .build();
        }
    }
}
