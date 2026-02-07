package com.petajournal.server.infrastructure.parser;

import com.petajournal.server.domain.repository.MarkdownParser;
import jakarta.annotation.PostConstruct;
import jakarta.enterprise.context.ApplicationScoped;
import org.eclipse.microprofile.config.inject.ConfigProperty;
import org.jboss.logging.Logger;

import java.lang.foreign.*;
import java.lang.invoke.MethodHandle;
import java.nio.file.Files;
import java.nio.file.Path;

@ApplicationScoped
public class CmarkGfmParser implements MarkdownParser {

    private static final Logger LOG = Logger.getLogger(CmarkGfmParser.class);

    @ConfigProperty(name = "petajournal.lib.dir", defaultValue = "lib")
    String libDir;

    private MethodHandle parseHandle;
    private MethodHandle freeHandle;
    private boolean available;

    // MarkdownResult struct layout: { char* html, int error_code, char* error_message }
    private static final MemoryLayout RESULT_LAYOUT = MemoryLayout.structLayout(
            ValueLayout.ADDRESS.withName("html"),
            ValueLayout.JAVA_INT.withName("error_code"),
            MemoryLayout.paddingLayout(4), // padding for alignment
            ValueLayout.ADDRESS.withName("error_message")
    );

    @PostConstruct
    void init() {
        try {
            Path libPath = resolveLibraryPath();
            if (!Files.exists(libPath)) {
                LOG.warnv("Native markdown library not found at: {0}. Using fallback parser.", libPath);
                available = false;
                return;
            }

            Linker linker = Linker.nativeLinker();
            SymbolLookup lib = SymbolLookup.libraryLookup(libPath, Arena.global());

            // MarkdownResult* peta_markdown_parse(const char* markdown_text)
            parseHandle = linker.downcallHandle(
                    lib.find("peta_markdown_parse").orElseThrow(),
                    FunctionDescriptor.of(ValueLayout.ADDRESS, ValueLayout.ADDRESS)
            );

            // void peta_markdown_free_result(MarkdownResult* result)
            freeHandle = linker.downcallHandle(
                    lib.find("peta_markdown_free_result").orElseThrow(),
                    FunctionDescriptor.ofVoid(ValueLayout.ADDRESS)
            );

            available = true;
            LOG.infov("Native markdown parser loaded from: {0}", libPath);
        } catch (Exception e) {
            LOG.warn("Failed to load native markdown library. Using fallback parser.", e);
            available = false;
        }
    }

    @Override
    public String parseToHtml(String markdown) {
        if (!available) {
            return fallbackParse(markdown);
        }

        try (Arena arena = Arena.ofConfined()) {
            MemorySegment input = arena.allocateFrom(markdown);
            MemorySegment resultPtr = (MemorySegment) parseHandle.invoke(input);

            if (resultPtr.equals(MemorySegment.NULL)) {
                return fallbackParse(markdown);
            }

            resultPtr = resultPtr.reinterpret(RESULT_LAYOUT.byteSize());

            int errorCode = resultPtr.get(ValueLayout.JAVA_INT, ValueLayout.ADDRESS.byteSize());
            if (errorCode != 0) {
                MemorySegment errMsgPtr = resultPtr.get(ValueLayout.ADDRESS,
                        ValueLayout.ADDRESS.byteSize() + 4 + 4); // after html + error_code + padding
                errMsgPtr = errMsgPtr.reinterpret(1024);
                String errMsg = errMsgPtr.getString(0);
                LOG.warnv("Markdown parse error ({0}): {1}", errorCode, errMsg);
                freeHandle.invoke(resultPtr);
                return fallbackParse(markdown);
            }

            MemorySegment htmlPtr = resultPtr.get(ValueLayout.ADDRESS, 0);
            htmlPtr = htmlPtr.reinterpret(65536); // max expected size
            String html = htmlPtr.getString(0);

            freeHandle.invoke(resultPtr);
            return html;
        } catch (Throwable e) {
            LOG.warn("Error calling native markdown parser", e);
            return fallbackParse(markdown);
        }
    }

    private Path resolveLibraryPath() {
        String os = System.getProperty("os.name", "").toLowerCase();
        String arch;
        String ext;

        if (os.contains("linux")) {
            arch = "linux";
            ext = "so";
        } else if (os.contains("mac") || os.contains("darwin")) {
            arch = "macos";
            ext = "dylib";
        } else if (os.contains("win")) {
            arch = "windows";
            ext = "dll";
        } else {
            arch = "linux";
            ext = "so";
        }

        return Path.of(libDir, arch, "libpeta_markdown." + ext);
    }

    /**
     * Simple fallback when native library is not available.
     * Wraps each line in a paragraph tag.
     */
    private String fallbackParse(String markdown) {
        if (markdown == null || markdown.isEmpty()) {
            return "";
        }
        // Simple fallback: escape HTML and wrap in <p> tags
        String escaped = markdown
                .replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;");
        StringBuilder sb = new StringBuilder();
        for (String line : escaped.split("\n\n")) {
            if (!line.isBlank()) {
                sb.append("<p>").append(line.strip()).append("</p>\n");
            }
        }
        return sb.toString();
    }
}
