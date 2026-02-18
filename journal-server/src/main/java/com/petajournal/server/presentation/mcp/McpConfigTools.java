package com.petajournal.server.presentation.mcp;

import com.petajournal.server.application.ConfigService;
import io.quarkiverse.mcp.server.Tool;
import io.quarkiverse.mcp.server.ToolArg;
import jakarta.inject.Inject;

import java.util.Map;

public class McpConfigTools {

    @Inject
    ConfigService configService;

    @Inject
    McpJsonHelper jsonHelper;

    @Tool(description = "Get server configuration (week numbering setting)")
    String get_config() {
        return jsonHelper.toJson(Map.of("weekNumbering", configService.getWeekNumbering()));
    }

    @Tool(description = "Update server configuration")
    String update_config(
            @ToolArg(description = "Week numbering system: 'us' or 'iso'") String weekNumbering
    ) {
        configService.setWeekNumbering(weekNumbering);
        return jsonHelper.toJson(Map.of("weekNumbering", configService.getWeekNumbering()));
    }
}
