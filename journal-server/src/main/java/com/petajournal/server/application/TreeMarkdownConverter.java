package com.petajournal.server.application;

import com.petajournal.server.domain.model.Item;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;

@ApplicationScoped
public class TreeMarkdownConverter {
    @Inject
    FlatMarkdownSerializer serializer;

    record Marker(int pos, int level) {
    }

    /**
     * Convert a list of items to Tree Markdown (nested list format).
     * Normalizes levels relative to the minimum level in the list.
     */
    public String itemsToTreeMarkdown(List<Item> items) {
        if (items.isEmpty())
            return "";

        int minLevel = items.stream().mapToInt(Item::level).min().orElse(1);
        StringBuilder sb = new StringBuilder();

        for (int i = 0; i < items.size(); i++) {
            Item item = items.get(i);
            int depth = item.level() - minLevel;
            String indent = "  ".repeat(depth);
            String[] lines = item.body().split("\n", -1);

            sb.append(indent).append("- ").append(lines[0]);

            // Continuation lines
            for (int j = 1; j < lines.length; j++) {
                sb.append("\n").append(indent).append("  ").append(lines[j]);
            }

            if (i < items.size() - 1) {
                sb.append("\n");
            }
        }

        return sb.toString();
    }

    /**
     * Converts a list of items to Flat Markdown, including properties in the metadata line.
     *
     * @param items the items in outline order
     * @param scalarProps scalar properties keyed by item ID
     * @param listProps list properties keyed by item ID
     * @return the Flat Markdown text
     */
    public String itemsToFlatMarkdown(List<Item> items,
                                       Map<String, Map<String, String>> scalarProps,
                                       Map<String, Map<String, List<String>>> listProps) {
        return serializer.serialize(items, scalarProps, listProps);
    }

    /**
     * Parses Tree Markdown into Flat Markdown.
     * Each list item becomes a separate item. Items deeper than the top level get a
     * metadata line whose number of {@code @} characters is the level; top-level items
     * omit the metadata line, which the spec treats as level 1.
     */
    public String treeMarkdownToFlatMarkdown(String treeMd) {
        if (treeMd == null || treeMd.isEmpty())
            return "";

        List<String> items = new ArrayList<>();
        String[] lines = treeMd.replace("\r", "").split("\n", -1);
        StringBuilder currentItem = null;
        int itemPrefixLen = 0;
        int currentLevel = 0;

        for (String line : lines) {
            Optional<Marker> marker = findListMarkerPos(line);

            if (marker.isPresent()) {
                // New item — flush previous
                if (currentItem != null) {
                    String item = currentItem.toString();
                    if (currentLevel > 1) {
                        item = levelLine(currentLevel) + "\n" + item;
                    }
                    items.add(item);
                }
                currentLevel = marker.get().level();
                itemPrefixLen = marker.get().pos() + 2; // indent + "- "
                String firstLine = line.substring(itemPrefixLen);
                // Re-add "- " for GFM task prefix so flat markdown parser can detect task type
                if (firstLine.startsWith("[ ] ") || firstLine.startsWith("[x] ")) {
                    firstLine = "- " + firstLine;
                }
                currentItem = new StringBuilder(firstLine);
            } else if (currentItem != null && line.length() >= itemPrefixLen) {
                // Continuation line — strip the item's full prefix length
                String content = line.substring(itemPrefixLen);
                currentItem.append("\n").append(content);
            }
        }

        if (currentItem != null) {
            String item = currentItem.toString();
            if (currentLevel > 1) {
                item = levelLine(currentLevel) + "\n" + item;
            }
            items.add(item);
        }

        return String.join("\n\n", items);
    }

    private static String levelLine(int level) {
        return "<!-- " + "@".repeat(level) + " -->";
    }

    /**
     * Find the position of "- " list marker after optional indent (" " or "\t").
     * Returns the index of '-' or -1 if not a list item line.
     */
    private Optional<Marker> findListMarkerPos(String line) {
        int pos = 0;
        int level = 1; // level starts from 1 (level 1 = no indent)
        while (pos < line.length()) {
            if (pos + 1 < line.length() && line.charAt(pos) == '-' && line.charAt(pos + 1) == ' ') {
                return Optional.of(new Marker(pos, level));
            } else if (pos + 1 < line.length() && line.charAt(pos) == ' ' && line.charAt(pos + 1) == ' ') {
                pos += 2;
                level++;
            } else if (line.charAt(pos) == '\t') {
                pos += 1;
                level++;
            } else {
                return Optional.empty();
            }
        }
        return Optional.empty();
    }

}
