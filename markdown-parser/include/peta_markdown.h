#ifndef PETA_MARKDOWN_H
#define PETA_MARKDOWN_H

#ifdef _WIN32
  #define PETA_EXPORT __declspec(dllexport)
#else
  #define PETA_EXPORT __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    char* html;
    int error_code;
    char* error_message;
} MarkdownResult;

PETA_EXPORT MarkdownResult* peta_markdown_parse(const char* markdown_text);
PETA_EXPORT void peta_markdown_free_result(MarkdownResult* result);
PETA_EXPORT const char* peta_markdown_version(void);

#ifdef __cplusplus
}
#endif

#endif
