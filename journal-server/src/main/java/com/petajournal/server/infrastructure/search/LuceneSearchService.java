package com.petajournal.server.infrastructure.search;

import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.SearchResult;
import com.petajournal.server.domain.repository.SearchService;
import io.quarkus.runtime.Shutdown;
import io.quarkus.runtime.Startup;
import jakarta.annotation.PostConstruct;
import jakarta.enterprise.context.ApplicationScoped;
import org.apache.lucene.analysis.Analyzer;
import org.apache.lucene.analysis.ja.JapaneseAnalyzer;
import org.apache.lucene.document.*;
import org.apache.lucene.index.*;
import org.apache.lucene.queryparser.classic.QueryParser;
import org.apache.lucene.search.*;
import org.apache.lucene.search.highlight.*;
import org.apache.lucene.store.FSDirectory;
import org.eclipse.microprofile.config.inject.ConfigProperty;
import org.jboss.logging.Logger;

import com.sosuisha.hmtid.HMTID;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

@ApplicationScoped
@Startup
public class LuceneSearchService implements SearchService {

    private static final Logger LOG = Logger.getLogger(LuceneSearchService.class);

    private static final String FIELD_PAGE_ID = "pageId";
    private static final String FIELD_PAGE_TITLE = "pageTitle";
    private static final String FIELD_ITEM_ID = "itemId";
    private static final String FIELD_BODY = "body";
    private static final String FIELD_TYPE = "type"; // "page" or "item"
    private static final String TYPE_PAGE = "page";
    private static final String TYPE_ITEM = "item";

    @ConfigProperty(name = "petajournal.index.dir")
    String indexDir;

    private FSDirectory directory;
    private Analyzer analyzer;
    private IndexWriter writer;

    @PostConstruct
    void init() {
        try {
            Path path = Path.of(resolveHome(indexDir));
            Files.createDirectories(path);
            directory = FSDirectory.open(path);
            analyzer = new JapaneseAnalyzer();
            IndexWriterConfig config = new IndexWriterConfig(analyzer);
            config.setOpenMode(IndexWriterConfig.OpenMode.CREATE_OR_APPEND);
            writer = new IndexWriter(directory, config);
            LOG.infov("Lucene index initialized at: {0}", path);
        } catch (IOException e) {
            throw new RuntimeException("Failed to initialize Lucene index", e);
        }
    }

    @Shutdown
    void shutdown() {
        try {
            if (writer != null) writer.close();
            if (directory != null) directory.close();
        } catch (IOException e) {
            LOG.warn("Error closing Lucene index", e);
        }
    }

    @Override
    public void indexPage(Page page) {
        try {
            // Delete existing page document (must use BooleanQuery for AND semantics)
            BooleanQuery deleteQuery = new BooleanQuery.Builder()
                    .add(new TermQuery(new Term(FIELD_PAGE_ID, page.id().toString())), BooleanClause.Occur.MUST)
                    .add(new TermQuery(new Term(FIELD_TYPE, TYPE_PAGE)), BooleanClause.Occur.MUST)
                    .build();
            writer.deleteDocuments(deleteQuery);

            Document doc = new Document();
            doc.add(new StringField(FIELD_TYPE, TYPE_PAGE, Field.Store.YES));
            doc.add(new StringField(FIELD_PAGE_ID, page.id().toString(), Field.Store.YES));
            String title = page.dirPath() + page.name();
            doc.add(new TextField(FIELD_PAGE_TITLE, title, Field.Store.YES));
            doc.add(new StringField(FIELD_ITEM_ID, "", Field.Store.YES));
            writer.addDocument(doc);
            writer.commit();
        } catch (IOException e) {
            LOG.warnv("Failed to index page: {0}", page.id(), e);
        }
    }

    @Override
    public void indexItem(Item item, String pageTitle) {
        try {
            replaceItemDocument(item, pageTitle);
            writer.commit();
        } catch (IOException e) {
            LOG.warnv("Failed to index item: {0}", item.id(), e);
        }
    }

    @Override
    public void indexItems(List<Item> items, String pageTitle) {
        if (items.isEmpty()) return;
        try {
            for (Item item : items) {
                replaceItemDocument(item, pageTitle);
            }
        } catch (IOException e) {
            LOG.warnv("Failed to index {0} items", items.size(), e);
        }
    }

    /** Removes the item's current document and adds a fresh one; the caller commits. */
    private void replaceItemDocument(Item item, String pageTitle) throws IOException {
        writer.deleteDocuments(new Term(FIELD_ITEM_ID, item.id().toString()));
        Document doc = new Document();
        doc.add(new StringField(FIELD_TYPE, TYPE_ITEM, Field.Store.YES));
        doc.add(new StringField(FIELD_PAGE_ID, item.pageId().toString(), Field.Store.YES));
        doc.add(new TextField(FIELD_PAGE_TITLE, pageTitle, Field.Store.YES));
        doc.add(new StringField(FIELD_ITEM_ID, item.id().toString(), Field.Store.YES));
        doc.add(new TextField(FIELD_BODY, item.body(), Field.Store.YES));
        writer.addDocument(doc);
    }

    @Override
    public void removeItems(List<String> itemIds) {
        if (itemIds.isEmpty()) return;
        try {
            for (String id : itemIds) {
                writer.deleteDocuments(new Term(FIELD_ITEM_ID, id));
            }
        } catch (IOException e) {
            LOG.warnv("Failed to remove {0} items from index", itemIds.size(), e);
        }
    }

    @Override
    public void flush() {
        try {
            long t0 = System.currentTimeMillis();
            writer.commit();
            long ms = System.currentTimeMillis() - t0;
            if (ms > 2000) {
                LOG.warnv("Lucene commit took {0} ms (index dir: {1}); check antivirus or sync tools on that folder",
                        ms, indexDir);
            }
        } catch (IOException e) {
            LOG.warn("Failed to commit search index", e);
        }
    }

    @Override
    public void removePage(String pageId) {
        try {
            writer.deleteDocuments(new Term(FIELD_PAGE_ID, pageId));
            writer.commit();
        } catch (IOException e) {
            LOG.warnv("Failed to remove page from index: {0}", pageId, e);
        }
    }

    @Override
    public void removeItem(String itemId) {
        try {
            writer.deleteDocuments(new Term(FIELD_ITEM_ID, itemId));
            writer.commit();
        } catch (IOException e) {
            LOG.warnv("Failed to remove item from index: {0}", itemId, e);
        }
    }

    @Override
    public List<SearchResult> searchPageNames(String query, int maxResults) {
        if (maxResults <= 0) maxResults = 20;
        try (DirectoryReader reader = DirectoryReader.open(writer)) {
            // Try prefix search first (query*)
            List<SearchResult> results = searchPageNamesByPattern(
                    QueryParser.escape(query) + "*", reader, maxResults, false);
            if (!results.isEmpty()) {
                return results;
            }
            // Fallback to contains search (*query*)
            return searchPageNamesByPattern(
                    "*" + QueryParser.escape(query) + "*", reader, maxResults, true);
        } catch (Exception e) {
            LOG.warnv("Page name search failed for query: {0}", query, e);
        }
        return List.of();
    }

    private List<SearchResult> searchPageNamesByPattern(String pattern, DirectoryReader reader,
                                                        int maxResults, boolean allowLeadingWildcard) {
        List<SearchResult> results = new ArrayList<>();
        try {
            IndexSearcher searcher = new IndexSearcher(reader);
            QueryParser parser = new QueryParser(FIELD_PAGE_TITLE, analyzer);
            parser.setDefaultOperator(QueryParser.Operator.AND);
            parser.setAllowLeadingWildcard(allowLeadingWildcard);

            BooleanQuery.Builder bq = new BooleanQuery.Builder();
            bq.add(parser.parse(pattern), BooleanClause.Occur.MUST);
            bq.add(new TermQuery(new Term(FIELD_TYPE, TYPE_PAGE)), BooleanClause.Occur.MUST);

            TopDocs topDocs = searcher.search(bq.build(), maxResults);
            for (ScoreDoc sd : topDocs.scoreDocs) {
                Document doc = searcher.storedFields().document(sd.doc);
                results.add(new SearchResult(
                        HMTID.of(doc.get(FIELD_PAGE_ID)),
                        doc.get(FIELD_PAGE_TITLE),
                        "",
                        doc.get(FIELD_PAGE_TITLE),
                        sd.score
                ));
            }
        } catch (Exception e) {
            LOG.warnv("Page name search by pattern failed: {0}", pattern, e);
        }
        return results;
    }

    @Override
    public List<SearchResult> searchFullText(String query, int maxResults) {
        if (maxResults <= 0) maxResults = 20;
        List<SearchResult> results = new ArrayList<>();
        try (DirectoryReader reader = DirectoryReader.open(writer)) {
            IndexSearcher searcher = new IndexSearcher(reader);
            QueryParser parser = new QueryParser(FIELD_BODY, analyzer);
            parser.setDefaultOperator(QueryParser.Operator.AND);
            org.apache.lucene.search.Query luceneQuery = parser.parse(query);

            TopDocs topDocs = searcher.search(luceneQuery, maxResults);

            // Highlighter
            Highlighter highlighter = new Highlighter(
                    new SimpleHTMLFormatter("<mark>", "</mark>"),
                    new QueryScorer(luceneQuery));
            highlighter.setTextFragmenter(new SimpleFragmenter(100));

            for (ScoreDoc sd : topDocs.scoreDocs) {
                Document doc = searcher.storedFields().document(sd.doc);
                String body = doc.get(FIELD_BODY);
                String snippet = body;
                if (body != null) {
                    try {
                        String highlighted = highlighter.getBestFragment(analyzer, FIELD_BODY, body);
                        if (highlighted != null) snippet = highlighted;
                    } catch (Exception ignored) {}
                }
                results.add(new SearchResult(
                        HMTID.of(doc.get(FIELD_PAGE_ID)),
                        doc.get(FIELD_PAGE_TITLE),
                        doc.get(FIELD_ITEM_ID),
                        snippet,
                        sd.score
                ));
            }
        } catch (Exception e) {
            LOG.warnv("Full text search failed for query: {0}", query, e);
        }
        return results;
    }

    @Override
    public boolean isEmpty() {
        try (DirectoryReader reader = DirectoryReader.open(writer)) {
            return reader.numDocs() == 0;
        } catch (IOException e) {
            return true;
        }
    }

    @Override
    public void deleteAll() {
        try {
            writer.deleteAll();
            writer.commit();
            LOG.info("Lucene index cleared");
        } catch (IOException e) {
            LOG.warn("Failed to clear Lucene index", e);
        }
    }

    private String resolveHome(String path) {
        if (path.startsWith("~")) {
            return System.getProperty("user.home") + path.substring(1);
        }
        return path;
    }
}
