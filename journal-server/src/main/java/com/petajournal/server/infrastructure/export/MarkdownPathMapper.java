package com.petajournal.server.infrastructure.export;

import com.petajournal.server.domain.model.Page;

import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.HexFormat;
import java.util.Optional;

/**
 * Maps pages to file paths inside the markdown directory and back.
 * Layout: {@code {type}s/{dirPath}{name}.md}, e.g. {@code docs/proj/a.md}, {@code journals/2026-09-27.md}.
 * Also decides which files in the folder are ours to look at.
 */
public final class MarkdownPathMapper {

    public static final String MD_EXT = ".md";
    private static final String CONFLICT_MARKER = ".conflict";

    private MarkdownPathMapper() {}

    /** Location of a page's markdown file, decoded from a relative path. */
    public record PageLocation(String type, String dirPath, String name) {}

    /** Relative path (with '/') for a page. */
    public static String toRelPath(Page page) {
        return toRelPath(page.type(), page.dirPath(), page.name());
    }

    public static String toRelPath(String type, String dirPath, String name) {
        String dir = dirPath == null ? "" : dirPath;
        return type + "s/" + dir + name + MD_EXT;
    }

    /**
     * Decodes a relative path into (type, dirPath, name).
     * Returns empty if the path is not a page file we manage.
     */
    public static Optional<PageLocation> fromRelPath(String relPath) {
        if (relPath == null || !relPath.endsWith(MD_EXT)) return Optional.empty();
        int slash = relPath.indexOf('/');
        if (slash <= 0) return Optional.empty();
        String top = relPath.substring(0, slash);
        String type;
        if ("docs".equals(top)) type = "doc";
        else if ("journals".equals(top)) type = "journal";
        else return Optional.empty();

        String rest = relPath.substring(slash + 1, relPath.length() - MD_EXT.length());
        if (rest.isEmpty()) return Optional.empty();
        int lastSlash = rest.lastIndexOf('/');
        String dirPath = lastSlash < 0 ? "" : rest.substring(0, lastSlash + 1);
        String name = lastSlash < 0 ? rest : rest.substring(lastSlash + 1);
        if (name.isEmpty()) return Optional.empty();
        return Optional.of(new PageLocation(type, dirPath, name));
    }

    /** Relative path of {@code file} under {@code root}, always using '/'. */
    public static String relativize(Path root, Path file) {
        return root.relativize(file).toString().replace('\\', '/');
    }

    /**
     * Files we must never import: temp/partial files from editors and Dropbox,
     * Dropbox conflicted copies, our own conflict backups, and anything not .md.
     */
    public static boolean isIgnored(String relPath) {
        if (relPath == null || !relPath.endsWith(MD_EXT)) return true;
        if (hasIgnoredSegment(relPath)) return true;
        String name = relPath.substring(relPath.lastIndexOf('/') + 1);
        String base = name.substring(0, name.length() - MD_EXT.length());
        if (base.contains(CONFLICT_MARKER)) return true;          // a.conflict.md / a.conflict-20260927-101500.md
        if (base.toLowerCase().contains("conflicted copy")) return true; // Dropbox
        return false;
    }

    /** True when any path segment is a hidden/temp entry: .dropbox, .dropbox.cache, .sb-xxxx, ~$lock files. */
    public static boolean hasIgnoredSegment(String relPath) {
        for (String segment : relPath.split("/")) {
            if (segment.isEmpty()) continue;
            if (segment.startsWith(".") || segment.startsWith("~$")) return true;
        }
        return false;
    }

    /** Name for a conflict backup written next to the page file. */
    public static String conflictFileName(String name, String timestamp) {
        return name + CONFLICT_MARKER + "-" + timestamp + MD_EXT;
    }

    public static String sha256(String content) {
        try {
            MessageDigest md = MessageDigest.getInstance("SHA-256");
            return HexFormat.of().formatHex(md.digest(content.getBytes(StandardCharsets.UTF_8)));
        } catch (NoSuchAlgorithmException e) {
            throw new IllegalStateException(e);
        }
    }
}
