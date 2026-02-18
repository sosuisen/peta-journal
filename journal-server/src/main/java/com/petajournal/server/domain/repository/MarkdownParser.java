package com.petajournal.server.domain.repository;

/** Converts markdown text to HTML. */
public interface MarkdownParser {

    /**
     * Parses markdown text and returns the HTML representation.
     *
     * @param markdown the markdown source
     * @return the rendered HTML
     */
    String parseToHtml(String markdown);
}
