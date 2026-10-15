package com.petajournal.server.presentation.mcp;

import com.petajournal.server.application.ConfigService;
import io.quarkiverse.mcp.server.Tool;
import io.quarkiverse.mcp.server.ToolArg;
import jakarta.inject.Inject;

import java.util.Map;

public class McpConfigTools {

    private final ConfigService configService;
    private final McpJsonHelper jsonHelper;

    @Inject
    public McpConfigTools(ConfigService configService, McpJsonHelper jsonHelper) {
        this.configService = configService;
        this.jsonHelper = jsonHelper;
    }

    @Tool(description = "Get server configuration (markdownDir, weekNumbering, logicalOutdent)")
    String get_config() {
        return jsonHelper.toJson(Map.of(
                "markdownDir", configService.getMarkdownDir(),
                "weekNumbering", configService.getWeekNumbering(),
                "logicalOutdent", configService.getLogicalOutdent()));
    }

    @Tool(description = "Update server configuration")
    String update_config(
            @ToolArg(description = "Week numbering system: 'us' or 'iso'", required = false) String weekNumbering,
            @ToolArg(description = "Markdown export directory path", required = false) String markdownDir,
            @ToolArg(description = "Logical outdent: move children with parent", required = false) Boolean logicalOutdent
    ) {
        if (weekNumbering != null) configService.setWeekNumbering(weekNumbering);
        if (markdownDir != null) configService.setMarkdownDir(markdownDir);
        if (logicalOutdent != null) configService.setLogicalOutdent(logicalOutdent);
        return jsonHelper.toJson(Map.of(
                "markdownDir", configService.getMarkdownDir(),
                "weekNumbering", configService.getWeekNumbering(),
                "logicalOutdent", configService.getLogicalOutdent()));
    }
}
