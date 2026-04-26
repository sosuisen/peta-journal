package com.petajournal.server.domain;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;

/**
 * Extracts hashtag values from a parsed AST JSON string.
 */
public final class HashtagExtractor {

    private static final ObjectMapper MAPPER = new ObjectMapper();

    private HashtagExtractor() {}

    /**
     * Parses the AST JSON and collects all hashtag values, deduplicated and in order.
     *
     * @param astJson the AST JSON string
     * @return list of unique hashtag values (without the '#' prefix)
     */
    public static List<String> extractHashtags(String astJson) {
        if (astJson == null || astJson.isBlank()) {
            return List.of();
        }
        try {
            JsonNode root = MAPPER.readTree(astJson);
            Set<String> tags = new LinkedHashSet<>();
            collectHashtags(root, tags);
            return new ArrayList<>(tags);
        } catch (Exception e) {
            return List.of();
        }
    }

    private static void collectHashtags(JsonNode node, Set<String> tags) {
        if (node.isArray()) {
            var it = node.elements();
            while (it.hasNext()) {
                collectHashtags(it.next(), tags);
            }
        } else if (node.isObject()) {
            if ("hashtag".equals(nodeText(node, "type"))) {
                String value = nodeText(node, "value");
                if (value != null && !value.isEmpty()) {
                    tags.add(value);
                }
            }
            JsonNode children = node.get("children");
            if (children != null) {
                collectHashtags(children, tags);
            }
            // Also recurse into all object fields in case AST nests differently
            for (var entry : node.properties()) {
                JsonNode val = entry.getValue();
                if (val.isArray() || val.isObject()) {
                    collectHashtags(val, tags);
                }
            }
        }
    }

    private static String nodeText(JsonNode node, String field) {
        JsonNode child = node.get(field);
        return child != null && child.isTextual() ? child.asText() : null;
    }
}
