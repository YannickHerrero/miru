# miru (見る)

A sleek, terminal-native anime streaming CLI that connects Anilist, Torrentio, Real-Debrid, and MPV into a seamless viewing experience.

## Features

- **Fast**: Sub-second startup, minimal keystrokes from launch to playback
- **Beautiful**: Rich terminal UI with smooth animations and Catppuccin-inspired colors
- **Simple**: Zero configuration beyond the Real-Debrid API key
- **Reliable**: Graceful error handling with clear feedback

## Installation

```bash
cargo install --path .
```

## Quick Start

1. Get a Real-Debrid API key from https://real-debrid.com/apitoken
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
miru s "frieren"

# Manage configuration
miru config --show
miru config --set rd_api_key <KEY>
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

## Configuration

Configuration is stored at `~/.config/miru/config.toml`:

```toml
[real_debrid]
api_key = "your_api_key_here"

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

## How It Works

```
User Input → Anilist API → ID Mapping → Torrentio → Real-Debrid → MPV
```

1. Search anime using Anilist's database
2. Map anime IDs to IMDB IDs via arm-server
3. Fetch available torrents from Torrentio
4. Resolve stream URLs through Real-Debrid
5. Launch MPV with the direct stream

## License

MIT
