package com.petajournal.server.infrastructure.export;

import com.petajournal.server.domain.JournalFileNameMatcher;

import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import java.util.Optional;
import java.util.function.Function;
import java.util.function.UnaryOperator;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Converts page links between the in-app form and the on-disk form of a page body.
 * In the app a body holds wikilinks ({@code [[path]]}, {@code [[path#id|label]]}) and
 * hashtags ({@code #tag}). On disk they become Markdown links whose destination is the
 * path of the target file relative to the file being written, so that the exported
 * Markdown works in any Markdown tool on its own.
 * Text inside fenced code blocks and inline code spans is never converted.
 */
public final class PageLinkConverter {

    private static final Pattern WIKILINK = Pattern.compile(
            "\\[\\[([^\\[\\]|]+?)(?:\\|([^\\[\\]]*))?\\]\\]");
    private static final Pattern HASHTAG = Pattern.compile(
            "(?<=^| )#([\\p{L}\\p{N}_/-]+)", Pattern.MULTILINE);
    private static final Pattern MARKDOWN_LINK = Pattern.compile(
            "(?<!!)\\[([^\\[\\]]*)\\]\\((<[^<>]*>|[^()\\s]+)\\)");
    private static final Pattern INLINE_CODE = Pattern.compile("(`+)[\\s\\S]*?\\1");
    private static final Pattern FENCE = Pattern.compile("^\\s{0,3}(```|~~~)");

    private PageLinkConverter() {}

    /**
     * Rewrites wikilinks and hashtags as relative Markdown links for the file at {@code sourceRelPath}.
     *
     * @param body the body as stored in the app
     * @param sourceRelPath the path of the file being written, relative to the markdown root
     *                      (for example {@code docs/a/b.md})
     * @return the body to write to disk
     * @throws NullPointerException if any argument is null
     */
    public static String toMarkdownLinks(String body, String sourceRelPath) {
        Objects.requireNonNull(body, "body must not be null");
        Objects.requireNonNull(sourceRelPath, "sourceRelPath must not be null");
        String sourceDir = dirOf(sourceRelPath);
        return mapOutsideCode(body, text -> hashtagsToLinks(wikilinksToLinks(text, sourceDir), sourceDir));
    }

    /**
     * Rewrites relative Markdown links that point at a page file back into wikilinks and hashtags.
     * Links that do not resolve to a page file under {@code docs/} or {@code journals/} are left as they are.
     *
     * @param body the body as read from disk
     * @param sourceRelPath the path of the file being read, relative to the markdown root
     * @return the body to store in the app
     * @throws NullPointerException if any argument is null
     */
    public static String toWikilinks(String body, String sourceRelPath) {
        Objects.requireNonNull(body, "body must not be null");
        Objects.requireNonNull(sourceRelPath, "sourceRelPath must not be null");
        String sourceDir = dirOf(sourceRelPath);
        return mapOutsideCode(body, text -> linksToWikilinks(text, sourceDir));
    }

    // ---- export direction ----

    private static String wikilinksToLinks(String text, String sourceDir) {
        return replaceAll(WIKILINK, text, m -> {
            String target = m.group(1);
            String label = m.group(2);
            int hash = target.indexOf('#');
            String pagePath = hash < 0 ? target : target.substring(0, hash);
            String fragment = hash < 0 ? "" : target.substring(hash);
            String dest = relativize(sourceDir, pageFileOf(pagePath)) + fragment;
            return markdownLink(label != null ? label : target, dest);
        });
    }

    private static String hashtagsToLinks(String text, String sourceDir) {
        return replaceAll(HASHTAG, text, m -> {
            String tag = trimTrailingSlashes(m.group(1));
            if (tag.isEmpty() || tag.startsWith("/")) {
                return m.group();
            }
            String rest = m.group(1).substring(tag.length());
            return markdownLink("#" + tag, relativize(sourceDir, pageFileOf(tag))) + rest;
        });
    }

    private static String markdownLink(String label, String dest) {
        boolean needsBrackets = dest.chars().anyMatch(c -> c == ' ' || c == '(' || c == ')' || c == '<' || c == '>');
        return "[" + label + "](" + (needsBrackets ? "<" + dest + ">" : dest) + ")";
    }

    /** Maps a page path (dirPath + name) to its file path under the markdown root. */
    private static String pageFileOf(String pagePath) {
        int slash = pagePath.lastIndexOf('/');
        String dirPath = slash < 0 ? "" : pagePath.substring(0, slash + 1);
        String name = slash < 0 ? pagePath : pagePath.substring(slash + 1);
        String type = dirPath.isEmpty() && JournalFileNameMatcher.isJournalFileName(name) ? "journal" : "doc";
        return MarkdownPathMapper.toRelPath(type, dirPath, name);
    }

    // ---- import direction ----

    private static String linksToWikilinks(String text, String sourceDir) {
        return replaceAll(MARKDOWN_LINK, text, m -> {
            String label = m.group(1);
            String dest = unwrapBrackets(m.group(2));
            int hash = dest.indexOf('#');
            String file = hash < 0 ? dest : dest.substring(0, hash);
            String fragment = hash < 0 ? "" : dest.substring(hash);
            Optional<String> pagePath = pagePathOf(resolve(sourceDir, file));
            if (pagePath.isEmpty()) {
                return m.group();
            }
            return wikilinkOrHashtag(label, pagePath.get(), fragment);
        });
    }

    private static String wikilinkOrHashtag(String label, String pagePath, String fragment) {
        if (fragment.isEmpty() && label.equals("#" + pagePath)) {
            return "#" + pagePath;
        }
        String target = pagePath + fragment;
        if (label.equals(target) || label.equals(pagePath)) {
            return "[[" + target + "]]";
        }
        return "[[" + target + "|" + label + "]]";
    }

    private static Optional<String> pagePathOf(Optional<String> relPath) {
        return relPath.flatMap(MarkdownPathMapper::fromRelPath).map(loc -> loc.dirPath() + loc.name());
    }

    private static String unwrapBrackets(String dest) {
        if (dest.startsWith("<") && dest.endsWith(">")) {
            return dest.substring(1, dest.length() - 1);
        }
        return dest;
    }

    // ---- path helpers (POSIX style, independent of the host file system) ----

    private static String dirOf(String relPath) {
        int slash = relPath.lastIndexOf('/');
        return slash < 0 ? "" : relPath.substring(0, slash);
    }

    /** Returns the path of {@code targetRelPath} relative to {@code sourceDir}; both are root-relative. */
    static String relativize(String sourceDir, String targetRelPath) {
        List<String> from = segments(sourceDir);
        List<String> to = segments(targetRelPath);
        int common = 0;
        while (common < from.size() && common < to.size() - 1 && from.get(common).equals(to.get(common))) {
            common++;
        }
        var sb = new StringBuilder();
        for (int i = common; i < from.size(); i++) {
            sb.append("../");
        }
        sb.append(String.join("/", to.subList(common, to.size())));
        return sb.toString();
    }

    /** Resolves {@code relative} against {@code sourceDir}; empty when it escapes the root. */
    static Optional<String> resolve(String sourceDir, String relative) {
        if (relative.isEmpty() || relative.startsWith("/") || relative.contains("://")) {
            return Optional.empty();
        }
        List<String> stack = new ArrayList<>(segments(sourceDir));
        for (String seg : relative.split("/")) {
            if (seg.isEmpty() || seg.equals(".")) {
                continue;
            }
            if (seg.equals("..")) {
                if (stack.isEmpty()) {
                    return Optional.empty();
                }
                stack.removeLast();
            } else {
                stack.add(seg);
            }
        }
        return Optional.of(String.join("/", stack));
    }

    private static List<String> segments(String path) {
        List<String> out = new ArrayList<>();
        for (String seg : path.split("/")) {
            if (!seg.isEmpty()) {
                out.add(seg);
            }
        }
        return out;
    }

    private static String trimTrailingSlashes(String s) {
        int end = s.length();
        while (end > 0 && s.charAt(end - 1) == '/') {
            end--;
        }
        return s.substring(0, end);
    }

    // ---- code protection ----

    /** Applies {@code fn} to every stretch of text that is outside fenced blocks and inline code. */
    private static String mapOutsideCode(String body, UnaryOperator<String> fn) {
        var out = new StringBuilder();
        var textRun = new StringBuilder();
        boolean inFence = false;
        String[] lines = body.split("\n", -1);
        for (int i = 0; i < lines.length; i++) {
            String line = lines[i];
            String nl = i < lines.length - 1 ? "\n" : "";
            if (FENCE.matcher(line).find()) {
                flush(out, textRun, fn);
                out.append(line).append(nl);
                inFence = !inFence;
            } else if (inFence) {
                out.append(line).append(nl);
            } else {
                textRun.append(line).append(nl);
            }
        }
        flush(out, textRun, fn);
        return out.toString();
    }

    private static void flush(StringBuilder out, StringBuilder textRun, UnaryOperator<String> fn) {
        if (textRun.isEmpty()) {
            return;
        }
        // Inline code spans stay untouched; only the text between them is converted
        out.append(replaceAll(INLINE_CODE, textRun.toString(), fn, Matcher::group));
        textRun.setLength(0);
    }

    private static String replaceAll(Pattern p, String text, Function<Matcher, String> onMatch) {
        return replaceAll(p, text, UnaryOperator.identity(), onMatch);
    }

    /** Rebuilds {@code text}, passing gaps through {@code onGap} and matches through {@code onMatch}. */
    private static String replaceAll(Pattern p, String text, UnaryOperator<String> onGap,
                                     Function<Matcher, String> onMatch) {
        var sb = new StringBuilder();
        Matcher m = p.matcher(text);
        int last = 0;
        while (m.find()) {
            sb.append(onGap.apply(text.substring(last, m.start())));
            sb.append(onMatch.apply(m));
            last = m.end();
        }
        sb.append(onGap.apply(text.substring(last)));
        return sb.toString();
    }
}
