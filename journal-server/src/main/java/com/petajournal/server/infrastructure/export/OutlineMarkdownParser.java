package com.petajournal.server.infrastructure.export;

import com.petajournal.server.application.FlatMarkdownSerializer;
import com.petajournal.server.application.FlatMarkdownSplitter;
import com.petajournal.server.application.FlatMarkdownSplitter.SplitItem;
import com.petajournal.server.application.FlatMarkdownSplitter.SplitPage;
import com.petajournal.server.domain.ItemBody;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * Parses a Flat Markdown page (spec v0.2) into its front matter and its items.
 * The item split is done by {@link FlatMarkdownSplitter}. This class maps the result:
 * YAML front matter to key-value pairs, and the metadata line
 * ({@code <!-- @@ ID idx="z0" collapsed="true" transclude="ID" key="value" -->}) to item fields.
 */
@ApplicationScoped
public class OutlineMarkdownParser {

    private final FlatMarkdownSplitter splitter;

    /**
     * Creates the parser.
     *
     * @param splitter splits a page into items
     * @throws NullPointerException if splitter is null
     */
    @Inject
    public OutlineMarkdownParser(FlatMarkdownSplitter splitter) {
        this.splitter = Objects.requireNonNull(splitter, "splitter must not be null");
    }

    /** Front matter and items of one parsed page. */
    public record ParseResult(
        Map<String, String> frontmatter,
        List<ParsedItem> items
    ) {}

    /** One item as written in the file. {@code id}, {@code transcludeItemId} and {@code idx} are null when absent. */
    public record ParsedItem(
        String id,
        int level,
        String body,
        boolean collapsed,
        List<String> types,
        Map<String, String> properties,
        String transcludeItemId,
        String idx
    ) {}

    /**
     * Parses a whole page.
     *
     * @param markdown the file content; null or empty gives an empty result
     * @return the front matter and the items in file order
     */
    public ParseResult parse(String markdown) {
        SplitPage page = splitter.split(markdown);

        Map<String, String> frontmatter = page.frontMatter() == null
                ? Map.of()
                : parseFrontmatter(stripFrontmatterDelimiters(page.frontMatter()));

        List<ParsedItem> items = new ArrayList<>();
        for (SplitItem item : page.items()) {
            items.add(toParsedItem(item));
        }
        return new ParseResult(frontmatter, items);
    }

    /** Removes the {@code ---} lines around the front matter body. */
    private static String stripFrontmatterDelimiters(String raw) {
        String s = raw.strip();
        if (s.startsWith("---")) {
            s = s.substring(3);
        }
        if (s.endsWith("---")) {
            s = s.substring(0, s.length() - 3);
        }
        return s.strip();
    }

    private Map<String, String> parseFrontmatter(String frontmatter) {
        Map<String, String> map = new LinkedHashMap<>();
        if (frontmatter.isEmpty()) {
            return map;
        }

        StringBuilder currentListKey = null;
        StringBuilder currentListValues = null;

        for (String line : frontmatter.split("\n")) {
            if (line.startsWith("  - ") && currentListKey != null) {
                if (currentListValues.length() > 0) {
                    currentListValues.append(",");
                }
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

    private static ParsedItem toParsedItem(SplitItem item) {
        // The reserved keys are item fields, not user properties
        Map<String, String> properties = new LinkedHashMap<>(item.properties());
        String idx = properties.remove(FlatMarkdownSerializer.IDX_KEY);
        boolean collapsed = "true".equals(properties.remove(FlatMarkdownSerializer.COLLAPSED_KEY));
        String transclude = properties.remove(FlatMarkdownSerializer.TRANSCLUDE_KEY);

        ItemBody body = ItemBody.parse(item.body());
        List<String> types = new ArrayList<>();
        if (body.task()) {
            types.add(ItemBody.TASK_TYPE);
        }
        types.addAll(body.textTypes());

        return new ParsedItem(item.id(), item.level(), body.text(), collapsed, types, properties, transclude, idx);
    }
}
