package com.petajournal.server.domain.repository;

/** Converts markdown text to AST JSON. */
public interface MarkdownParser {

    /**
     * Parses markdown text and returns the AST JSON representation.
     *
     * @param markdown the markdown source
     * @return the AST JSON representation
     */
    String parseToAst(String markdown);
}
