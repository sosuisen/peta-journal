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
 * Format: YAML frontmatter + blank-line separated blocks with pandoc attributes.
 */
public class OutlineMarkdownParser {

    private static final Pattern ATTR_PATTERN = Pattern.compile("\\{#(\\w+)([^}]*)\\}\\s*$");
    private static final Pattern CLASS_PATTERN = Pattern.compile("\\.(\\w+)");

    public record ParseResult(
        Map<String, String> frontmatter,
        List<ParsedBlock> blocks
    ) {}

    public record ParsedBlock(
        String id,
        int level,
        String body,
        boolean collapsed,
        List<String> types
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
            String blockId = (pb.id() != null && !pb.id().isEmpty()) ? pb.id() : BlockIdGenerator.generate();

            blocks.add(new Block(
                    blockId, pageId, dirPath, fileName, type, now, now, idx,
                    pb.level(), pb.body(), "",
                    pb.collapsed(), "normal", pb.types(), "{}"
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
        // Split on blank lines
        String[] rawBlocks = content.split("\n\n");

        for (String raw : rawBlocks) {
            if (raw.isBlank()) continue;

            String body = raw;
            String id = null;
            boolean collapsed = false;
            List<String> types = new ArrayList<>();

            // Check for pandoc attributes at end
            Matcher attrMatcher = ATTR_PATTERN.matcher(raw);
            if (attrMatcher.find()) {
                id = attrMatcher.group(1);
                String attrsStr = attrMatcher.group(2);

                Matcher classMatcher = CLASS_PATTERN.matcher(attrsStr);
                while (classMatcher.find()) {
                    String cls = classMatcher.group(1);
                    if ("collapsed".equals(cls)) {
                        collapsed = true;
                    } else {
                        types.add(cls);
                    }
                }

                body = raw.substring(0, attrMatcher.start()).stripTrailing();
            }

            // Detect indentation level (2 spaces per level)
            int level = 0;
            for (int i = 0; i < body.length(); i++) {
                if (body.charAt(i) == ' ') level++;
                else break;
            }
            level = level / 2;

            // Remove indentation
            String indent = "  ".repeat(level);
            body = body.lines()
                    .map(line -> line.startsWith(indent) ? line.substring(indent.length()) : line)
                    .reduce((a, b) -> a + "\n" + b)
                    .orElse("");

            blocks.add(new ParsedBlock(id, level, body, collapsed, types));
        }

        return blocks;
    }
}
