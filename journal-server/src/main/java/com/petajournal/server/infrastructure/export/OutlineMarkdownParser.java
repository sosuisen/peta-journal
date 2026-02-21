package com.petajournal.server.infrastructure.export;

import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.BlockIdGenerator;
import com.petajournal.server.domain.FractionalIndexer;

import java.time.Instant;
import java.util.*;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Parses Outline Markdown format back into Page and Block domain objects.
 * Format: YAML frontmatter + blank-line separated blocks with HTML comment metadata.
 */
public class OutlineMarkdownParser {

    private static final Pattern ID_COMMENT = Pattern.compile("^<!--\\s*([0-9a-f]{7,8}(-[0-9a-f-]+)?)\\s*-->$");
    private static final Pattern LEVEL_COMMENT = Pattern.compile("^<!--\\s*level=(\\d+)\\s*-->$");
    private static final Pattern COLLAPSED_COMMENT = Pattern.compile("^<!--\\s*collapsed=true\\s*-->$");
    private static final Pattern PROP_COMMENT = Pattern.compile("^<!--\\s*(\\w+)=(.*?)\\s*-->$");

    public record ParseResult(
        Map<String, String> frontmatter,
        List<ParsedBlock> blocks
    ) {}

    public record ParsedBlock(
        String id,
        int level,
        String body,
        boolean collapsed,
        List<String> types,
        String status,
        String propertiesJson
    ) {}

    public ParseResult parse(String markdown) {
        if (markdown == null || markdown.isEmpty()) {
            return new ParseResult(Map.of(), List.of());
        }

        String frontmatterStr = "";
        String content = markdown;

        // Extract frontmatter
        if (markdown.startsWith("---\n")) {
            int endIdx = markdown.indexOf("\n---\n", 4);
            if (endIdx > 0) {
                frontmatterStr = markdown.substring(4, endIdx);
                content = markdown.substring(endIdx + 5).trim();
            }
        }

        Map<String, String> frontmatter = parseFrontmatter(frontmatterStr);
        List<ParsedBlock> blocks = parseBlocks(content);

        return new ParseResult(frontmatter, blocks);
    }

    public Page toPage(ParseResult result, String dirPath, String fileName, String type) {
        Map<String, String> fm = result.frontmatter();

        UUID id = fm.containsKey("id") ? UUID.fromString(fm.get("id")) : UUID.randomUUID();
        Instant now = Instant.now();
        Instant created = fm.containsKey("created") ? Instant.parse(fm.get("created")) : now;
        Instant modified = fm.containsKey("modified") ? Instant.parse(fm.get("modified")) : now;

        List<String> tags = new ArrayList<>();
        if (fm.containsKey("tags")) {
            // Tags are stored as "tag1,tag2" in simplified frontmatter
            for (String tag : fm.get("tags").split(",")) {
                String t = tag.trim();
                if (!t.isEmpty()) tags.add(t);
            }
        }

        return new Page(id, dirPath, fileName, type, created, modified, tags, "{}");
    }

    public List<Block> toBlocks(ParseResult result, UUID pageId, String dirPath, String fileName, String type) {
        List<Block> blocks = new ArrayList<>();
        Instant now = Instant.now();
        String prevIdx = null;

        for (ParsedBlock pb : result.blocks()) {
            String idx = FractionalIndexer.between(prevIdx, null);
            UUID blockId;
            if (pb.id() != null && !pb.id().isEmpty()) {
                try {
                    blockId = UUID.fromString(pb.id());
                } catch (IllegalArgumentException e) {
                    blockId = BlockIdGenerator.generate();
                }
            } else {
                blockId = BlockIdGenerator.generate();
            }

            blocks.add(new Block(
                    blockId, pageId, dirPath, fileName, type, now, now, idx,
                    pb.level(), pb.body(), "",
                    pb.collapsed(), pb.status(), pb.types(), pb.propertiesJson()
            ));
            prevIdx = idx;
        }
        return blocks;
    }

    private Map<String, String> parseFrontmatter(String frontmatter) {
        Map<String, String> map = new LinkedHashMap<>();
        if (frontmatter.isEmpty()) return map;

        StringBuilder currentListKey = null;
        StringBuilder currentListValues = null;

        for (String line : frontmatter.split("\n")) {
            if (line.startsWith("  - ") && currentListKey != null) {
                if (currentListValues.length() > 0) currentListValues.append(",");
                currentListValues.append(line.substring(4).trim());
                continue;
            }

            // Flush current list
            if (currentListKey != null) {
                map.put(currentListKey.toString(), currentListValues.toString());
                currentListKey = null;
                currentListValues = null;
            }

            int colon = line.indexOf(':');
            if (colon > 0) {
                String key = line.substring(0, colon).trim();
                String value = line.substring(colon + 1).trim();

                // Remove quotes
                if (value.startsWith("\"") && value.endsWith("\"")) {
                    value = value.substring(1, value.length() - 1).replace("\\\"", "\"");
                }

                if (value.isEmpty()) {
                    // Start of a list
                    currentListKey = new StringBuilder(key);
                    currentListValues = new StringBuilder();
                } else {
                    map.put(key, value);
                }
            }
        }

        if (currentListKey != null) {
            map.put(currentListKey.toString(), currentListValues.toString());
        }

        return map;
    }

    private List<ParsedBlock> parseBlocks(String content) {
        if (content.isEmpty()) return List.of();

        List<ParsedBlock> blocks = new ArrayList<>();
        String[] lines = content.split("\n", -1);
        boolean inCodeFence = false;

        // Current block accumulation
        String currentId = null;
        int currentLevel = 1;
        boolean currentCollapsed = false;
        List<String> currentBodyLines = new ArrayList<>();
        Map<String, String> currentProps = new LinkedHashMap<>();
        boolean hasContent = false;

        for (String line : lines) {
            // Track code fences
            if (line.trim().startsWith("```")) {
                inCodeFence = !inCodeFence;
            }

            // Blank line outside code fence → flush current block
            if (line.isEmpty() && !inCodeFence) {
                if (hasContent) {
                    blocks.add(buildParsedBlock(currentId, currentLevel, currentBodyLines, currentCollapsed, currentProps));
                    currentId = null;
                    currentLevel = 1;
                    currentCollapsed = false;
                    currentBodyLines = new ArrayList<>();
                    currentProps = new LinkedHashMap<>();
                    hasContent = false;
                }
                continue;
            }

            // Try to match metadata comments (only outside code fences)
            if (!inCodeFence) {
                Matcher idMatcher = ID_COMMENT.matcher(line);
                if (idMatcher.matches()) {
                    currentId = idMatcher.group(1);
                    hasContent = true;
                    continue;
                }

                Matcher levelMatcher = LEVEL_COMMENT.matcher(line);
                if (levelMatcher.matches()) {
                    currentLevel = Integer.parseInt(levelMatcher.group(1));
                    hasContent = true;
                    continue;
                }

                Matcher collapsedMatcher = COLLAPSED_COMMENT.matcher(line);
                if (collapsedMatcher.matches()) {
                    currentCollapsed = true;
                    hasContent = true;
                    continue;
                }

                Matcher propMatcher = PROP_COMMENT.matcher(line);
                if (propMatcher.matches()) {
                    String key = propMatcher.group(1);
                    String value = propMatcher.group(2);
                    currentProps.put(key, value);
                    hasContent = true;
                    continue;
                }
            }

            // Regular body line
            currentBodyLines.add(line);
            hasContent = true;
        }

        // Flush last block
        if (hasContent) {
            blocks.add(buildParsedBlock(currentId, currentLevel, currentBodyLines, currentCollapsed, currentProps));
        }

        return blocks;
    }

    private ParsedBlock buildParsedBlock(String id, int level, List<String> bodyLines, boolean collapsed, Map<String, String> props) {
        String body = String.join("\n", bodyLines);

        // Whitespace-only body → empty
        if (!body.isEmpty() && body.trim().isEmpty()) {
            body = "";
        }

        // Detect status from first body line
        String status = "normal";
        if (body.startsWith("- [ ] ")) {
            status = "todo";
            body = body.substring(6);
        } else if (body.startsWith("- [x] ")) {
            status = "done";
            body = body.substring(6);
        }

        // Detect block types from body
        List<String> types = new ArrayList<>();
        String trimmed = body.trim();
        if (trimmed.startsWith("#")) types.add("heading");
        if (trimmed.startsWith("```")) types.add("code");
        if (trimmed.startsWith(">")) types.add("blockquote");

        // Build properties JSON
        String propsJson = "{}";
        if (!props.isEmpty()) {
            StringBuilder pj = new StringBuilder("{");
            boolean first = true;
            for (var entry : props.entrySet()) {
                if (!first) pj.append(",");
                pj.append("\"").append(entry.getKey()).append("\":\"").append(entry.getValue()).append("\"");
                first = false;
            }
            pj.append("}");
            propsJson = pj.toString();
        }

        return new ParsedBlock(id, level, body, collapsed, types, status, propsJson);
    }
}
