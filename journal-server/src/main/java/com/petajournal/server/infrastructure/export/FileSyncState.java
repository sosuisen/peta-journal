package com.petajournal.server.infrastructure.export;

import com.sosuisha.hmtid.HMTID;

/**
 * What we last knew about one markdown file in the export directory.
 *
 * @param relPath      path relative to the markdown directory, using '/' as separator
 * @param pageId       the page the file belongs to
 * @param contentHash  SHA-256 of the file content as last read or written by us
 * @param fileSize     file size at that time
 * @param fileMtime    file mtime (epoch millis) at that time
 * @param syncedAt     when the row was last updated (epoch millis)
 * @param status       {@code synced} or {@code missing}
 * @param missingSince when the file was first seen missing (epoch millis), or null
 */
public record FileSyncState(
        String relPath,
        HMTID pageId,
        String contentHash,
        long fileSize,
        long fileMtime,
        long syncedAt,
        String status,
        Long missingSince
) {
    public static final String SYNCED = "synced";
    public static final String MISSING = "missing";

    public boolean isMissing() {
        return MISSING.equals(status);
    }
}
