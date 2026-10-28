//! Opens a native folder selection dialog with the tools of the current OS.

use std::process::Command;

/// Result of a folder selection dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderChoice {
    /// True when the user chose a folder.
    pub selected: bool,
    /// The chosen folder, or an empty string.
    pub path: String,
}

impl FolderChoice {
    fn none() -> FolderChoice {
        FolderChoice {
            selected: false,
            path: String::new(),
        }
    }
}

/// Opens a native folder selection dialog appropriate for the current OS.
pub fn choose_folder() -> FolderChoice {
    if cfg!(target_os = "windows") {
        choose_folder_windows()
    } else if cfg!(target_os = "macos") {
        choose_folder_macos()
    } else {
        choose_folder_linux()
    }
}

fn choose_folder_windows() -> FolderChoice {
    let script = "Add-Type -AssemblyName System.Windows.Forms; \
        $dialog = New-Object System.Windows.Forms.FolderBrowserDialog; \
        $dialog.Description = 'Select Markdown Directory'; \
        $dialog.ShowNewFolderButton = $false; \
        $result = $dialog.ShowDialog(); \
        if ($result -eq [System.Windows.Forms.DialogResult]::OK) { \
            Write-Output $dialog.SelectedPath \
        }";
    let mut cmd = Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    execute(cmd, "Windows")
}

fn choose_folder_macos() -> FolderChoice {
    let script = "tell application \"Finder\"\n    activate\n    set folderPath to choose folder with prompt \"Select Markdown Directory\"\n    return POSIX path of folderPath\nend tell";
    let mut cmd = Command::new("osascript");
    cmd.args(["-e", script]);
    execute(cmd, "macOS")
}

fn choose_folder_linux() -> FolderChoice {
    // Try zenity first
    let mut cmd = Command::new("zenity");
    cmd.args([
        "--file-selection",
        "--directory",
        "--title=Select Markdown Directory",
    ]);
    let choice = execute(cmd, "Linux");
    if choice.selected {
        return choice;
    }
    // Fallback to kdialog
    let home = crate::config::home_dir();
    let mut cmd = Command::new("kdialog");
    cmd.arg("--getexistingdirectory")
        .arg(home)
        .args(["--title", "Select Markdown Directory"]);
    execute(cmd, "Linux")
}

fn execute(mut cmd: Command, os: &str) -> FolderChoice {
    match cmd.output() {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout);
            let selected: String = text
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join("");
            if output.status.success() && !selected.is_empty() {
                tracing::info!("Folder selected: {selected}");
                FolderChoice {
                    selected: true,
                    path: selected,
                }
            } else {
                tracing::info!("Folder selection cancelled or failed");
                FolderChoice::none()
            }
        }
        Err(e) => {
            tracing::error!("Failed to show folder chooser on {os}: {e}");
            FolderChoice::none()
        }
    }
}
