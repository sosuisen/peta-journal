package com.petajournal.server.domain.repository;

public interface MarkdownParser {

    String parseToHtml(String markdown);
}
