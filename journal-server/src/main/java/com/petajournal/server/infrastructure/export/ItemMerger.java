package com.petajournal.server.infrastructure.export;

import com.petajournal.server.domain.HashtagExtractor;
import com.petajournal.server.domain.IdGenerator;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.MarkdownParser;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.infrastructure.export.OutlineMarkdownParser.ParsedItem;
import com.petajournal.server.infrastructure.persistence.PropertyRepository;
import com.sosuisha.hmtid.HMTID;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.time.Instant;
import java.util.*;

/**
 * Merges the items read from a markdown file into the items a page has in the database.
 * Items are matched by id; unchanged ones are left alone, including their index.
 * Items missing from the file are deleted, since the file is the newer version of the page.
 */
@ApplicationScoped
public class ItemMerger {

    private static final Logger LOG = Logger.getLogger(ItemMerger.class);
    private static final Set<String> RESERVED_KEYS = Set.of("level", "collapsed", "transclude", "tag", "idx");
    private static final String ITEM_TARGET = "item";

    /**
     * @param changed        any item was added, changed or removed
     * @param needsWriteBack the file lacks an id or idx, or its idx comments disagree with the line order
     */
    public record MergeResult(boolean changed, boolean needsWriteBack) {}

    private final ItemRepository itemRepository;
    private final PropertyRepository propertyRepository;
    private final MarkdownParser markdownParser;
    private final SearchService searchService;
    private final IdGenerator idGenerator;

    @Inject
    public ItemMerger(
            ItemRepository itemRepository,
            PropertyRepository propertyRepository,
            MarkdownParser markdownParser,
            SearchService searchService,
            IdGenerator idGenerator) {
        this.itemRepository = itemRepository;
        this.propertyRepository = propertyRepository;
        this.markdownParser = markdownParser;
        this.searchService = searchService;
        this.idGenerator = idGenerator;
    }

    /**
     * Makes the page's items match the parsed ones.
     *
     * @param page        the page the items belong to
     * @param parsedItems the items in file order
     * @param now         the time to stamp on new and changed items
     */
    public MergeResult merge(Page page, List<ParsedItem> parsedItems, Instant now) {
        HMTID pageId = page.id();
        String pageRelPath = MarkdownPathMapper.toRelPath(page);
        String title = page.dirPath() + page.name();
        List<Item> existing = itemRepository.findByPageId(pageId);
        Map<HMTID, Item> byId = new HashMap<>();
        for (Item item : existing) byId.put(item.id(), item);

        // Resolve an id for every parsed item
        int n = parsedItems.size();
        List<HMTID> ids = new ArrayList<>(n);
        Set<HMTID> seen = new HashSet<>();
        boolean anyMissingId = false;
        for (ParsedItem pi : parsedItems) {
            HMTID id = PageMatcher.parseId(pi.id());
            if (id != null && !seen.contains(id) && !byId.containsKey(id) && itemRepository.findById(id).isPresent()) {
                // Belongs to another page (copy-pasted between files): don't steal it
                id = null;
            }
            if (id == null || seen.contains(id)) {
                id = idGenerator.generateItemId();
                anyMissingId = true;
            }
            seen.add(id);
            ids.add(id);
        }

        // Decide ordering. The idx written in the file is the writer's DB value and is taken as-is,
        // so every device ends up with the same idx (needed for direct DB sync later). The line
        // order in the file still wins: idx comments that disagree with it are recomputed.
        List<String> wantedIdx = new ArrayList<>(n);
        for (int i = 0; i < n; i++) {
            String fileIdx = parsedItems.get(i).idx();
            Item old = byId.get(ids.get(i));
            wantedIdx.add(fileIdx != null ? fileIdx : old != null ? old.idx() : null);
        }
        List<String> idxs;
        try {
            idxs = ItemOrderPlanner.plan(wantedIdx);
        } catch (IllegalStateException e) {
            LOG.warnv("Fractional index exhausted on page {0}, reindexing all items", pageId);
            idxs = ItemOrderPlanner.reindexAll(n);
        }
        boolean needsWriteBack = anyMissingId;
        for (int i = 0; i < n; i++) {
            if (!idxs.get(i).equals(parsedItems.get(i).idx())) needsWriteBack = true;
        }

        // Items no longer in the file go first, so their idx values are free for the survivors
        List<String> existingIds = existing.stream().map(i -> i.id().toString()).toList();
        Map<String, Map<String, String>> scalars = propertyRepository.findScalarByTargets(ITEM_TARGET, existingIds);
        boolean changed = false;
        List<String> removedIds = new ArrayList<>();
        for (Item old : existing) {
            if (seen.contains(old.id())) continue;
            itemRepository.delete(old.id());
            removedIds.add(old.id().toString());
            changed = true;
        }
        propertyRepository.deleteAllForTargets(ITEM_TARGET, removedIds);
        searchService.removeItems(removedIds); // one Lucene commit for all of them

        // (page_id, idx) is unique: if a surviving item is about to take another survivor's current idx,
        // park every survivor on a temporary idx first. Rare (two devices inserted at the same spot).
        Map<String, HMTID> idxOwner = new HashMap<>();
        for (Item old : existing) {
            if (seen.contains(old.id())) idxOwner.put(old.idx(), old.id());
        }
        boolean collision = false;
        for (int i = 0; i < n; i++) {
            HMTID owner = idxOwner.get(idxs.get(i));
            if (owner != null && !owner.equals(ids.get(i))) collision = true;
        }
        if (collision) {
            for (Item old : existing) {
                if (seen.contains(old.id())) itemRepository.updateIndex(old.id(), "\u007f" + old.idx());
            }
        }

        Map<String, String> defs = propertyRepository.findAllDefinitions();
        List<Item> toIndex = new ArrayList<>(); // indexed in one commit at the end

        for (int i = 0; i < n; i++) {
            ParsedItem pi = parsedItems.get(i);
            HMTID id = ids.get(i);
            String idx = idxs.get(i);
            Item old = byId.get(id);

            // The file holds relative Markdown links; the app stores wikilinks and hashtags
            String body = PageLinkConverter.toWikilinks(pi.body() == null ? "" : pi.body(), pageRelPath);
            HMTID transclude = PageMatcher.parseId(pi.transcludeItemId());
            if (transclude == null && old != null && old.transcludeItemId() != null && body.equals(old.body())) {
                transclude = old.transcludeItemId(); // file written before transclude comments existed
            }
            List<String> types = pi.types();

            boolean same = old != null
                    && old.body().equals(body)
                    && old.level() == pi.level()
                    && old.collapsed() == pi.collapsed()
                    && sameTypes(old.types(), types)
                    && Objects.equals(old.transcludeItemId(), transclude)
                    && old.idx().equals(idx)
                    && !collision;

            if (!same) {
                boolean bodyChanged = old == null || !old.body().equals(body);
                String ast = bodyChanged ? markdownParser.parseToAst(body) : old.ast();
                Item item = new Item(id, pageId, null, page.dirPath(), page.name(), page.type(),
                        old != null ? old.createdDate() : now,
                        bodyChanged || old == null ? now : old.modifiedDate(),
                        idx, pi.level(), body, ast, pi.collapsed(), types, transclude);
                itemRepository.save(item);
                if (bodyChanged) {
                    propertyRepository.syncTags(ITEM_TARGET, id.toString(), HashtagExtractor.extractHashtags(ast));
                }
                toIndex.add(item);
                changed = true;
            }

            if (syncScalarProperties(id.toString(), pi.properties(), scalars.getOrDefault(id.toString(), Map.of()), defs)) {
                changed = true;
            }
        }

        searchService.indexItems(toIndex, title);
        if (changed) {
            itemRepository.recalculateParentIds(pageId);
        }
        return new MergeResult(changed, needsWriteBack);
    }

    /**
     * Makes the scalar properties of an item match the key="value" pairs of its metadata line.
     * Reserved keys and list-typed keys are not properties here.
     *
     * @return true if any property was written or removed
     */
    private boolean syncScalarProperties(String itemId, Map<String, String> fileProperties,
                                         Map<String, String> have, Map<String, String> defs) {
        Map<String, String> want = new LinkedHashMap<>();
        for (var e : fileProperties.entrySet()) {
            if (RESERVED_KEYS.contains(e.getKey())) continue;
            String type = defs.getOrDefault(e.getKey(), "text");
            if ("list".equals(type) || "tags".equals(type)) continue;
            want.put(e.getKey(), e.getValue());
        }
        boolean changed = false;
        for (var e : want.entrySet()) {
            if (!e.getValue().equals(have.get(e.getKey()))) {
                String type = defs.getOrDefault(e.getKey(), "text");
                try {
                    propertyRepository.upsertScalar(ITEM_TARGET, itemId, e.getKey(), type, e.getValue());
                    changed = true;
                } catch (RuntimeException ex) {
                    LOG.warnv("Skipping property {0}={1} on item {2}: {3}", e.getKey(), e.getValue(), itemId, ex.getMessage());
                }
            }
        }
        for (String key : have.keySet()) {
            if (!want.containsKey(key)) {
                propertyRepository.deleteProperty(ITEM_TARGET, itemId, key);
                changed = true;
            }
        }
        return changed;
    }

    private static boolean sameTypes(List<String> a, List<String> b) {
        return new HashSet<>(a == null ? List.of() : a).equals(new HashSet<>(b == null ? List.of() : b));
    }
}
