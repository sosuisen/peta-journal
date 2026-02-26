package com.petajournal.server.application;

import com.petajournal.server.domain.model.Block;
import jakarta.enterprise.context.ApplicationScoped;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

@ApplicationScoped
public class TreeMarkdownConverter {
    record Marker(int pos, int level) {
    }

    /**
     * Convert a list of blocks to Tree Markdown (nested list format).
     * Normalizes levels relative to the minimum level in the list.
     */
    public String blocksToTreeMarkdown(List<Block> blocks) {
        if (blocks.isEmpty())
            return "";

        int minLevel = blocks.stream().mapToInt(Block::level).min().orElse(1);
        StringBuilder sb = new StringBuilder();

        for (int i = 0; i < blocks.size(); i++) {
            Block block = blocks.get(i);
            int depth = block.level() - minLevel;
            String indent = "  ".repeat(depth);
            String[] lines = block.body().split("\n", -1);

            // First line with list marker (and GFM task prefix for todo/done)
            String status = block.status() != null ? block.status() : "normal";
            String marker = "- ";
            if ("todo".equals(status))
                marker = "- [ ] ";
            else if ("done".equals(status))
                marker = "- [x] ";
            sb.append(indent).append(marker).append(lines[0]);

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
     * Convert a list of blocks to Flat Markdown (HTML comment metadata format).
     */
    public String blocksToFlatMarkdown(List<Block> blocks) {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < blocks.size(); i++) {
            Block block = blocks.get(i);
            String body = block.body();
            // Empty body → single space (blank line would be a block separator)
            if (body == null || body.isEmpty())
                body = " ";

            String status = block.status() != null ? block.status() : "normal";

            // Prepend GFM task list prefix for todo/done
            if ("todo".equals(status)) {
                body = "- [ ] " + body;
            } else if ("done".equals(status)) {
                body = "- [x] " + body;
            }

            sb.append(body).append("\n");
            sb.append("<!-- ").append(block.id()).append(" -->");

            if (block.level() > 1) {
                sb.append("\n<!-- level=").append(block.level()).append(" -->");
            }

            if (block.collapsed()) {
                sb.append("\n<!-- collapsed=true -->");
            }

            if (i < blocks.size() - 1) {
                sb.append("\n\n");
            }
        }
        return sb.toString();
    }

    /**
     * Parse Tree Markdown into Flat Markdown with HTML comment metadata.
     * Each list item becomes a separate block with <!-- level=N --> when depth > 0.
     */
    public String treeMarkdownToFlatMarkdown(String treeMd) {
        if (treeMd == null || treeMd.isEmpty())
            return "";

        List<String> items = new ArrayList<>();
        String[] lines = treeMd.replace("\r", "").split("\n", -1);
        StringBuilder currentItem = null;
        int blockPrefixLen = 0;
        int currentLevel = 0;

        for (String line : lines) {
            Optional<Marker> marker = findListMarkerPos(line);

            if (marker.isPresent()) {
                // New block — flush previous
                if (currentItem != null) {
                    String block = currentItem.toString();
                    if (currentLevel > 1) {
                        block += "\n<!-- level=" + currentLevel + " -->";
                    }
                    items.add(block);
                }
                currentLevel = marker.get().level();
                blockPrefixLen = marker.get().pos() + 2; // indent + "- "
                String firstLine = line.substring(blockPrefixLen);
                // Re-add "- " for GFM task prefix so flat markdown parser can detect status
                if (firstLine.startsWith("[ ] ") || firstLine.startsWith("[x] ")) {
                    firstLine = "- " + firstLine;
                }
                currentItem = new StringBuilder(firstLine);
            } else if (currentItem != null && line.length() >= blockPrefixLen) {
                // Continuation line — strip the block's full prefix length
                String content = line.substring(blockPrefixLen);
                currentItem.append("\n").append(content);
            }
        }

        if (currentItem != null) {
            String block = currentItem.toString();
            if (currentLevel > 1) {
                block += "\n<!-- level=" + currentLevel + " -->";
            }
            items.add(block);
        }

        return String.join("\n\n", items);
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
