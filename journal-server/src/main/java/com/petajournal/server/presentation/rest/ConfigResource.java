package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.ConfigService;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;

@Path("/api/config")
@Produces(MediaType.APPLICATION_JSON)
@Consumes(MediaType.APPLICATION_JSON)
public class ConfigResource {

    @Inject
    ConfigService configService;

    record ConfigResponse(String weekNumbering) {}
    record UpdateConfigRequest(String weekNumbering) {}

    @GET
    public ConfigResponse getConfig() {
        return new ConfigResponse(configService.getWeekNumbering());
    }

    @PUT
    public ConfigResponse updateConfig(UpdateConfigRequest req) {
        configService.setWeekNumbering(req.weekNumbering());
        return new ConfigResponse(configService.getWeekNumbering());
    }
}
