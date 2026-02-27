package com.petajournal.server.infrastructure.search;

import com.petajournal.server.domain.model.Block;
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

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

@ApplicationScoped
@Startup
public class LuceneSearchService implements SearchService {

    private static final Logger LOG = Logger.getLogger(LuceneSearchService.class);

    private static final String FIELD_PAGE_ID = "pageId";
    private static final String FIELD_PAGE_TITLE = "pageTitle";
    private static final String FIELD_BLOCK_ID = "blockId";
    private static final String FIELD_BODY = "body";
    private static final String FIELD_TYPE = "type"; // "page" or "block"

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
                    .add(new TermQuery(new Term(FIELD_TYPE, "page")), BooleanClause.Occur.MUST)
                    .build();
            writer.deleteDocuments(deleteQuery);

            Document doc = new Document();
            doc.add(new StringField(FIELD_TYPE, "page", Field.Store.YES));
            doc.add(new StringField(FIELD_PAGE_ID, page.id().toString(), Field.Store.YES));
            String title = page.dirPath() + page.name();
            doc.add(new TextField(FIELD_PAGE_TITLE, title, Field.Store.YES));
            doc.add(new StringField(FIELD_BLOCK_ID, "", Field.Store.YES));
            writer.addDocument(doc);
            writer.commit();
        } catch (IOException e) {
            LOG.warnv("Failed to index page: {0}", page.id(), e);
        }
    }

    @Override
    public void indexBlock(Block block, String pageTitle) {
        try {
            writer.deleteDocuments(new Term(FIELD_BLOCK_ID, block.id().toString()));

            Document doc = new Document();
            doc.add(new StringField(FIELD_TYPE, "block", Field.Store.YES));
            doc.add(new StringField(FIELD_PAGE_ID, block.pageId().toString(), Field.Store.YES));
            doc.add(new TextField(FIELD_PAGE_TITLE, pageTitle, Field.Store.YES));
            doc.add(new StringField(FIELD_BLOCK_ID, block.id().toString(), Field.Store.YES));
            doc.add(new TextField(FIELD_BODY, block.body(), Field.Store.YES));
            writer.addDocument(doc);
            writer.commit();
        } catch (IOException e) {
            LOG.warnv("Failed to index block: {0}", block.id(), e);
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
    public void removeBlock(String blockId) {
        try {
            writer.deleteDocuments(new Term(FIELD_BLOCK_ID, blockId));
            writer.commit();
        } catch (IOException e) {
            LOG.warnv("Failed to remove block from index: {0}", blockId, e);
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
            bq.add(new TermQuery(new Term(FIELD_TYPE, "page")), BooleanClause.Occur.MUST);

            TopDocs topDocs = searcher.search(bq.build(), maxResults);
            for (ScoreDoc sd : topDocs.scoreDocs) {
                Document doc = searcher.storedFields().document(sd.doc);
                results.add(new SearchResult(
                        UUID.fromString(doc.get(FIELD_PAGE_ID)),
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
                        UUID.fromString(doc.get(FIELD_PAGE_ID)),
                        doc.get(FIELD_PAGE_TITLE),
                        doc.get(FIELD_BLOCK_ID),
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
