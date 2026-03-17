package com.petajournal.server.domain.model;

import java.util.UUID;

public sealed interface ItemPosition {
    ItemPosition FIRST = new First();
    ItemPosition LAST = new Last();

    static ItemPosition after(UUID itemId) {
        return new After(itemId);
    }

    static ItemPosition parse(String s) {
        if ("first".equals(s)) return FIRST;
        if ("last".equals(s)) return LAST;
        if (s.startsWith("after:")) return after(UUID.fromString(s.substring(6)));
        throw new IllegalArgumentException("Unknown position: " + s);
    }

    record First() implements ItemPosition {}
    record Last() implements ItemPosition {}
    record After(UUID itemId) implements ItemPosition {}
}
