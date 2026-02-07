#include "peta_markdown.h"

#include <cmark-gfm.h>
#include <cmark-gfm-core-extensions.h>
#include <cstring>
#include <cstdlib>

static const char* VERSION = "1.0.0";

static MarkdownResult* make_error(int code, const char* message) {
    auto* result = static_cast<MarkdownResult*>(calloc(1, sizeof(MarkdownResult)));
    result->error_code = code;
    result->error_message = strdup(message);
    return result;
}

extern "C" MarkdownResult* peta_markdown_parse(const char* markdown_text) {
    if (!markdown_text) {
        return make_error(1, "Input markdown text is null");
    }

    cmark_gfm_core_extensions_ensure_registered();

    cmark_parser* parser = cmark_parser_new(CMARK_OPT_DEFAULT);
    if (!parser) {
        return make_error(2, "Failed to create cmark parser");
    }

    const char* extension_names[] = {"table", "strikethrough", "autolink", "tasklist"};
    for (const char* name : extension_names) {
        cmark_syntax_extension* ext = cmark_find_syntax_extension(name);
        if (ext) {
            cmark_parser_attach_syntax_extension(parser, ext);
        }
    }

    size_t len = strlen(markdown_text);
    cmark_parser_feed(parser, markdown_text, len);

    cmark_node* doc = cmark_parser_finish(parser);
    if (!doc) {
        cmark_parser_free(parser);
        return make_error(3, "Failed to parse markdown document");
    }

    char* html = cmark_render_html(doc, CMARK_OPT_DEFAULT, cmark_parser_get_syntax_extensions(parser));

    cmark_node_free(doc);
    cmark_parser_free(parser);

    if (!html) {
        return make_error(4, "Failed to render HTML");
    }

    auto* result = static_cast<MarkdownResult*>(calloc(1, sizeof(MarkdownResult)));
    result->html = html;
    result->error_code = 0;
    result->error_message = nullptr;
    return result;
}

extern "C" void peta_markdown_free_result(MarkdownResult* result) {
    if (!result) return;
    if (result->html) free(result->html);
    if (result->error_message) free(result->error_message);
    free(result);
}

extern "C" const char* peta_markdown_version(void) {
    return VERSION;
}
