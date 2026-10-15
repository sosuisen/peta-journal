package com.petajournal.server.presentation.rest;

import com.petajournal.server.infrastructure.export.FileSyncStateRepository;
import com.petajournal.server.infrastructure.export.MarkdownExportService;
import com.petajournal.server.infrastructure.export.MarkdownReconcileService;
import jakarta.inject.Inject;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import org.eclipse.microprofile.config.inject.ConfigProperty;

/** Folder ↔ DB sync: status and a manual scan, mainly for checking that the sync is working. */
@Path("/api/sync")
@Produces(MediaType.APPLICATION_JSON)
public class SyncResource {

    private final boolean importEnabled;
    private final MarkdownExportService exportService;
    private final MarkdownReconcileService reconcileService;
    private final FileSyncStateRepository fileSyncStateRepository;

    @Inject
    public SyncResource(
            @ConfigProperty(name = "petajournal.import.enabled", defaultValue = "true") boolean importEnabled,
            MarkdownExportService exportService,
            MarkdownReconcileService reconcileService,
            FileSyncStateRepository fileSyncStateRepository) {
        this.importEnabled = importEnabled;
        this.exportService = exportService;
        this.reconcileService = reconcileService;
        this.fileSyncStateRepository = fileSyncStateRepository;
    }

    record StatusResponse(boolean importEnabled, String markdownRoot, boolean rootAvailable,
                          int trackedFiles, int missingFiles) {}

    record ScanResponse(int imported, int conflicts, int newMissing, int exported) {}

    @GET
    @Path("/status")
    public StatusResponse status() {
        var root = exportService.markdownRoot();
        return new StatusResponse(
                importEnabled,
                root.map(java.nio.file.Path::toString).orElse(""),
                root.isPresent(),
                fileSyncStateRepository.findAll().size(),
                fileSyncStateRepository.findMissing().size());
    }

    /** Runs a full folder scan now and waits for it. Returns zeros if a scan is already running. */
    @POST
    @Path("/scan")
    public ScanResponse scan() {
        var s = reconcileService.scan();
        return new ScanResponse(s.imported(), s.conflicts(), s.newMissing(), s.exported());
    }
}
