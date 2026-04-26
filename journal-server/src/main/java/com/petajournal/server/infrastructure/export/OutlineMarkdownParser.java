package com.petajournal.server.infrastructure.export;

import com.petajournal.server.domain.IdGenerator;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.FractionalIndexer;
import com.sosuisha.hmtid.HMTID;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import java.time.Instant;
import java.util.*;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Parses Outline Markdown format back into Page and Item domain objects.
 * Format: YAML frontmatter + blank-line separated items with HTML comment metadata.
 */
@ApplicationScoped
public class OutlineMarkdownParser {

    private static final Pattern ID_COMMENT = Pattern.compile("^<!--\\s*([0-9a-zA-Z-]{7,})\\s*-->$");
    private static final Pattern LEVEL_COMMENT = Pattern.compile("^<!--\\s*level=(\\d+)\\s*-->$");
    private static final Pattern COLLAPSED_COMMENT = Pattern.compile("^<!--\\s*collapsed=true\\s*-->$");
    private static final Pattern PROP_COMMENT = Pattern.compile("^<!--\\s*(\\w+)=(.*?)\\s*-->$");

    @Inject
    IdGenerator idGenerator;

    public record ParseResult(
        Map<String, String> frontmatter,
        List<ParsedItem> items
    ) {}

    public record ParsedItem(
        String id,
        int level,
        String body,
        boolean collapsed,
        List<String> types,
        Map<String, String> properties
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
        List<ParsedItem> items = parseItems(content);

        return new ParseResult(frontmatter, items);
    }

    public Page toPage(ParseResult result, String dirPath, String name, String type) {
        Map<String, String> fm = result.frontmatter();

        HMTID id = fm.containsKey("id") ? HMTID.of(fm.get("id")) : idGenerator.generatePageId();
        Instant now = Instant.now();
        Instant created = fm.containsKey("created") ? Instant.parse(fm.get("created")) : now;
        Instant modified = fm.containsKey("modified") ? Instant.parse(fm.get("modified")) : now;

        return new Page(id, null, dirPath, name, type, created, modified);
    }

    public List<Item> toItems(ParseResult result, HMTID pageId, String dirPath, String name, String type) {
        List<Item> items = new ArrayList<>();
        Instant now = Instant.now();
        String prevIdx = null;

        for (ParsedItem pb : result.items()) {
            String idx = FractionalIndexer.between(prevIdx, null);
            HMTID itemId;
            if (pb.id() != null && !pb.id().isEmpty()) {
                try {
                    itemId = HMTID.of(pb.id());
                } catch (IllegalArgumentException e) {
                    itemId = idGenerator.generateItemId();
                }
            } else {
                itemId = idGenerator.generateItemId();
            }

            items.add(new Item(
                    itemId, pageId, null, dirPath, name, type, now, now, idx,
                    pb.level(), pb.body(), "",
                    pb.collapsed(), pb.types(), null
            ));
            prevIdx = idx;
        }
        return items;
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

    private List<ParsedItem> parseItems(String content) {
        if (content.isEmpty()) return List.of();

        List<ParsedItem> items = new ArrayList<>();
        String[] lines = content.split("\n", -1);
        boolean inCodeFence = false;

        // Current item accumulation
        String currentId = null;
        int currentLevel = 1;
        boolean currentCollapsed = false;
        Map<String, String> currentProperties = new LinkedHashMap<>();
        List<String> currentBodyLines = new ArrayList<>();
        boolean hasContent = false;

        for (String line : lines) {
            // Track code fences
            if (line.trim().startsWith("```")) {
                inCodeFence = !inCodeFence;
            }

            // Blank line outside code fence → flush current item
            if (line.isEmpty() && !inCodeFence) {
                if (hasContent) {
                    items.add(buildParsedItem(currentId, currentLevel, currentBodyLines, currentCollapsed, currentProperties));
                    currentId = null;
                    currentLevel = 1;
                    currentCollapsed = false;
                    currentProperties = new LinkedHashMap<>();
                    currentBodyLines = new ArrayList<>();
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

                // Accumulate property comments (excluding level/collapsed handled above)
                Matcher propMatcher = PROP_COMMENT.matcher(line);
                if (propMatcher.matches()) {
                    currentProperties.put(propMatcher.group(1), propMatcher.group(2));
                    hasContent = true;
                    continue;
                }
            }

            // Regular body line
            currentBodyLines.add(line);
            hasContent = true;
        }

        // Flush last item
        if (hasContent) {
            items.add(buildParsedItem(currentId, currentLevel, currentBodyLines, currentCollapsed, currentProperties));
        }

        return items;
    }

    private ParsedItem buildParsedItem(String id, int level, List<String> bodyLines, boolean collapsed, Map<String, String> properties) {
        String body = String.join("\n", bodyLines);

        // Whitespace-only body → empty
        if (!body.isEmpty() && body.trim().isEmpty()) {
            body = "";
        }

        // Detect task type from GFM checkbox prefix
        boolean isTask = false;
        if (body.startsWith("- [ ] ")) {
            isTask = true;
            body = body.substring(6);
        } else if (body.startsWith("- [x] ")) {
            isTask = true;
            body = body.substring(6);
        }

        // Detect item types from body
        List<String> types = new ArrayList<>();
        if (isTask) types.add("task");
        String trimmed = body.trim();
        if (trimmed.startsWith("#")) types.add("heading");
        if (trimmed.startsWith("```")) types.add("code");
        if (trimmed.startsWith(">")) types.add("blockquote");

        return new ParsedItem(id, level, body, collapsed, types, properties);
    }
}
