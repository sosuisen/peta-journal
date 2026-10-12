package com.petajournal.server.application;

import com.petajournal.server.domain.model.Item;
import jakarta.enterprise.context.ApplicationScoped;

import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.TreeMap;
import java.util.function.UnaryOperator;

/**
 * Serializes items to Flat Markdown (spec v0.2).
 * Every item is written as one metadata line followed by its body:
 * {@code <!-- @@ ID idx="z0" collapsed="true" transclude="ID" key="value" -->}.
 * The number of {@code @} characters is the item level.
 * Items are separated by a blank line so that each body is its own block.
 */
@ApplicationScoped
public class FlatMarkdownSerializer {

    /** Metadata key that holds the fractional index of the item. */
    public static final String IDX_KEY = "idx";
    /** Metadata key that marks a collapsed item. */
    public static final String COLLAPSED_KEY = "collapsed";
    /** Metadata key that holds the source item ID of a transclusion. */
    public static final String TRANSCLUDE_KEY = "transclude";

    /**
     * Serializes items in order, including their properties.
     *
     * @param items the items in outline order
     * @param scalarProps scalar properties keyed by item ID
     * @param listProps list properties keyed by item ID
     * @return the Flat Markdown text
     * @throws NullPointerException if any argument is null
     */
    public String serialize(List<Item> items,
                            Map<String, Map<String, String>> scalarProps,
                            Map<String, Map<String, List<String>>> listProps) {
        return serialize(items, scalarProps, listProps, UnaryOperator.identity());
    }

    /**
     * Serializes items in order, passing each body through {@code bodyMapper} before it is written.
     * The file export uses this to rewrite page links for the location of the file.
     *
     * @param items the items in outline order
     * @param scalarProps scalar properties keyed by item ID
     * @param listProps list properties keyed by item ID
     * @param bodyMapper the transformation applied to every body
     * @return the Flat Markdown text
     * @throws NullPointerException if any argument is null
     */
    public String serialize(List<Item> items,
                            Map<String, Map<String, String>> scalarProps,
                            Map<String, Map<String, List<String>>> listProps,
                            UnaryOperator<String> bodyMapper) {
        Objects.requireNonNull(items, "items must not be null");
        Objects.requireNonNull(scalarProps, "scalarProps must not be null");
        Objects.requireNonNull(listProps, "listProps must not be null");
        Objects.requireNonNull(bodyMapper, "bodyMapper must not be null");
        var sb = new StringBuilder();
        for (int i = 0; i < items.size(); i++) {
            Item item = items.get(i);
            String id = item.id().toString();
            if (i > 0) {
                sb.append("\n");
            }
            appendItem(sb, item, scalarProps.getOrDefault(id, Map.of()), listProps.getOrDefault(id, Map.of()),
                    bodyMapper);
        }
        return sb.toString();
    }

    /**
     * Appends one item as a metadata line and its body, each ending with a newline.
     *
     * @param sb the output
     * @param item the item
     * @param scalars scalar properties of the item
     * @param lists list properties of the item
     * @throws NullPointerException if any argument is null
     */
    public void appendItem(StringBuilder sb, Item item, Map<String, String> scalars, Map<String, List<String>> lists) {
        appendItem(sb, item, scalars, lists, UnaryOperator.identity());
    }

    /**
     * Appends one item as a metadata line and its body, passing the body through {@code bodyMapper}.
     *
     * @param sb the output
     * @param item the item
     * @param scalars scalar properties of the item
     * @param lists list properties of the item
     * @param bodyMapper the transformation applied to the body
     * @throws NullPointerException if any argument is null
     */
    public void appendItem(StringBuilder sb, Item item, Map<String, String> scalars, Map<String, List<String>> lists,
                           UnaryOperator<String> bodyMapper) {
        Objects.requireNonNull(sb, "sb must not be null");
        Objects.requireNonNull(item, "item must not be null");
        Objects.requireNonNull(scalars, "scalars must not be null");
        Objects.requireNonNull(lists, "lists must not be null");
        Objects.requireNonNull(bodyMapper, "bodyMapper must not be null");
        sb.append(metadataLine(item, scalars, lists)).append("\n");
        String body = bodyMapper.apply(bodyWithTaskPrefix(item, scalars));
        // An empty body is expressed by the metadata line alone
        if (!body.isEmpty()) {
            sb.append(body).append("\n");
        }
    }

    /**
     * Builds the metadata line of an item.
     * Properties are written in a fixed order: idx, collapsed, transclude, then the
     * remaining keys sorted alphabetically, so that the same item always serializes the same way.
     *
     * @param item the item
     * @param scalars scalar properties of the item
     * @param lists list properties of the item
     * @return the metadata line without a trailing newline
     * @throws NullPointerException if any argument is null
     */
    public String metadataLine(Item item, Map<String, String> scalars, Map<String, List<String>> lists) {
        Objects.requireNonNull(item, "item must not be null");
        Objects.requireNonNull(scalars, "scalars must not be null");
        Objects.requireNonNull(lists, "lists must not be null");
        var sb = new StringBuilder("<!-- ");
        sb.append("@".repeat(Math.max(1, item.level())));
        sb.append(" ").append(item.id());
        appendProperty(sb, IDX_KEY, item.idx());
        if (item.collapsed()) {
            appendProperty(sb, COLLAPSED_KEY, "true");
        }
        if (item.transcludeItemId() != null) {
            appendProperty(sb, TRANSCLUDE_KEY, item.transcludeItemId().toString());
        }
        var others = new TreeMap<String, String>(scalars);
        for (var entry : lists.entrySet()) {
            others.put(entry.getKey(), String.join(",", entry.getValue()));
        }
        for (var entry : others.entrySet()) {
            appendProperty(sb, entry.getKey(), entry.getValue());
        }
        return sb.append(" -->").toString();
    }

    /**
     * Returns the body with the GFM task prefix when the item is a task.
     * The prefix is {@code - [x] } when the task has an {@code end} property, otherwise {@code - [ ] }.
     *
     * @param item the item
     * @param scalars scalar properties of the item
     * @return the body to write
     * @throws NullPointerException if any argument is null
     */
    public static String bodyWithTaskPrefix(Item item, Map<String, String> scalars) {
        Objects.requireNonNull(item, "item must not be null");
        Objects.requireNonNull(scalars, "scalars must not be null");
        String body = item.body() == null ? "" : item.body();
        if (item.types() != null && item.types().contains("task")) {
            return (scalars.containsKey("end") ? "- [x] " : "- [ ] ") + body;
        }
        return body;
    }

    private static void appendProperty(StringBuilder sb, String key, String value) {
        if (value == null) {
            return;
        }
        sb.append(" ").append(key).append("=\"").append(escape(value)).append("\"");
    }

    /** Escapes a property value: {@code \} becomes {@code \} and {@code "} becomes {@code \"}. */
    static String escape(String value) {
        return value.replace("\\", "\\\\").replace("\"", "\\\"");
    }
}
