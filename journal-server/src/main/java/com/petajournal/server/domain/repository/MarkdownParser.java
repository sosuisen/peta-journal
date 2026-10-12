package com.petajournal.server.domain.repository;

/** Converts Flat Markdown text to AST JSON. */
public interface MarkdownParser {

    /**
     * Parses the body of one item and returns the AST JSON representation.
     *
     * @param markdown the markdown source of one item body
     * @return the AST JSON representation
     */
    String parseToAst(String markdown);

    /**
     * Splits a whole Flat Markdown page into items.
     * The result follows the flatmarkdown page JSON:
     * {@code {"type":"page","front_matter":string|null,"items":[{level,id,properties,body,ast}]}}.
     *
     * @param markdown the markdown source of a whole page
     * @return the page JSON representation
     */
    String parsePage(String markdown);
}
