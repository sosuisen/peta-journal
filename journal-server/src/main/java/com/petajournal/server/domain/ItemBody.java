package com.petajournal.server.domain;

import java.util.ArrayList;
import java.util.List;

/**
 * The body text of an item as the user wrote it, split into the stored text and the
 * facts the server derives from it.
 * A GFM task prefix ({@code - [ ] } or {@code - [x] }) marks the item as a task and is
 * not stored. Headings, code blocks and blockquotes are recognized from the first
 * character of the text and stored as item types.
 *
 * @param text the body without the task prefix; never null
 * @param task true when the body started with a task prefix
 */
public record ItemBody(String text, boolean task) {

    public static final String TASK_TYPE = "task";
    public static final String HEADING_TYPE = "heading";
    public static final String CODE_TYPE = "code";
    public static final String BLOCKQUOTE_TYPE = "blockquote";

    private static final String OPEN_TASK_PREFIX = "- [ ] ";
    private static final String DONE_TASK_PREFIX = "- [x] ";

    /**
     * Splits a raw body into its text and its task flag.
     *
     * @param raw the body as written; null counts as empty
     * @return the body without the task prefix
     */
    public static ItemBody parse(String raw) {
        if (raw == null) {
            return new ItemBody("", false);
        }
        if (raw.startsWith(OPEN_TASK_PREFIX) || raw.startsWith(DONE_TASK_PREFIX)) {
            return new ItemBody(raw.substring(OPEN_TASK_PREFIX.length()), true);
        }
        return new ItemBody(raw, false);
    }

    /**
     * Returns the types read from the text: {@code heading}, {@code code} and {@code blockquote}.
     * The task type is not included; see {@link #task()}.
     *
     * @return the types in a fixed order, possibly empty
     */
    public List<String> textTypes() {
        List<String> types = new ArrayList<>();
        String trimmed = text.trim();
        if (trimmed.startsWith("#")) {
            types.add(HEADING_TYPE);
        }
        if (trimmed.startsWith("```")) {
            types.add(CODE_TYPE);
        }
        if (trimmed.startsWith(">")) {
            types.add(BLOCKQUOTE_TYPE);
        }
        return types;
    }
}
