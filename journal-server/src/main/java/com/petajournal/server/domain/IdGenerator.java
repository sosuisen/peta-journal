package com.petajournal.server.domain;

import com.sosuisha.hmtid.HMTID;
import com.sosuisha.hmtid.HmtidGenerator;
import jakarta.enterprise.context.ApplicationScoped;

@ApplicationScoped
public class IdGenerator {
    private final HmtidGenerator pageGenerator =
        HMTID.monotonicFactory(Math::random, '-', true);
    private final HmtidGenerator itemGenerator =
        HMTID.monotonicFactory(Math::random, '-', true);

    public HMTID generatePageId() { return pageGenerator.generate(); }
    public HMTID generateItemId() { return itemGenerator.generate(); }
}
