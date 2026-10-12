package com.petajournal.server.infrastructure.export;

import com.petajournal.server.application.FlatMarkdownSerializer;
import com.petajournal.server.application.FlatMarkdownSplitter;
import com.petajournal.server.application.FlatMarkdownSplitter.SplitItem;
import com.petajournal.server.application.FlatMarkdownSplitter.SplitPage;
import com.petajournal.server.domain.IdGenerator;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.FractionalIndexer;
import com.sosuisha.hmtid.HMTID;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import java.time.Instant;
import java.util.*;

/**
 * Parses a Flat Markdown page (spec v0.2) back into Page and Item domain objects.
 * The item split is done by {@link FlatMarkdownSplitter}. This class maps the result:
 * YAML front matter to page fields, and the metadata line
 * ({@code <!-- @@ ID idx="z0" collapsed="true" transclude="ID" key="value" -->}) to item fields.
 */
@ApplicationScoped
public class OutlineMarkdownParser {

    private final IdGenerator idGenerator;
    private final FlatMarkdownSplitter splitter;

    /**
     * Creates the parser.
     *
     * @param idGenerator generates IDs for items and pages that have none in the file
     * @param splitter splits a page into items
     * @throws NullPointerException if any argument is null
     */
    @Inject
    public OutlineMarkdownParser(IdGenerator idGenerator, FlatMarkdownSplitter splitter) {
        this.idGenerator = Objects.requireNonNull(idGenerator, "idGenerator must not be null");
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

    /**
     * Builds the page from the front matter. Missing fields fall back to a new ID and the current time.
     *
     * @param result the parsed page
     * @param dirPath the directory path of the page
     * @param name the page name
     * @param type the page type
     * @return the page
     */
    public Page toPage(ParseResult result, String dirPath, String name, String type) {
        Map<String, String> fm = result.frontmatter();

        HMTID id = fm.containsKey("id") ? HMTID.of(fm.get("id")) : idGenerator.generatePageId();
        Instant now = Instant.now();
        Instant created = fm.containsKey("created") ? Instant.parse(fm.get("created")) : now;
        Instant modified = fm.containsKey("modified") ? Instant.parse(fm.get("modified")) : now;

        return new Page(id, null, dirPath, name, type, created, modified);
    }

    /**
     * Builds the items of the page. Items without an ID or with a malformed one get a new ID.
     *
     * @param result the parsed page
     * @param pageId the page ID
     * @param dirPath the directory path of the page
     * @param name the page name
     * @param type the page type
     * @return the items in file order
     */
    public List<Item> toItems(ParseResult result, HMTID pageId, String dirPath, String name, String type) {
        List<Item> items = new ArrayList<>();
        Instant now = Instant.now();
        String prevIdx = null;

        for (ParsedItem pb : result.items()) {
            String idx = pb.idx() != null && (prevIdx == null || prevIdx.compareTo(pb.idx()) < 0)
                    ? pb.idx() : FractionalIndexer.between(prevIdx, null);
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

            HMTID transclude = null;
            if (pb.transcludeItemId() != null) {
                try {
                    transclude = HMTID.of(pb.transcludeItemId());
                } catch (IllegalArgumentException ignored) {
                    // malformed id in file: treat as a plain item
                }
            }

            items.add(new Item(
                    itemId, pageId, null, dirPath, name, type, now, now, idx,
                    pb.level(), pb.body(), "",
                    pb.collapsed(), pb.types(), transclude
            ));
            prevIdx = idx;
        }
        return items;
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
        String body = item.body();

        // The reserved keys are item fields, not user properties
        Map<String, String> properties = new LinkedHashMap<>(item.properties());
        String idx = properties.remove(FlatMarkdownSerializer.IDX_KEY);
        boolean collapsed = "true".equals(properties.remove(FlatMarkdownSerializer.COLLAPSED_KEY));
        String transclude = properties.remove(FlatMarkdownSerializer.TRANSCLUDE_KEY);

        // Detect task type from GFM checkbox prefix
        boolean isTask = false;
        if (body.startsWith("- [ ] ") || body.startsWith("- [x] ")) {
            isTask = true;
            body = body.substring(6);
        }

        List<String> types = new ArrayList<>();
        if (isTask) {
            types.add("task");
        }
        String trimmed = body.trim();
        if (trimmed.startsWith("#")) {
            types.add("heading");
        }
        if (trimmed.startsWith("```")) {
            types.add("code");
        }
        if (trimmed.startsWith(">")) {
            types.add("blockquote");
        }

        return new ParsedItem(item.id(), item.level(), body, collapsed, types, properties, transclude, idx);
    }
}
