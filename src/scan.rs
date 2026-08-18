use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};

use jwalk::WalkDir;

use crate::app::{Entry, ScanCommand, ScanMessage};

/// Recursively sum the size of every file under `path`.
///
/// `jwalk` parallelizes the directory walk internally, which is what makes
/// this fast enough to run per-entry without freezing the UI thread for too
/// long on a big folder. We still do this off the UI thread (see `run`).
fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .skip_hidden(false)
        .into_iter()
        // Silently skip entries we can't read (permission-denied, broken
        // symlinks, etc) rather than aborting the whole size calculation.
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| entry.metadata().ok())
        .map(|meta| meta.len())
        .sum()
}

/// List the immediate children of `path` and compute a size for each:
/// - files: their own byte length
/// - directories: the recursive total via `dir_size`
///
/// This is the "lazy" approach (like ncdu's live mode) rather than
/// pre-scanning the whole drive up front, so opening the tool is instant
/// and you only pay the cost for directories you actually drill into.
fn list_dir(path: &Path) -> std::io::Result<Vec<Entry>> {
    let mut entries = Vec::new();

    for dir_entry in std::fs::read_dir(path)? {
        let dir_entry = match dir_entry {
            Ok(e) => e,
            // A single unreadable entry shouldn't kill the whole listing.
            Err(_) => continue,
        };

        let entry_path = dir_entry.path();
        let name = dir_entry.file_name().to_string_lossy().into_owned();

        let metadata = match dir_entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let is_dir = metadata.is_dir();
        let size = if is_dir {
            dir_size(&entry_path)
        } else {
            metadata.len()
        };

        entries.push(Entry {
            name,
            path: entry_path,
            size,
            is_dir,
        });
    }

    Ok(entries)
}

/// Background worker loop: blocks on `cmd_rx` for scan requests, does the
/// (potentially slow) filesystem work, and reports back over `msg_tx`.
/// Runs on its own thread so the UI thread never blocks on I/O.
pub fn run(cmd_rx: Receiver<ScanCommand>, msg_tx: Sender<ScanMessage>) {
    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            ScanCommand::Scan(path) => match list_dir(&path) {
                Ok(entries) => {
                    let _ = msg_tx.send(ScanMessage::DirReady { path, entries });
                }
                Err(e) => {
                    let _ = msg_tx.send(ScanMessage::Error {
                        path,
                        message: e.to_string(),
                    });
                }
            },
            ScanCommand::Quit => break,
        }
    }
}

/// Convenience wrapper so call sites in `main.rs` don't need to know the
/// message type used for requests.
pub fn request_scan(cmd_tx: &Sender<ScanCommand>, path: PathBuf) {
    let _ = cmd_tx.send(ScanCommand::Scan(path));
}
