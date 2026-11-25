# SpaceCheck

SpaceCheck is a fast terminal disk usage analyzer written in Rust. It scans a directory, recursively calculates folder sizes, sorts entries from largest to smallest, and lets you navigate the filesystem without leaving the terminal.

## Features

- Recursively calculates file and directory sizes
- Displays sizes using standard binary units (`KiB`, `MiB`, `GiB`, `TiB`, and `PiB`)
- Sorts entries with the largest space users first
- Includes a relative size bar for each entry
- Runs filesystem scans on a background thread
- Opens directories directly in the TUI
- Moves selected files or directories to the operating system trash/recycle bin
- Handles unreadable entries without stopping the entire scan

## Controls

| Key | Action |
| --- | --- |
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `Enter` | Open the selected directory |
| `Backspace` | Go to the parent directory |
| `d` | Ask to move the selected entry to trash/recycle bin |
| `y` | Confirm deletion |
| Any other key | Cancel deletion |
| `q` / `Esc` | Quit |

> `d` does not permanently delete data. SpaceCheck uses the [`trash`](https://crates.io/crates/trash) crate to move entries to the system recycle bin or trash.

## Requirements

- Rust and Cargo
- A terminal that supports a TUI and raw mode
- Platform support for your operating system's trash/recycle bin

## Installation

```bash
git clone https://github.com/NyxieLucy/SpaceCheck.git
cd SpaceCheck
cargo build --release
```

The compiled executable is created at:

- Linux/macOS: `target/release/disk-tui`
- Windows: `target\\release\\disk-tui.exe`

## Usage

Scan the current directory:

```bash
cargo run --release
```

Scan a specific directory:

```bash
cargo run --release -- /path/to/directory
```

Example on Windows:

```powershell
cargo run --release -- C:\Users\YourName
```

## How scanning works

SpaceCheck starts a background worker thread and sends it directory scan requests. Each immediate file or directory is measured; directories are totaled recursively. Scan results are sent back to the UI thread, keeping navigation responsive while larger directories are being calculated.

## Dependencies

- [Ratatui](https://crates.io/crates/ratatui) for the terminal interface
- [Crossterm](https://crates.io/crates/crossterm) for terminal and keyboard handling
- [jwalk](https://crates.io/crates/jwalk) for parallel directory traversal
- [trash](https://crates.io/crates/trash) for recycle-bin/trash support
- [Anyhow](https://crates.io/crates/anyhow) for error handling

## License

License information has not yet been added. Please ask the repository owner before reusing the project in a different project.
