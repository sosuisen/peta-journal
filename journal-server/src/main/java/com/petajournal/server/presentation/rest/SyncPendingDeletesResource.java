package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.PageService;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.infrastructure.export.FileSyncState;
import com.petajournal.server.infrastructure.export.FileSyncStateRepository;
import com.petajournal.server.infrastructure.export.MarkdownExportService;
import com.sosuisha.hmtid.HMTID;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import org.jboss.logging.Logger;

import java.time.Instant;
import java.time.ZoneId;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

/**
 * Plain HTML page listing pages whose markdown file disappeared from the folder.
 * The user ticks the ones that should really be deleted and submits; nothing is
 * deleted otherwise. Served by the journal server itself so it works without the
 * Angular frontend.
 */
@Path("/api/sync/pending-deletes")
public class SyncPendingDeletesResource {

    private static final Logger LOG = Logger.getLogger(SyncPendingDeletesResource.class);
    private static final DateTimeFormatter TS = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm");

    @Inject FileSyncStateRepository fileSyncStateRepository;
    @Inject PageRepository pageRepository;
    @Inject ItemRepository itemRepository;
    @Inject PageService pageService;
    @Inject MarkdownExportService exportService;

    record Row(FileSyncState state, Page page, int itemCount) {}

    @GET
    @Produces(MediaType.TEXT_HTML + ";charset=UTF-8")
    public String page() {
        return render(loadRows(), List.of());
    }

    @POST
    @Consumes(MediaType.APPLICATION_FORM_URLENCODED)
    @Produces(MediaType.TEXT_HTML + ";charset=UTF-8")
    public Response submit(@HeaderParam("Origin") String origin,
                           @HeaderParam("Referer") String referer,
                           @FormParam("action") String action,
                           @FormParam("pageId") List<String> pageIds) {
        // Only accept submissions from this page itself, not from other sites
        if (!isLocal(origin) && !isLocal(referer)) {
            return Response.status(Response.Status.FORBIDDEN).entity("Forbidden").build();
        }

        List<String> messages = new ArrayList<>();
        boolean restore = "restore".equals(action);
        for (String idStr : pageIds == null ? List.<String>of() : pageIds) {
            HMTID id;
            try {
                id = HMTID.of(idStr);
            } catch (IllegalArgumentException e) {
                continue;
            }
            Optional<Page> page = pageRepository.findById(id);
            String label = page.map(p -> p.type() + "s/" + p.dirPath() + p.name() + ".md").orElse(idStr);
            try {
                if (restore) {
                    exportService.queueExport(id);
                    messages.add("Restored file: " + label);
                } else {
                    pageService.deletePage(id);
                    messages.add("Deleted: " + label);
                }
            } catch (IllegalArgumentException e) {
                messages.add("Skipped " + label + ": " + e.getMessage());
            } catch (RuntimeException e) {
                LOG.warnv("Failed to {0} page {1}: {2}", restore ? "restore" : "delete", id, e.getMessage());
                messages.add("Failed " + label + ": " + e.getMessage());
            }
        }
        return Response.ok(render(loadRows(), messages)).build();
    }

    private List<Row> loadRows() {
        List<Row> rows = new ArrayList<>();
        for (FileSyncState state : fileSyncStateRepository.findMissing()) {
            Optional<Page> page = pageRepository.findById(state.pageId());
            if (page.isEmpty()) {
                fileSyncStateRepository.deleteByPath(state.relPath()); // page already gone
                continue;
            }
            Page p = page.get();
            // These can't be deleted anyway; the "restore" action still works for them
            int items = itemRepository.findByPageId(p.id()).size();
            rows.add(new Row(state, p, items));
        }
        return rows;
    }

    private String render(List<Row> rows, List<String> messages) {
        StringBuilder sb = new StringBuilder();
        sb.append("""
            <!doctype html>
            <html lang="en">
            <head>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <title>PetaJournal - Files disappeared</title>
            <style>
              body { font-family: system-ui, sans-serif; max-width: 860px; margin: 2rem auto; padding: 0 1rem; color: #222; }
              h1 { font-size: 1.4rem; }
              .note { background: #fff8e1; border: 1px solid #f0d58c; padding: .75rem 1rem; border-radius: 6px; }
              table { border-collapse: collapse; width: 100%; margin: 1rem 0; }
              th, td { text-align: left; padding: .4rem .6rem; border-bottom: 1px solid #ddd; vertical-align: top; }
              th { background: #f5f5f5; }
              code { font-family: ui-monospace, monospace; }
              .actions button { padding: .5rem 1rem; margin-right: .5rem; }
              .danger { background: #c62828; color: #fff; border: 0; border-radius: 4px; }
              .msg { background: #e8f5e9; border: 1px solid #a5d6a7; padding: .5rem 1rem; border-radius: 6px; margin-bottom: 1rem; }
              .muted { color: #666; }
            </style>
            </head>
            <body>
            <h1>Markdown files disappeared</h1>
            <p class="note">
              The files below were removed from the markdown folder (or are not visible right now).
              Dropbox <b>selective sync</b>, folders that have not finished syncing, and
              <b>online-only</b> files show up here too. Nothing is deleted until you tick a file and press
              <b>Delete selected</b>. Untick anything you are not sure about.
            </p>
            """);

        for (String m : messages) {
            sb.append("<div class=\"msg\">").append(escape(m)).append("</div>\n");
        }

        if (rows.isEmpty()) {
            sb.append("<p>No pending files. You can close this tab.</p>\n</body></html>");
            return sb.toString();
        }

        sb.append("""
            <form method="post">
            <table>
            <thead><tr><th></th><th>File</th><th>Items</th><th>Missing since</th></tr></thead>
            <tbody>
            """);
        for (Row row : rows) {
            String since = row.state().missingSince() == null ? "" :
                    TS.format(Instant.ofEpochMilli(row.state().missingSince()).atZone(ZoneId.systemDefault()));
            sb.append("<tr><td><input type=\"checkbox\" name=\"pageId\" value=\"")
              .append(escape(row.page().id().toString())).append("\"></td>")
              .append("<td><code>").append(escape(row.state().relPath())).append("</code></td>")
              .append("<td>").append(row.itemCount()).append("</td>")
              .append("<td class=\"muted\">").append(escape(since)).append("</td></tr>\n");
        }
        sb.append("""
            </tbody>
            </table>
            <p class="actions">
              <button type="submit" name="action" value="delete" class="danger"
                      onclick="return confirm('Delete the selected pages from the database? This cannot be undone.')">Delete selected</button>
              <button type="submit" name="action" value="restore">Write selected back to the folder</button>
            </p>
            <p class="muted">Pages you leave unticked stay in the database and will be listed again next time.</p>
            </form>
            </body></html>
            """);
        return sb.toString();
    }

    private static boolean isLocal(String url) {
        if (url == null) return false;
        return url.startsWith("http://localhost:") || url.startsWith("http://127.0.0.1:")
                || url.equals("http://localhost") || url.equals("http://127.0.0.1");
    }

    private static String escape(String s) {
        if (s == null) return "";
        return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("\"", "&quot;");
    }
}
