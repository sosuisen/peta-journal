#include "peta_markdown.h"
#include <cstdio>
#include <cstring>
#include <cstdlib>

static int tests_passed = 0;
static int tests_failed = 0;

static void check(const char* test_name, MarkdownResult* result, const char* expected_substr) {
    if (!result) {
        printf("FAIL [%s]: null result\n", test_name);
        tests_failed++;
        return;
    }
    if (result->error_code != 0) {
        printf("FAIL [%s]: error %d: %s\n", test_name, result->error_code, result->error_message);
        tests_failed++;
        peta_markdown_free_result(result);
        return;
    }
    if (strstr(result->html, expected_substr)) {
        printf("PASS [%s]\n", test_name);
        tests_passed++;
    } else {
        printf("FAIL [%s]: expected substring '%s' in:\n%s\n", test_name, expected_substr, result->html);
        tests_failed++;
    }
    peta_markdown_free_result(result);
}

int main() {
    printf("peta_markdown version: %s\n\n", peta_markdown_version());

    // Basic markdown
    check("Bold text",
          peta_markdown_parse("**hello**"),
          "<strong>hello</strong>");

    check("Italic text",
          peta_markdown_parse("*world*"),
          "<em>world</em>");

    check("Heading",
          peta_markdown_parse("# Title"),
          "<h1>Title</h1>");

    // GFM table
    check("GFM table",
          peta_markdown_parse("| A | B |\n|---|---|\n| 1 | 2 |\n"),
          "<table>");

    // Task list
    check("Task list checked",
          peta_markdown_parse("- [x] Done\n- [ ] Todo\n"),
          "checked");

    // Strikethrough
    check("Strikethrough",
          peta_markdown_parse("~~deleted~~"),
          "<del>deleted</del>");

    // Code block
    check("Code block",
          peta_markdown_parse("```java\nint x = 1;\n```\n"),
          "<code");

    // Autolink
    check("Autolink",
          peta_markdown_parse("Visit https://example.com today"),
          "<a href=\"https://example.com\"");

    // Null input
    {
        MarkdownResult* r = peta_markdown_parse(nullptr);
        if (r && r->error_code == 1) {
            printf("PASS [Null input error handling]\n");
            tests_passed++;
        } else {
            printf("FAIL [Null input error handling]\n");
            tests_failed++;
        }
        peta_markdown_free_result(r);
    }

    printf("\n--- Results: %d passed, %d failed ---\n", tests_passed, tests_failed);
    return tests_failed > 0 ? 1 : 0;
}
