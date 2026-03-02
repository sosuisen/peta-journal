package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.ConfigService;
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

    record ConfigResponse(String weekNumbering) {}
    record UpdateConfigRequest(String weekNumbering) {}
    record PropertyDefinitionsResponse(Map<String, String> propertyDefinitions) {}

    @GET
    public ConfigResponse getConfig() {
        return new ConfigResponse(configService.getWeekNumbering());
    }

    @GET
    @Path("/property-definitions")
    public PropertyDefinitionsResponse getPropertyDefinitions() {
        return new PropertyDefinitionsResponse(propertyRepository.findAllDefinitions());
    }

    @PUT
    public ConfigResponse updateConfig(UpdateConfigRequest req) {
        configService.setWeekNumbering(req.weekNumbering());
        return new ConfigResponse(configService.getWeekNumbering());
    }
}
