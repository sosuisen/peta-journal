package com.petajournal.server.infrastructure.launcher;

import jakarta.enterprise.context.ApplicationScoped;
import org.jboss.logging.Logger;

import java.awt.Desktop;
import java.net.URI;

@ApplicationScoped
public class AppLauncher {

    private static final Logger LOG = Logger.getLogger(AppLauncher.class);

    /**
     * Launch app with custom URL scheme
     * @param url URL scheme string (e.g., "peta-journal://page/123")
     * @return true if successful, false otherwise
     */
    public boolean launch(String url) {
        try {
            if (Desktop.isDesktopSupported() && Desktop.getDesktop().isSupported(Desktop.Action.BROWSE)) {
                Desktop.getDesktop().browse(new URI(url));
                LOG.infov("Launched app with URL: {0}", url);
                return true;
            } else {
                // Fallback to OS command
                return launchViaCommand(url);
            }
        } catch (Exception e) {
            LOG.errorv("Failed to launch app with URL {0}: {1}", url, e.getMessage());
            return false;
        }
    }

    private boolean launchViaCommand(String url) {
        try {
            String os = System.getProperty("os.name").toLowerCase();

            ProcessBuilder pb;
            if (os.contains("win")) {
                pb = new ProcessBuilder("cmd", "/c", "start", "", url);
            } else if (os.contains("mac")) {
                pb = new ProcessBuilder("open", url);
            } else if (os.contains("nix") || os.contains("nux")) {
                pb = new ProcessBuilder("xdg-open", url);
            } else {
                LOG.warnv("Unsupported OS: {0}", os);
                return false;
            }

            pb.start();
            LOG.infov("Launched app via command: {0}", url);
            return true;
        } catch (Exception e) {
            LOG.errorv("Failed to launch via command: {0}", e.getMessage());
            return false;
        }
    }
}
