package com.petajournal.server.infrastructure.parser;

import com.sosuisha.flatmarkdown.FlatmarkdownJava;
import com.petajournal.server.domain.repository.MarkdownParser;
import jakarta.annotation.Priority;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.enterprise.inject.Alternative;

/** {@link MarkdownParser} backed by the native flatmarkdown library. */
@ApplicationScoped
@Alternative
@Priority(1)
public class FlatmarkdownParser implements MarkdownParser {

    @Override
    public String parseToAst(String markdown) {
        if (markdown == null || markdown.isEmpty()) {
            return "";
        }
        return FlatmarkdownJava.bodyToAst(markdown);
    }

    @Override
    public String parsePage(String markdown) {
        if (markdown == null) {
            return FlatmarkdownJava.pageToAst("");
        }
        return FlatmarkdownJava.pageToAst(markdown);
    }
}
