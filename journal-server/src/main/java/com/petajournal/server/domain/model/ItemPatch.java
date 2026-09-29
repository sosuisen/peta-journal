package com.petajournal.server.domain.model;

import java.time.Instant;
import java.util.List;
import java.util.Objects;

/**
 * A partial update of one item. Only the fields that are set are written.
 * The other columns keep the value they have in the database at write time,
 * so two updates of different fields never overwrite each other.
 *
 * @param body         new markdown body, or null to keep the current body
 * @param ast          parsed AST of the new body; required when body is set
 * @param bodyTypes    types found in the new body (heading, code, blockquote); required when body is set.
 *                     They replace every current type except {@code task}
 * @param task         true to add the {@code task} type, false to remove it, null to keep it
 * @param level        new indentation level, or null to keep the current level
 * @param collapsed    new collapsed state, or null to keep the current state
 * @param modifiedDate new modified date of the item
 */
public record ItemPatch(
    String body,
    String ast,
    List<String> bodyTypes,
    Boolean task,
    Integer level,
    Boolean collapsed,
    Instant modifiedDate
) {
    public static final String TASK_TYPE = "task";

    /**
     * Checks that a body comes with its AST and its types.
     *
     * @throws NullPointerException     if modifiedDate is null, or if body is set without ast or bodyTypes
     * @throws IllegalArgumentException if level is less than 1
     */
    public ItemPatch {
        Objects.requireNonNull(modifiedDate, "modifiedDate must not be null");
        if (body != null) {
            Objects.requireNonNull(ast, "ast must not be null when body is set");
            Objects.requireNonNull(bodyTypes, "bodyTypes must not be null when body is set");
            bodyTypes = List.copyOf(bodyTypes);
        }
        if (level != null && level < 1) {
            throw new IllegalArgumentException("level must be 1 or more: " + level);
        }
    }

    /**
     * Returns whether the patch changes the body.
     *
     * @return true if body is set
     */
    public boolean changesBody() {
        return body != null;
    }
}
