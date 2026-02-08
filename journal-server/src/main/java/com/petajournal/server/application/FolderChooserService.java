package com.petajournal.server.application;

import jakarta.enterprise.context.ApplicationScoped;
import org.jboss.logging.Logger;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;

@ApplicationScoped
public class FolderChooserService {

    private static final Logger LOG = Logger.getLogger(FolderChooserService.class);

    public static class FolderChoice {
        public final boolean selected;
        public final String path;

        public FolderChoice(boolean selected, String path) {
            this.selected = selected;
            this.path = path;
        }
    }

    public FolderChoice chooseFolder() {
        String os = System.getProperty("os.name").toLowerCase();

        if (os.contains("win")) {
            return chooseFolderWindows();
        } else if (os.contains("mac")) {
            return chooseFolderMacOS();
        } else {
            return chooseFolderLinux();
        }
    }

    private FolderChoice chooseFolderWindows() {
        try {
            String script =
                "Add-Type -AssemblyName System.Windows.Forms; " +
                "$dialog = New-Object System.Windows.Forms.FolderBrowserDialog; " +
                "$dialog.Description = 'Select Markdown Directory'; " +
                "$dialog.ShowNewFolderButton = $false; " +
                "$result = $dialog.ShowDialog(); " +
                "if ($result -eq [System.Windows.Forms.DialogResult]::OK) { " +
                "    Write-Output $dialog.SelectedPath " +
                "}";

            ProcessBuilder pb = new ProcessBuilder(
                "powershell.exe", "-NoProfile", "-NonInteractive", "-Command", script
            );
            return executeProcess(pb);
        } catch (Exception e) {
            LOG.error("Failed to show folder chooser on Windows", e);
            return new FolderChoice(false, "");
        }
    }

    private FolderChoice chooseFolderMacOS() {
        try {
            String script =
                "tell application \"Finder\"\n" +
                "    activate\n" +
                "    set folderPath to choose folder with prompt \"Select Markdown Directory\"\n" +
                "    return POSIX path of folderPath\n" +
                "end tell";

            ProcessBuilder pb = new ProcessBuilder("osascript", "-e", script);
            return executeProcess(pb);
        } catch (Exception e) {
            LOG.error("Failed to show folder chooser on macOS", e);
            return new FolderChoice(false, "");
        }
    }

    private FolderChoice chooseFolderLinux() {
        try {
            // Try zenity first
            ProcessBuilder pb = new ProcessBuilder(
                "zenity", "--file-selection", "--directory",
                "--title=Select Markdown Directory"
            );
            FolderChoice choice = executeProcess(pb);
            if (choice.selected) {
                return choice;
            }

            // Fallback to kdialog
            pb = new ProcessBuilder(
                "kdialog", "--getexistingdirectory", System.getProperty("user.home"),
                "--title", "Select Markdown Directory"
            );
            return executeProcess(pb);
        } catch (Exception e) {
            LOG.error("Failed to show folder chooser on Linux", e);
            return new FolderChoice(false, "");
        }
    }

    private FolderChoice executeProcess(ProcessBuilder pb) throws Exception {
        Process process = pb.start();

        BufferedReader reader = new BufferedReader(
            new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8)
        );

        StringBuilder output = new StringBuilder();
        String line;
        while ((line = reader.readLine()) != null) {
            if (!line.trim().isEmpty()) {
                output.append(line.trim());
            }
        }

        int exitCode = process.waitFor();
        String selectedPath = output.toString();

        if (exitCode == 0 && !selectedPath.isEmpty()) {
            LOG.infov("Folder selected: {0}", selectedPath);
            return new FolderChoice(true, selectedPath);
        } else {
            LOG.info("Folder selection cancelled or failed");
            return new FolderChoice(false, "");
        }
    }
}
