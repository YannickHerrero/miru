# miru (見る)

A sleek, terminal-native streaming CLI for movies and TV shows. Connects TMDB, Torrentio, and MPV into a seamless viewing experience.

## Demo

https://github.com/user-attachments/assets/0fc5c941-bcf2-4b0a-bdbf-791b87337e22

## Features

- **TMDB search**: Search movies, TV shows, and anime via TMDB
- **Fast**: Sub-second startup, minimal keystrokes from launch to playback
- **Beautiful**: Rich terminal UI with smooth animations and Catppuccin-inspired colors that adapt to your terminal's light/dark theme
- **Smart flow**: Automatically skips episode selection for movies, shows season selection for TV shows
- **Reliable**: Graceful error handling with clear feedback
- **Flexible streaming**: Works with Real-Debrid for instant cached playback, or direct P2P streaming without any account
- **Customizable**: Full theme customization with support for custom colors

## Installation

### Requirements

- **Video player**: [mpv](https://mpv.io/) (recommended) or VLC
- **TMDB API key** (required): Get one at https://www.themoviedb.org/settings/api
- **Real-Debrid API key** (optional): For faster cached playback — https://real-debrid.com/apitoken

### Build from Source

Miru requires the [Rust toolchain](https://rustup.rs/) and a C compiler.

#### Linux (Debian/Ubuntu)

```bash
# Install dependencies
sudo apt update
sudo apt install build-essential pkg-config mpv

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install miru
cargo install --git https://github.com/YannickHerrero/miru
```

#### Linux (Fedora)

```bash
# Install dependencies
sudo dnf install gcc pkg-config mpv

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install miru
cargo install --git https://github.com/YannickHerrero/miru
```

#### Linux (Arch)

```bash
# Install dependencies
sudo pacman -S base-devel mpv

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install miru
cargo install --git https://github.com/YannickHerrero/miru
```

#### macOS

```bash
# Install Xcode Command Line Tools (includes C compiler)
xcode-select --install

# Install mpv via Homebrew
brew install mpv

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install miru
cargo install --git https://github.com/YannickHerrero/miru
```

#### Windows

1. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "C++ build tools" workload
2. Install [Rust](https://rustup.rs/)
3. Install mpv: `winget install mpv` (or download from https://mpv.io/installation/)
4. Install miru:
   ```powershell
   cargo install --git https://github.com/YannickHerrero/miru
   ```

#### Windows (WSL)

Miru runs in WSL but can use Windows-native mpv for video playback, avoiding the need for X11/WSLg setup.

```bash
# In WSL: Install build dependencies
sudo apt update
sudo apt install build-essential pkg-config

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install miru
cargo install --git https://github.com/YannickHerrero/miru
```

**Configure Windows mpv**: Install mpv on Windows (`winget install mpv`), then point miru to it:

```bash
miru config --set player_command=/mnt/c/Users/<YourUsername>/AppData/Local/Microsoft/WinGet/Links/mpv.exe
```

Common mpv paths: Scoop (`scoop/shims/mpv.exe`), Chocolatey (`ProgramData/chocolatey/bin/mpv.exe`).

## Quick Start

1. Get your API keys:
    - **TMDB** (required): https://www.themoviedb.org/settings/api
      - Needed to search for movies, TV shows, and anime
      - Use the **API Key (v3 auth)**, not the Read Access Token
    - **Real-Debrid** (optional, paid subscription): https://real-debrid.com/apitoken
      - Provides instant cached playback for popular content
      - Requires a paid Real-Debrid subscription
      - Without it, miru uses direct P2P streaming (may require buffering while downloading)

2. Run the setup wizard:
   ```bash
   miru init
   ```
    During setup:
    - TMDB key is **required** to enable search functionality
    - Real-Debrid is **optional** - choose between:
      - **Direct P2P Streaming** (free): Download torrents directly to your device
      - **Real-Debrid Cached** (requires paid subscription): Access cached torrents on Real-Debrid servers

3. Start watching:
   ```bash
   miru
   ```

### Add Real-Debrid Later

If you chose direct P2P streaming, you can add Real-Debrid anytime:

```bash
miru config --set rd_api_key YOUR_API_KEY
```

## Usage

```bash
# Interactive mode (default) - full TUI experience
miru

# Quick search - skip straight to results
miru search "frieren"
miru search "inception"
miru s "breaking bad"

# Manage configuration
miru config --show
miru config --set rd_api_key <KEY>
miru config --set tmdb_api_key <KEY>
miru config --reset
```

## Keyboard Navigation

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `Enter` | Select |
| `Esc` / `q` | Back / Quit |
| `/` | Focus search |
| `Ctrl+T` | Cycle theme (auto/dark/light) |

## Search Results

Results are displayed with type indicators:

- `[Movie]` - Movies (pink)
- `[TV]` - TV shows and anime (green)

## Configuration

Configuration is stored at `~/.config/miru/config.toml`. Here's a full example with all available options:

```toml
[real_debrid]
api_key = "your_real_debrid_api_key"  # Optional - leave empty for P2P streaming

[tmdb]
api_key = "your_tmdb_api_key"  # Required

[torrentio]
providers = ["yts", "eztv", "rarbg", "1337x", "thepiratebay", "kickasstorrents", "torrentgalaxy", "nyaasi"]
quality = "best"  # "best", "1080p", "720p", "480p"
sort = "quality"  # "quality", "size", "seeders"

[player]
command = "mpv"
args = ["--fullscreen"]

[streaming]
http_port = 3131              # Port for P2P streaming server
cleanup_after_playback = true # Delete downloaded files after playback

[ui]
theme = "auto"  # "auto", "dark", "light"

# Optional: custom color overrides (hex format)
[ui.colors]
# primary = "#89b4fa"    # Highlights, selected items
# secondary = "#f5c2e7"  # Titles, movie badges
# success = "#a6e3a1"    # TV badges, checkmarks
# warning = "#f9e2af"    # HDR labels, ratings
# error = "#f38ba8"      # Error messages
# muted = "#6c7086"      # Secondary text, borders
# text = "#cdd6f4"       # Normal text
```

### Theme Configuration

Miru supports three theme modes. Press `Ctrl+T` at any time to cycle through them:

| Theme | Description |
|-------|-------------|
| `auto` | (Default) Uses terminal's default ANSI colors - automatically adapts to your terminal's light/dark theme |
| `dark` | Catppuccin Mocha with specific RGB colors optimized for dark backgrounds |
| `light` | Catppuccin Latte with specific RGB colors optimized for light backgrounds |

Theme changes are saved automatically to your config file.

You can also override individual colors using the `[ui.colors]` section with hex color codes (`#RRGGBB`).

### Streaming Modes

**With Real-Debrid (recommended):**
- Instant playback from Real-Debrid's cache
- No local downloading required
- Works best for popular content

**Without Real-Debrid (P2P):**
- Direct torrent streaming via librqbit
- May require buffering before playback starts
- Downloaded to temp directory, cleaned up after playback

## License

MIT
