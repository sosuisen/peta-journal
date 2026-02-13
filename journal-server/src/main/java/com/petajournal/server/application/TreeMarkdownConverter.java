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
        if (blocks.isEmpty())
            return "";

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
        if (treeMd == null || treeMd.isEmpty())
            return "";

        List<String> items = new ArrayList<>();
        String[] lines = treeMd.replace("\r", "").split("\n", -1);
        StringBuilder currentItem = null;
        int blockPrefixLen = 0;

        for (String line : lines) {
            int markerPos = findListMarkerPos(line);

            if (markerPos >= 0) {
                // New block — flush previous
                if (currentItem != null) {
                    items.add(currentItem.toString());
                }
                blockPrefixLen = markerPos + 2; // indent + "- "
                currentItem = new StringBuilder(removeOneIndent(line));
            } else if (currentItem != null && line.length() >= blockPrefixLen) {
                // Continuation line — strip the block's full prefix length
                String content = line.substring(blockPrefixLen);
                currentItem.append("\n").append(content);
            }
        }

        if (currentItem != null) {
            items.add(currentItem.toString());
        }

        return String.join("\n\n", items);
    }

    /**
     * Find the position of "- " list marker after optional indent ("  " or "\t").
     * Returns the index of '-' or -1 if not a list item line.
     */
    private int findListMarkerPos(String line) {
        int pos = 0;
        while (pos < line.length()) {
            if (pos + 1 < line.length() && line.charAt(pos) == '-' && line.charAt(pos + 1) == ' ') {
                return pos;
            } else if (pos + 1 < line.length() && line.charAt(pos) == ' ' && line.charAt(pos + 1) == ' ') {
                pos += 2;
            } else if (line.charAt(pos) == '\t') {
                pos += 1;
            } else {
                return -1;
            }
        }
        return -1;
    }

    /**
     * Remove one indent token from the start of a line.
     * One token is "- " (list marker), "  " (double space), or "\t" (tab).
     */
    private String removeOneIndent(String line) {
        if (line.startsWith("- ")) {
            return line.substring(2);
        } else if (line.startsWith("\t")) {
            return line.substring(1);
        } else if (line.startsWith("  ")) {
            return line.substring(2);
        }
        return line;
    }
}
