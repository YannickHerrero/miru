# miru (見る)

A sleek, terminal-native streaming CLI for movies and TV shows. Connects TMDB, Torrentio, Real-Debrid, and MPV into a seamless viewing experience.

## Demo

https://github.com/user-attachments/assets/0fc5c941-bcf2-4b0a-bdbf-791b87337e22

## Features

- **TMDB search**: Search movies, TV shows, and anime via TMDB
- **Fast**: Sub-second startup, minimal keystrokes from launch to playback
- **Beautiful**: Rich terminal UI with smooth animations and Catppuccin-inspired colors
- **Smart flow**: Automatically skips episode selection for movies, shows season selection for TV shows
- **Reliable**: Graceful error handling with clear feedback

## Installation

```bash
cargo install --path .
```

## Quick Start

1. Get your API keys:
   - **Real-Debrid** (required): https://real-debrid.com/apitoken
   - **TMDB** (required): https://www.themoviedb.org/settings/api
     - Use the **API Key (v3 auth)**, not the Read Access Token

2. Run the setup wizard:
   ```bash
   miru init
   ```

3. Start watching:
   ```bash
   miru
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

## Search Results

Results are displayed with type indicators:

- `[Movie]` - Movies (pink)
- `[TV]` - TV shows and anime (green)

## Configuration

Configuration is stored at `~/.config/miru/config.toml`:

```toml
[real_debrid]
api_key = "your_real_debrid_api_key"

[tmdb]
api_key = "your_tmdb_api_key"

[torrentio]
providers = ["yts", "eztv", "rarbg", "1337x", "thepiratebay"]
quality = "best"
sort = "quality"

[player]
command = "mpv"
args = ["--fullscreen"]

[ui]
theme = "default"
```

## Requirements

- [MPV](https://mpv.io/) media player (or another compatible player)
- [Real-Debrid](https://real-debrid.com/) subscription and API key
- [TMDB API key](https://www.themoviedb.org/settings/api)

## License

MIT
