package com.petajournal.server.domain.model;

import com.sosuisha.hmtid.HMTID;
import java.time.Instant;

public record Page(
    HMTID id,
    HMTID parentId,
    String dirPath,
    String name,
    String type,        // "journal" or "doc"
    Instant createdDate,
    Instant modifiedDate
) {
}
