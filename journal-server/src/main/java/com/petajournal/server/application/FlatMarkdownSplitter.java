package com.petajournal.server.application;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.petajournal.server.domain.repository.MarkdownParser;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import java.io.IOException;
import java.io.UncheckedIOException;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * Splits a Flat Markdown page into items by the rules of spec v0.2.
 * The split itself is done by the flatmarkdown library. This class only maps its
 * page JSON to plain records and drops the per-item AST, which callers do not need.
 */
@ApplicationScoped
public class FlatMarkdownSplitter {

    private static final ObjectMapper MAPPER = new ObjectMapper();

    private final MarkdownParser markdownParser;

    /**
     * Creates the splitter.
     *
     * @param markdownParser parses a whole page
     * @throws NullPointerException if markdownParser is null
     */
    @Inject
    public FlatMarkdownSplitter(MarkdownParser markdownParser) {
        this.markdownParser = Objects.requireNonNull(markdownParser, "markdownParser must not be null");
    }

    /**
     * One item of a page as written in the source.
     *
     * @param level the indent level; 1 when the metadata line is omitted
     * @param id the HMTID from the metadata line, or null when absent
     * @param properties the key-value pairs from the metadata line; empty when absent
     * @param body the Markdown source of the item, without leading and trailing blank lines
     */
    public record SplitItem(int level, String id, Map<String, String> properties, String body) {}

    /**
     * A page split into front matter and items.
     *
     * @param frontMatter the raw YAML front matter including the {@code ---} lines, or null when absent
     * @param items the items in source order
     */
    public record SplitPage(String frontMatter, List<SplitItem> items) {}

    /**
     * Splits a page.
     *
     * @param markdown the page source; null or empty gives a page without items
     * @return the front matter and the items
     */
    public SplitPage split(String markdown) {
        if (markdown == null || markdown.isEmpty()) {
            return new SplitPage(null, List.of());
        }
        JsonNode page = readPage(markdownParser.parsePage(markdown));

        JsonNode fm = page.get("front_matter");
        String frontMatter = fm == null || fm.isNull() ? null : fm.asText();

        List<SplitItem> items = new ArrayList<>();
        for (JsonNode node : page.path("items")) {
            items.add(toSplitItem(node));
        }
        return new SplitPage(frontMatter, items);
    }

    private static SplitItem toSplitItem(JsonNode node) {
        JsonNode idNode = node.get("id");
        String id = idNode == null || idNode.isNull() ? null : idNode.asText();
        Map<String, String> properties = new LinkedHashMap<>();
        for (var entry : node.path("properties").properties()) {
            properties.put(entry.getKey(), entry.getValue().asText());
        }
        return new SplitItem(node.path("level").asInt(1), id, properties, node.path("body").asText(""));
    }

    private static JsonNode readPage(String pageJson) {
        try {
            return MAPPER.readTree(pageJson);
        } catch (IOException e) {
            throw new UncheckedIOException("flatmarkdown returned invalid page JSON", e);
        }
    }
}
