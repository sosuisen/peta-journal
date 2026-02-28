package com.petajournal.server.infrastructure.parser;

import com.petajournal.flatmarkdown.FlatmarkdownJava;
import com.petajournal.server.domain.repository.MarkdownParser;
import jakarta.annotation.Priority;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.enterprise.inject.Alternative;

@ApplicationScoped
@Alternative
@Priority(1)
public class FlatmarkdownParser implements MarkdownParser {

    @Override
    public String parseToHtml(String markdown) {
        if (markdown == null || markdown.isEmpty()) {
            return "";
        }
        return FlatmarkdownJava.markdownToHtml(markdown);
    }
}
