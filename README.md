# herdr-quicknotes

Fast, modal popup notes manager for [Herdr](https://herdr.dev), the terminal multiplexer for coding agents.

![Rust](https://img.shields.io/badge/rust-edition%202024-orange.svg)
![Herdr](https://img.shields.io/badge/herdr-%3E%3D%200.7.4-8a2be2)
![Platforms](https://img.shields.io/badge/platforms-linux%20%7C%20macos-informational)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)

---

## Features

- **Modal Popup (`placement = "popup"`):** Opens in a centered, focused popup over your current Herdr workspace via a single keybinding (`prefix+n`).
- **Instant Search (`/`):** Real-time fuzzy matching across titles and note contents.
- **1-Key Clipboard Copy (`y`):** Copies the selected note directly to your system clipboard (Wayland / X11 / OSC 52).
- **External Editor (`e`):** Edits the note using `$EDITOR` or `$VISUAL`.
- **Delete (`d` / `x`):** Safe modal deletion with confirmation prompt.
- **AI Agent Interoperability (CLI Mode):** Dispatched coding agents (Antigravity, Claude Code, Codex) can read, add, and query notes headless via CLI.
- **Local-First & Syncable:** Notes are stored under `~/.config/herdr/quicknotes/*.json`.

---

## Installation

### From Herdr Plugin Marketplace (Once published)
```bash
herdr plugin install QuantumEdu/herdr-quicknotes
```

### Local Development / Build from source
```bash
git clone https://github.com/QuantumEdu/herdr-quicknotes.git ~/dev/herdr-quicknotes
cd ~/dev/herdr-quicknotes
cargo build --release
herdr plugin link .
```

---

## Keybinding Setup

Add the following to your `~/.config/herdr/config.toml`:

```toml
[[keys.command]]
key = "prefix+n"
type = "plugin_action"
command = "herdr-quicknotes.open"
description = "Abrir Quick Notes Popup"
```

Reload Herdr config:
```bash
herdr server reload-config
```

Now press `Ctrl+B n` (or your Herdr prefix + `n`) to toggle the notes popup from anywhere.

---

## Keyboard Shortcuts (Inside TUI)

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down the list |
| `k` / `↑` | Move up the list |
| `/` | Live fuzzy search |
| `n` | Create new note (Title -> Content modal) |
| `y` | Copy note to clipboard |
| `e` | Open note in `$EDITOR` |
| `d` / `x` | Delete selected note |
| `q` / `Esc` | Close popup |

---

## CLI Usage (For humans & AI Agents)

```bash
# Add a note
herdr-quicknotes add --title "Architecture Decision" --content "Use Rust + Ratatui"

# List notes (table or json)
herdr-quicknotes list
herdr-quicknotes list --json

# Read a specific note
herdr-quicknotes get <id>

# Copy note to clipboard
herdr-quicknotes copy <id>

# Delete note
herdr-quicknotes delete <id>
```

---

## License

MIT © QuantumEdu
