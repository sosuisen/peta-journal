package com.petajournal.server.application;

import com.petajournal.server.domain.model.Block;
import jakarta.enterprise.context.ApplicationScoped;

import java.util.ArrayList;
import java.util.List;

@ApplicationScoped
public class TreeMarkdownConverter {

    /**
     * Convert a list of blocks to Tree Markdown (nested list format).
     * Normalizes levels relative to the minimum level in the list.
     */
    public String blocksToTreeMarkdown(List<Block> blocks) {
        if (blocks.isEmpty()) return "";

        int minLevel = blocks.stream().mapToInt(Block::level).min().orElse(0);
        StringBuilder sb = new StringBuilder();

        for (int i = 0; i < blocks.size(); i++) {
            Block block = blocks.get(i);
            int depth = block.level() - minLevel;
            String indent = "  ".repeat(depth);
            String[] lines = block.body().split("\n", -1);

            // First line with list marker
            sb.append(indent).append("- ").append(lines[0]);

            // Continuation lines
            for (int j = 1; j < lines.length; j++) {
                sb.append("\n").append(indent).append("  ").append(lines[j]);
            }

            if (i < blocks.size() - 1) {
                sb.append("\n");
            }
        }

        return sb.toString();
    }

    /**
     * Convert a list of blocks to Flat Markdown (blank-line separated bodies).
     */
    public String blocksToFlatMarkdown(List<Block> blocks) {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < blocks.size(); i++) {
            sb.append(blocks.get(i).body());
            if (i < blocks.size() - 1) {
                sb.append("\n\n");
            }
        }
        return sb.toString();
    }

    /**
     * Parse Tree Markdown into Flat Markdown.
     * Each list item becomes a separate paragraph, separated by blank lines.
     */
    public String treeMarkdownToFlatMarkdown(String treeMd) {
        if (treeMd == null || treeMd.isEmpty()) return "";

        List<String> items = new ArrayList<>();
        String[] lines = treeMd.split("\n", -1);
        StringBuilder currentItem = null;

        for (String line : lines) {
            // Count leading spaces
            int spaces = 0;
            while (spaces < line.length() && line.charAt(spaces) == ' ') {
                spaces++;
            }

            String trimmedFromIndent = line.substring(spaces);

            // Check if this is a new list item
            if (trimmedFromIndent.startsWith("- ")) {
                // Flush previous item
                if (currentItem != null) {
                    items.add(currentItem.toString());
                }
                // Start new item - strip the "- " prefix
                currentItem = new StringBuilder(trimmedFromIndent.substring(2));
            } else if (currentItem != null) {
                // Continuation line - strip the indent
                String content = trimmedFromIndent;
                currentItem.append("\n").append(content);
            }
        }

        // Flush last item
        if (currentItem != null) {
            items.add(currentItem.toString());
        }

        return String.join("\n\n", items);
    }
}
